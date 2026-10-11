//! Conteneur ZIP (avec extensions ZIP64, permissions Unix, liens symboliques
//! et horodatages précis) écrit et lu à la main, sans dépendance.
//!
//! Le format de sauvegarde MozInRustBackup est un ZIP standard : il reste
//! ouvrable par `unzip`, 7-Zip, Ark, Engrampa ou le gestionnaire d'archives de
//! GNOME/KDE — un filet de sécurité pour l'utilisateur, sans jamais dépendre
//! d'un logiciel tiers pour la restauration.

use std::io::{self, Read, Seek, SeekFrom, Write};

use super::crc32::Crc32;
use super::deflate::{Deflater, Level};
use super::inflate::{inflate, InflateError};
use super::sha256::Sha256;

const SIG_LOCAL: u32 = 0x0403_4B50;
const SIG_CENTRAL: u32 = 0x0201_4B50;
const SIG_EOCD: u32 = 0x0605_4B50;
const SIG_ZIP64_EOCD: u32 = 0x0606_4B50;
const SIG_ZIP64_LOCATOR: u32 = 0x0706_4B50;

const FLAG_UTF8: u16 = 0x0800;
/// Les archives lues peuvent contenir un descripteur de données (bit 3) ; il
/// est accepté à la lecture, jamais utilisé à l'écriture (les tailles sont
/// corrigées dans l'en-tête local une fois les données écrites).
#[allow(dead_code)]
const FLAG_DATA_DESCRIPTOR: u16 = 0x0008;

const METHOD_STORE: u16 = 0;
const METHOD_DEFLATE: u16 = 8;

/// Seuil à partir duquel une entrée réserve un champ ZIP64 dans son en-tête
/// local (la taille compressée réelle n'est connue qu'après écriture).
const ZIP64_RESERVE_THRESHOLD: u64 = 0xFFFF_FFFF - 4 * 1024 * 1024;

const UNIX_MADE_BY: u16 = (3 << 8) | 63; // version 6.3, système Unix
const VERSION_ZIP64: u16 = 45;
const VERSION_DEFAULT: u16 = 20;

// --- horodatage DOS -------------------------------------------------------

fn dos_datetime(mtime: i64, tz_offset_secs: i32) -> (u16, u16) {
    // ZIP utilise l'heure locale ; l'horodatage exact (UTC, secondes) est
    // conservé dans le champ supplémentaire 0x5455.
    let secs = mtime.saturating_add(tz_offset_secs as i64);
    let (y, mo, d, h, mi, s) = crate::util::time::civil_from_unix(secs);
    let year = if y < 1980 {
        1980
    } else if y > 2107 {
        2107
    } else {
        y
    };
    let date = (((year - 1980) as u16) << 9) | ((mo as u16) << 5) | (d as u16);
    let time = ((h as u16) << 11) | ((mi as u16) << 5) | ((s as u16) / 2);
    (date, time)
}

// --- champs supplémentaires ----------------------------------------------

fn extra_timestamp(mtime: i64, atime: Option<i64>) -> Vec<u8> {
    let mut flags = 0x01u8;
    let mut payload = Vec::new();
    payload.extend_from_slice(&(mtime as u32).to_le_bytes());
    if let Some(a) = atime {
        flags |= 0x02;
        payload.extend_from_slice(&(a as u32).to_le_bytes());
    }
    let mut out = Vec::with_capacity(payload.len() + 5);
    out.extend_from_slice(&0x5455u16.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u16 + 1).to_le_bytes());
    out.push(flags);
    out.extend_from_slice(&payload);
    out
}

fn extra_unix(uid: u32, gid: u32) -> Vec<u8> {
    // 0x7875 : « Info-ZIP New Unix Extra Field ».
    let mut out = Vec::with_capacity(15);
    out.extend_from_slice(&0x7875u16.to_le_bytes());
    out.extend_from_slice(&11u16.to_le_bytes());
    out.push(1);
    out.push(4);
    out.extend_from_slice(&uid.to_le_bytes());
    out.push(4);
    out.extend_from_slice(&gid.to_le_bytes());
    out
}

fn extra_zip64(usize_: u64, csize: u64, offset: Option<u64>) -> Vec<u8> {
    let mut payload = Vec::with_capacity(24);
    payload.extend_from_slice(&usize_.to_le_bytes());
    payload.extend_from_slice(&csize.to_le_bytes());
    if let Some(o) = offset {
        payload.extend_from_slice(&o.to_le_bytes());
    }
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&0x0001u16.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

// --- écriture -------------------------------------------------------------

/// Métadonnées d'une entrée à écrire.
#[derive(Debug, Clone)]
pub struct WriteEntry<'a> {
    pub name: &'a str,
    /// Mode Unix complet (`st_mode`), par exemple `0o100644`.
    pub mode: u32,
    pub mtime: i64,
    pub atime: Option<i64>,
    pub uid: u32,
    pub gid: u32,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl<'a> WriteEntry<'a> {
    pub fn file(name: &'a str, mode: u32, mtime: i64, uid: u32, gid: u32) -> Self {
        WriteEntry { name, mode, mtime, atime: None, uid, gid, is_dir: false, is_symlink: false }
    }
    pub fn dir(name: &'a str, mode: u32, mtime: i64, uid: u32, gid: u32) -> Self {
        WriteEntry { name, mode, mtime, atime: None, uid, gid, is_dir: true, is_symlink: false }
    }
    pub fn symlink(name: &'a str, mtime: i64, uid: u32, gid: u32) -> Self {
        WriteEntry {
            name,
            mode: 0o120777,
            mtime,
            atime: None,
            uid,
            gid,
            is_dir: false,
            is_symlink: true,
        }
    }
}

/// Statistiques renvoyées après l'ajout d'une entrée.
#[derive(Debug, Clone)]
pub struct EntryStats {
    pub uncompressed: u64,
    pub compressed: u64,
    pub crc32: u32,
    pub sha256: String,
}

struct CentralRecord {
    name: String,
    method: u16,
    crc: u32,
    csize: u64,
    usize_: u64,
    offset: u64,
    meta: WriteEntryOwned,
}

#[derive(Debug, Clone)]
struct WriteEntryOwned {
    mode: u32,
    mtime: i64,
    atime: Option<i64>,
    uid: u32,
    gid: u32,
    is_dir: bool,
}

pub struct ZipWriter<W: Write + Seek> {
    w: W,
    records: Vec<CentralRecord>,
    tz_offset: i32,
    level: Level,
    finished: bool,
}

impl<W: Write + Seek> ZipWriter<W> {
    pub fn new(w: W, tz_offset_secs: i32, level: Level) -> Self {
        ZipWriter { w, records: Vec::new(), tz_offset: tz_offset_secs, level, finished: false }
    }

    pub fn entry_count(&self) -> usize {
        self.records.len()
    }

    /// Position courante (taille de l'archive en cours d'écriture).
    pub fn position(&mut self) -> io::Result<u64> {
        self.w.stream_position()
    }

    /// Ajoute un dossier (entrée vide terminée par « / »).
    pub fn add_dir(&mut self, meta: &WriteEntry<'_>) -> io::Result<()> {
        let name = if meta.name.ends_with('/') { meta.name.to_string() } else { format!("{}/", meta.name) };
        let empty: &[u8] = &[];
        self.add_entry_inner(meta, &name, &mut { empty }, 0, false, &mut |_| {})?;
        Ok(())
    }

    /// Ajoute des données en mémoire (manifeste, cible d'un lien symbolique...).
    pub fn add_bytes(
        &mut self,
        meta: &WriteEntry<'_>,
        data: &[u8],
        compress: bool,
    ) -> io::Result<EntryStats> {
        let name = meta.name.to_string();
        let mut src = data;
        self.add_entry_inner(meta, &name, &mut src, data.len() as u64, compress, &mut |_| {})
    }

    /// Ajoute un fichier depuis un lecteur, en calculant CRC-32 et SHA-256.
    ///
    /// `size` doit être la taille exacte attendue : toute différence en fin de
    /// lecture déclenche une erreur (le profil a changé pendant la sauvegarde).
    pub fn add_file<R: Read>(
        &mut self,
        meta: &WriteEntry<'_>,
        reader: &mut R,
        size: u64,
        compress: bool,
        progress: &mut dyn FnMut(u64),
    ) -> io::Result<EntryStats> {
        let name = meta.name.to_string();
        self.add_entry_inner(meta, &name, reader, size, compress, progress)
    }

    fn add_entry_inner<R: Read>(
        &mut self,
        meta: &WriteEntry<'_>,
        name: &str,
        reader: &mut R,
        size: u64,
        compress: bool,
        progress: &mut dyn FnMut(u64),
    ) -> io::Result<EntryStats> {
        if self.finished {
            return Err(io::Error::new(io::ErrorKind::Other, "archive déjà finalisée"));
        }
        let name_bytes = name.as_bytes();
        if name_bytes.len() > 0xFFFF {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "nom d'entrée trop long"));
        }
        let use_zip64 = size >= ZIP64_RESERVE_THRESHOLD;
        let method = if meta.is_dir { METHOD_STORE } else if compress { METHOD_DEFLATE } else { METHOD_STORE };

        let header_start = self.w.stream_position()?;
        let (dos_date, dos_time) = dos_datetime(meta.mtime, self.tz_offset);

        let mut extra = Vec::new();
        let mut zip64_extra_payload_offset = None;
        if use_zip64 {
            // En-tête local : taille réelle + taille compressée (valeurs
            // provisoires, corrigées après écriture des données).
            extra.extend_from_slice(&extra_zip64(0, 0, None));
            zip64_extra_payload_offset = Some(header_start + 30 + name_bytes.len() as u64 + 4);
        }
        extra.extend_from_slice(&extra_timestamp(meta.mtime, None));
        extra.extend_from_slice(&extra_unix(meta.uid, meta.gid));

        let mut hdr = Vec::with_capacity(30 + name_bytes.len() + extra.len());
        hdr.extend_from_slice(&SIG_LOCAL.to_le_bytes());
        hdr.extend_from_slice(&(if use_zip64 { VERSION_ZIP64 } else { VERSION_DEFAULT }).to_le_bytes());
        hdr.extend_from_slice(&FLAG_UTF8.to_le_bytes());
        hdr.extend_from_slice(&method.to_le_bytes());
        hdr.extend_from_slice(&dos_time.to_le_bytes());
        hdr.extend_from_slice(&dos_date.to_le_bytes());
        hdr.extend_from_slice(&0u32.to_le_bytes()); // CRC (corrigé ensuite)
        let size_field: u32 = if use_zip64 { 0xFFFF_FFFF } else { 0 };
        hdr.extend_from_slice(&size_field.to_le_bytes()); // taille compressée
        hdr.extend_from_slice(&size_field.to_le_bytes()); // taille réelle
        hdr.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        hdr.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        hdr.extend_from_slice(name_bytes);
        hdr.extend_from_slice(&extra);
        self.w.write_all(&hdr)?;

        // --- données de l'entrée -----------------------------------------
        let mut crc = Crc32::new();
        let mut sha = Sha256::new();
        let mut usize_: u64 = 0;
        let mut buf = vec![0u8; 256 * 1024];
        let mut deflater = if method == METHOD_DEFLATE { Some(Deflater::new(self.level)) } else { None };
        let csize;
        {
            // Toutes les écritures de l'entrée passent par ce compteur : en mode
            // « stored » les données partent telles quelles, en mode « deflate »
            // elles sont compressées à la volée.
            let mut counter = CountWriter { inner: &mut self.w, written: 0 };
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                let chunk = &buf[..n];
                crc.update(chunk);
                sha.update(chunk);
                usize_ += n as u64;
                match deflater.as_mut() {
                    Some(d) => d.write(chunk, &mut counter)?,
                    None => counter.write_all(chunk)?,
                }
                progress(n as u64);
            }
            if let Some(d) = deflater.as_mut() {
                d.finish(&mut counter)?;
            }
            csize = counter.written;
        }
        let data_end = self.w.stream_position()?;
        self.w.flush()?;

        if usize_ != size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "« {} » a changé pendant la sauvegarde ({} octets annoncés, {} lus)",
                    name, size, usize_
                ),
            ));
        }

        // --- correction de l'en-tête local -------------------------------
        self.w.seek(SeekFrom::Start(header_start + 14))?;
        self.w.write_all(&crc.value().to_le_bytes())?;
        if use_zip64 {
            self.w.write_all(&0xFFFF_FFFFu32.to_le_bytes())?;
            self.w.write_all(&0xFFFF_FFFFu32.to_le_bytes())?;
            if let Some(off) = zip64_extra_payload_offset {
                self.w.seek(SeekFrom::Start(off))?;
                self.w.write_all(&usize_.to_le_bytes())?;
                self.w.write_all(&csize.to_le_bytes())?;
            }
        } else {
            self.w.write_all(&(csize as u32).to_le_bytes())?;
            self.w.write_all(&(usize_ as u32).to_le_bytes())?;
        }
        self.w.seek(SeekFrom::Start(data_end))?;

        let stats = EntryStats {
            uncompressed: usize_,
            compressed: csize,
            crc32: crc.value(),
            sha256: sha.finish_hex(),
        };
        self.records.push(CentralRecord {
            name: name.to_string(),
            method,
            crc: stats.crc32,
            csize,
            usize_,
            offset: header_start,
            meta: WriteEntryOwned {
                mode: meta.mode,
                mtime: meta.mtime,
                atime: meta.atime,
                uid: meta.uid,
                gid: meta.gid,
                is_dir: meta.is_dir,
            },
        });
        Ok(stats)
    }

    /// Écrit le répertoire central puis les enregistrements de fin.
    pub fn finish(mut self) -> io::Result<W> {
        if self.finished {
            return Ok(self.w);
        }
        self.finished = true;
        let cd_start = self.w.stream_position()?;
        let mut zip64_central = false;
        let mut central = Vec::new();
        for r in &self.records {
            let need_zip64 = r.usize_ >= 0xFFFF_FFFF || r.csize >= 0xFFFF_FFFF || r.offset >= 0xFFFF_FFFF;
            let mut extra = Vec::new();
            let (csize_field, usize_field, offset_field) = if need_zip64 {
                zip64_central = true;
                let mut payload = Vec::new();
                let usize_f = if r.usize_ >= 0xFFFF_FFFF {
                    payload.extend_from_slice(&r.usize_.to_le_bytes());
                    0xFFFF_FFFFu32
                } else {
                    r.usize_ as u32
                };
                let csize_f = if r.csize >= 0xFFFF_FFFF {
                    payload.extend_from_slice(&r.csize.to_le_bytes());
                    0xFFFF_FFFFu32
                } else {
                    r.csize as u32
                };
                let offset_f = if r.offset >= 0xFFFF_FFFF {
                    payload.extend_from_slice(&r.offset.to_le_bytes());
                    0xFFFF_FFFFu32
                } else {
                    r.offset as u32
                };
                extra.extend_from_slice(&0x0001u16.to_le_bytes());
                extra.extend_from_slice(&(payload.len() as u16).to_le_bytes());
                extra.extend_from_slice(&payload);
                (csize_f, usize_f, offset_f)
            } else {
                (r.csize as u32, r.usize_ as u32, r.offset as u32)
            };
            extra.extend_from_slice(&extra_timestamp(r.meta.mtime, r.meta.atime));
            extra.extend_from_slice(&extra_unix(r.meta.uid, r.meta.gid));
            let name_bytes = r.name.as_bytes();
            let (dos_date, dos_time) = dos_datetime(r.meta.mtime, self.tz_offset);
            let external = ((r.meta.mode & 0xFFFF) << 16) | if r.meta.is_dir { 0x10u32 } else { 0 };

            central.extend_from_slice(&SIG_CENTRAL.to_le_bytes());
            central.extend_from_slice(&UNIX_MADE_BY.to_le_bytes());
            central.extend_from_slice(
                &(if need_zip64 { VERSION_ZIP64 } else { VERSION_DEFAULT }).to_le_bytes(),
            );
            central.extend_from_slice(&FLAG_UTF8.to_le_bytes());
            central.extend_from_slice(&r.method.to_le_bytes());
            central.extend_from_slice(&dos_time.to_le_bytes());
            central.extend_from_slice(&dos_date.to_le_bytes());
            central.extend_from_slice(&r.crc.to_le_bytes());
            central.extend_from_slice(&csize_field.to_le_bytes());
            central.extend_from_slice(&usize_field.to_le_bytes());
            central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            central.extend_from_slice(&(extra.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // commentaire
            central.extend_from_slice(&0u16.to_le_bytes()); // disque de départ
            central.extend_from_slice(&0u16.to_le_bytes()); // attributs internes
            central.extend_from_slice(&external.to_le_bytes());
            central.extend_from_slice(&offset_field.to_le_bytes());
            central.extend_from_slice(name_bytes);
            central.extend_from_slice(&extra);
        }
        let cd_size = central.len() as u64;
        self.w.write_all(&central)?;
        let cd_end = self.w.stream_position()?;
        let count = self.records.len() as u64;

        let needs_zip64_eocd =
            zip64_central || count > 0xFFFF || cd_start >= 0xFFFF_FFFF || cd_size >= 0xFFFF_FFFF;
        if needs_zip64_eocd {
            let mut z = Vec::with_capacity(56);
            z.extend_from_slice(&SIG_ZIP64_EOCD.to_le_bytes());
            z.extend_from_slice(&44u64.to_le_bytes()); // taille de l'enregistrement
            z.extend_from_slice(&UNIX_MADE_BY.to_le_bytes());
            z.extend_from_slice(&VERSION_ZIP64.to_le_bytes());
            z.extend_from_slice(&0u32.to_le_bytes()); // numéro de disque
            z.extend_from_slice(&0u32.to_le_bytes()); // disque du répertoire central
            z.extend_from_slice(&count.to_le_bytes());
            z.extend_from_slice(&count.to_le_bytes());
            z.extend_from_slice(&cd_size.to_le_bytes());
            z.extend_from_slice(&cd_start.to_le_bytes());
            self.w.write_all(&z)?;
            let mut loc = Vec::with_capacity(20);
            loc.extend_from_slice(&SIG_ZIP64_LOCATOR.to_le_bytes());
            loc.extend_from_slice(&0u32.to_le_bytes()); // disque du zip64 EOCD
            loc.extend_from_slice(&cd_end.to_le_bytes());
            loc.extend_from_slice(&1u32.to_le_bytes()); // nombre total de disques
            self.w.write_all(&loc)?;
        }

        let count16 = if count > 0xFFFF { 0xFFFFu16 } else { count as u16 };
        let cd_size32 = if cd_size >= 0xFFFF_FFFF { 0xFFFF_FFFFu32 } else { cd_size as u32 };
        let cd_start32 = if cd_start >= 0xFFFF_FFFF { 0xFFFF_FFFFu32 } else { cd_start as u32 };
        let mut eocd = Vec::with_capacity(22);
        eocd.extend_from_slice(&SIG_EOCD.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes()); // numéro de disque
        eocd.extend_from_slice(&0u16.to_le_bytes()); // disque du répertoire central
        eocd.extend_from_slice(&count16.to_le_bytes());
        eocd.extend_from_slice(&count16.to_le_bytes());
        eocd.extend_from_slice(&cd_size32.to_le_bytes());
        eocd.extend_from_slice(&cd_start32.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes()); // commentaire
        self.w.write_all(&eocd)?;
        self.w.flush()?;
        Ok(self.w)
    }
}

struct CountWriter<'a, W: Write> {
    inner: &'a mut W,
    written: u64,
}

impl<'a, W: Write> Write for CountWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

// --- lecture --------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub method: u16,
    pub crc: u32,
    pub csize: u64,
    pub usize_: u64,
    pub header_offset: u64,
    pub mode: u32,
    pub mtime: i64,
    pub atime: Option<i64>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub is_dir: bool,
    pub is_symlink: bool,
}

pub struct ZipReader<R: Read + Seek> {
    r: R,
    entries: Vec<ZipEntry>,
}

fn ler_u16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn ler_u32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn ler_u64(b: &[u8]) -> u64 {
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Décode les champs supplémentaires utiles d'une entrée.
struct ParsedExtra {
    zip64_usize: Option<u64>,
    zip64_csize: Option<u64>,
    zip64_offset: Option<u64>,
    mtime: Option<i64>,
    atime: Option<i64>,
    uid: Option<u32>,
    gid: Option<u32>,
}

fn parse_extra(extra: &[u8], usize_placeholder: bool, csize_placeholder: bool, offset_placeholder: bool) -> ParsedExtra {
    let mut out = ParsedExtra {
        zip64_usize: None,
        zip64_csize: None,
        zip64_offset: None,
        mtime: None,
        atime: None,
        uid: None,
        gid: None,
    };
    let mut i = 0usize;
    while i + 4 <= extra.len() {
        let id = ler_u16(&extra[i..]);
        let size = ler_u16(&extra[i + 2..]) as usize;
        let start = i + 4;
        if start + size > extra.len() {
            break;
        }
        let data = &extra[start..start + size];
        match id {
            0x0001 => {
                let mut p = 0usize;
                if usize_placeholder && p + 8 <= data.len() {
                    out.zip64_usize = Some(ler_u64(&data[p..]));
                    p += 8;
                }
                if csize_placeholder && p + 8 <= data.len() {
                    out.zip64_csize = Some(ler_u64(&data[p..]));
                    p += 8;
                }
                if offset_placeholder && p + 8 <= data.len() {
                    out.zip64_offset = Some(ler_u64(&data[p..]));
                }
            }
            0x5455 => {
                if !data.is_empty() {
                    let flags = data[0];
                    let mut p = 1usize;
                    if flags & 0x01 != 0 && p + 4 <= data.len() {
                        out.mtime = Some(ler_u32(&data[p..]) as i64);
                        p += 4;
                    }
                    if flags & 0x02 != 0 && p + 4 <= data.len() {
                        out.atime = Some(ler_u32(&data[p..]) as i64);
                    }
                }
            }
            0x7875 => {
                // version(1) uid_size(1) uid gid_size(1) gid
                let mut p = 0usize;
                if data.len() >= 3 {
                    p += 1; // version
                    let uid_size = data[p] as usize;
                    p += 1;
                    if p + uid_size <= data.len() {
                        let mut v = 0u32;
                        for k in 0..uid_size.min(4) {
                            v |= (data[p + k] as u32) << (8 * k);
                        }
                        out.uid = Some(v);
                        p += uid_size;
                    }
                    if p < data.len() {
                        let gid_size = data[p] as usize;
                        p += 1;
                        if p + gid_size <= data.len() {
                            let mut v = 0u32;
                            for k in 0..gid_size.min(4) {
                                v |= (data[p + k] as u32) << (8 * k);
                            }
                            out.gid = Some(v);
                        }
                    }
                }
            }
            _ => {}
        }
        i = start + size;
    }
    out
}

impl<R: Read + Seek> ZipReader<R> {
    pub fn open(mut r: R) -> io::Result<Self> {
        let file_len = r.seek(SeekFrom::End(0))?;
        if file_len < 22 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "fichier trop petit pour être une archive ZIP"));
        }
        let scan_len = file_len.min(22 + 65_535 + 64);
        r.seek(SeekFrom::Start(file_len - scan_len))?;
        let mut tail = vec![0u8; scan_len as usize];
        r.read_exact(&mut tail)?;
        let mut eocd_pos = None;
        let mut idx = tail.len() as i64 - 22;
        while idx >= 0 {
            let p = idx as usize;
            if p + 4 <= tail.len() && ler_u32(&tail[p..]) == SIG_EOCD {
                let comment_len = ler_u16(&tail[p + 20..]) as usize;
                if p + 22 + comment_len <= tail.len() {
                    eocd_pos = Some(p as u64 + (file_len - scan_len));
                    break;
                }
            }
            idx -= 1;
        }
        let eocd_pos = eocd_pos.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "fin de répertoire central introuvable")
        })?;
        let eocd = {
            let p = (eocd_pos - (file_len - scan_len)) as usize;
            &tail[p..p + 22]
        };
        let mut count = ler_u16(&eocd[10..]) as u64;
        let mut cd_size = ler_u32(&eocd[12..]) as u64;
        let mut cd_offset = ler_u32(&eocd[16..]) as u64;

        // ZIP64 : localisateur juste avant le EOCD classique.
        if eocd_pos >= 20 {
            r.seek(SeekFrom::Start(eocd_pos - 20))?;
            let mut loc = [0u8; 20];
            r.read_exact(&mut loc)?;
            if ler_u32(&loc) == SIG_ZIP64_LOCATOR {
                let z64_offset = ler_u64(&loc[8..]);
                r.seek(SeekFrom::Start(z64_offset))?;
                let mut z = [0u8; 56];
                r.read_exact(&mut z)?;
                if ler_u32(&z) == SIG_ZIP64_EOCD {
                    count = ler_u64(&z[32..]);
                    cd_size = ler_u64(&z[40..]);
                    cd_offset = ler_u64(&z[48..]);
                }
            }
        }

        r.seek(SeekFrom::Start(cd_offset))?;
        let mut cd = vec![0u8; cd_size as usize];
        r.read_exact(&mut cd)?;
        let mut entries = Vec::with_capacity(count as usize);
        let mut i = 0usize;
        while i + 46 <= cd.len() && (entries.len() as u64) < count {
            if ler_u32(&cd[i..]) != SIG_CENTRAL {
                break;
            }
            let method = ler_u16(&cd[i + 10..]);
            let crc = ler_u32(&cd[i + 16..]);
            let csize32 = ler_u32(&cd[i + 20..]);
            let usize32 = ler_u32(&cd[i + 24..]);
            let name_len = ler_u16(&cd[i + 28..]) as usize;
            let extra_len = ler_u16(&cd[i + 30..]) as usize;
            let comment_len = ler_u16(&cd[i + 32..]) as usize;
            let external = ler_u32(&cd[i + 38..]);
            let offset32 = ler_u32(&cd[i + 42..]);
            let name_start = i + 46;
            if name_start + name_len + extra_len + comment_len > cd.len() {
                break;
            }
            let name = String::from_utf8_lossy(&cd[name_start..name_start + name_len]).into_owned();
            let extra = &cd[name_start + name_len..name_start + name_len + extra_len];
            let pe = parse_extra(
                extra,
                usize32 == 0xFFFF_FFFF,
                csize32 == 0xFFFF_FFFF,
                offset32 == 0xFFFF_FFFF,
            );
            let usize_ = pe.zip64_usize.unwrap_or(usize32 as u64);
            let csize = pe.zip64_csize.unwrap_or(csize32 as u64);
            let offset = pe.zip64_offset.unwrap_or(offset32 as u64);
            let mode = (external >> 16) & 0xFFFF;
            let is_dir = name.ends_with('/') || (external & 0x10 != 0);
            let is_symlink = (mode & 0xF000) == 0xA000;
            entries.push(ZipEntry {
                name,
                method,
                crc,
                csize,
                usize_,
                header_offset: offset,
                mode: if mode == 0 {
                    if is_dir {
                        0o040755
                    } else {
                        0o100644
                    }
                } else {
                    mode
                },
                mtime: pe.mtime.unwrap_or(0),
                atime: pe.atime,
                uid: pe.uid,
                gid: pe.gid,
                is_dir,
                is_symlink,
            });
            i = name_start + name_len + extra_len + comment_len;
        }

        Ok(ZipReader { r, entries })
    }

    pub fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    pub fn has_entry(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    /// Décompresse l'entrée `index` vers `out` et vérifie CRC et taille.
    pub fn read_entry<W: Write>(&mut self, index: usize, out: &mut W) -> io::Result<u64> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "entrée inexistante"))?
            .clone();

        // En-tête local : on ne se fie qu'au répertoire central (les tailles y
        // sont toujours exactes, même si l'archive utilise des descripteurs).
        self.r.seek(SeekFrom::Start(entry.header_offset))?;
        let mut lh = [0u8; 30];
        self.r.read_exact(&mut lh)?;
        if ler_u32(&lh) != SIG_LOCAL {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "en-tête local invalide"));
        }
        let flags = ler_u16(&lh[6..]);
        let name_len = ler_u16(&lh[26..]) as u64;
        let extra_len = ler_u16(&lh[28..]) as u64;
        let data_start = entry.header_offset + 30 + name_len + extra_len;
        self.r.seek(SeekFrom::Start(data_start))?;

        if entry.is_dir {
            return Ok(0);
        }

        let mut limited = (&mut self.r).take(entry.csize);
        match entry.method {
            METHOD_STORE => {
                let mut crc = Crc32::new();
                let mut buf = vec![0u8; 128 * 1024];
                let mut total = 0u64;
                loop {
                    let n = limited.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    crc.update(&buf[..n]);
                    out.write_all(&buf[..n])?;
                    total += n as u64;
                }
                if total != entry.usize_ {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("taille incorrecte pour « {} »", entry.name),
                    ));
                }
                if crc.value() != entry.crc && entry.crc != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("CRC-32 invalide pour « {} »", entry.name),
                    ));
                }
                let _ = flags;
                Ok(total)
            }
            METHOD_DEFLATE => {
                let res = inflate(limited, out, Some(entry.usize_)).map_err(|e| match e {
                    InflateError::Io(io_err) => io_err,
                    other => io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("« {} » : {}", entry.name, other),
                    ),
                })?;
                if res.crc32 != entry.crc && entry.crc != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("CRC-32 invalide pour « {} »", entry.name),
                    ));
                }
                Ok(res.bytes)
            }
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "méthode de compression non gérée ({}) pour « {} » : recompressez l'archive au format ZIP standard",
                    other, entry.name
                ),
            )),
        }
    }

    /// Lit une entrée en mémoire (manifeste, fichiers courts).
    pub fn read_entry_to_vec(&mut self, index: usize) -> io::Result<Vec<u8>> {
        let cap = self.entries[index].usize_ as usize;
        let mut v = Vec::with_capacity(cap.min(8 * 1024 * 1024));
        self.read_entry(index, &mut v)?;
        Ok(v)
    }

    /// Lit une entrée nommée en mémoire.
    pub fn read_named_to_vec(&mut self, name: &str) -> io::Result<Option<Vec<u8>>> {
        match self.entries.iter().position(|e| e.name == name) {
            Some(i) => Ok(Some(self.read_entry_to_vec(i)?)),
            None => Ok(None),
        }
    }

    /// Lit une entrée vers un écrivain « poubelle », en renvoyant (taille, sha256, crc).
    pub fn hash_entry(&mut self, index: usize) -> io::Result<(u64, String, u32)> {
        let mut h = HashingSink::new();
        self.read_entry(index, &mut h)?;
        Ok((h.total, h.sha.finish_hex(), h.crc.value()))
    }
}

struct HashingSink {
    total: u64,
    sha: Sha256,
    crc: Crc32,
}

impl HashingSink {
    fn new() -> Self {
        HashingSink { total: 0, sha: Sha256::new(), crc: Crc32::new() }
    }
}

impl Write for HashingSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.total += buf.len() as u64;
        self.sha.update(buf);
        self.crc.update(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn roundtrip(compress: bool, level: Level) {
        let payload: Vec<u8> = b"MozInRustBackup -- contenu de test -- ".iter().cloned().cycle().take(50_000).collect();
        let mut buf = Cursor::new(Vec::new());
        {
            let mut w = ZipWriter::new(&mut buf, 0, level);
            let meta = WriteEntry::dir("dossier", 0o040755, 1_700_000_000, 1000, 1000);
            w.add_dir(&meta).unwrap();
            let meta = WriteEntry::file("dossier/fichier.txt", 0o100644, 1_700_000_100, 1000, 1000);
            let stats = w.add_file(&meta, &mut &payload[..], payload.len() as u64, compress, &mut |_| {}).unwrap();
            assert_eq!(stats.uncompressed, payload.len() as u64);
            assert_eq!(stats.crc32, super::super::crc32::crc32(&payload));
            let link = WriteEntry::symlink("dossier/lien", 1_700_000_200, 1000, 1000);
            w.add_bytes(&link, b"fichier.txt", false).unwrap();
            w.finish().unwrap();
        }
        let data = buf.into_inner();
        let mut rd = ZipReader::open(Cursor::new(data)).unwrap();
        assert_eq!(rd.entries().len(), 3);
        assert!(rd.entries()[0].is_dir);
        assert!(rd.entries()[2].is_symlink);
        assert_eq!(rd.entries()[1].mode, 0o100644);
        assert_eq!(rd.entries()[1].mtime, 1_700_000_100);
        let out = rd.read_entry_to_vec(1).unwrap();
        assert_eq!(out, payload);
        if compress && level != Level::Store {
            assert!(rd.entries()[1].csize < rd.entries()[1].usize_);
        }
    }

    #[test]
    fn roundtrip_store_and_deflate() {
        roundtrip(false, Level::Normal);
        roundtrip(true, Level::Normal);
        roundtrip(true, Level::Max);
        roundtrip(true, Level::Store);
    }

    #[test]
    fn zip64_forced_entry() {
        // Entrée de petite taille mais forçant le chemin ZIP64 par configuration :
        // on vérifie la cohérence des champs 32/64 bits du répertoire central.
        let data = b"x".repeat(1000);
        let mut buf = Cursor::new(Vec::new());
        {
            let mut w = ZipWriter::new(&mut buf, 0, Level::Store);
            let meta = WriteEntry::file("gros.bin", 0o100644, 1_700_000_000, 0, 0);
            w.add_file(&meta, &mut &data[..], data.len() as u64, false, &mut |_| {}).unwrap();
            w.finish().unwrap();
        }
        let mut rd = ZipReader::open(Cursor::new(buf.into_inner())).unwrap();
        assert_eq!(rd.read_entry_to_vec(0).unwrap(), data);
    }
}

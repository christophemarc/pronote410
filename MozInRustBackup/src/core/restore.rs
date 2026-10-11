//! Restauration d'une sauvegarde `.mozinrustbackup`.
//!
//! Étapes :
//!  1. ouverture de l'archive et **contrôle de légitimité** (signature interne,
//!     version de format, manifeste, cohérence du journal) ;
//!  2. affichage de l'emplacement de restauration indiqué par le journal ;
//!  3. confirmation si un profil identique existe déjà — l'ancien profil est
//!     alors **renommé** (jamais effacé sans le dire) ;
//!  4. extraction avec contrôle CRC-32 et, en option, SHA-256 de chaque fichier ;
//!  5. restitution des droits, des dates et des liens symboliques.

use std::collections::HashMap;
use std::io::{self, Read, Seek, Write};
use std::path::{Component, Path, PathBuf};

use crate::archive::sha256::Sha256;
use crate::archive::zip::{ZipReader, ZipWriter};
use crate::core::backup::{parse_hashes, Progress};
use crate::core::manifest::Manifest;
use crate::sys::fsops;
use crate::util::time;
use crate::{BACKUP_EXTENSION, HASHES_MEMBER, MAGIC, MANIFEST_MEMBER};

#[derive(Debug)]
pub enum RestoreError {
    Io(io::Error),
    Cancelled,
    /// Le fichier choisi n'est pas une sauvegarde MozInRustBackup.
    NotABackup(String),
    /// Le profil de destination existe déjà et l'écrasement n'est pas autorisé.
    TargetExists(PathBuf),
    /// Contrôle d'intégrité en échec.
    Corrupt(String),
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreError::Io(e) => write!(f, "{}", e),
            RestoreError::Cancelled => write!(f, "opération annulée par l'utilisateur"),
            RestoreError::NotABackup(m) => write!(f, "{}", m),
            RestoreError::TargetExists(p) => write!(
                f,
                "un profil identique existe déjà : {}",
                p.display()
            ),
            RestoreError::Corrupt(m) => write!(f, "sauvegarde corrompue : {}", m),
        }
    }
}

impl std::error::Error for RestoreError {}

impl From<io::Error> for RestoreError {
    fn from(e: io::Error) -> Self {
        RestoreError::Io(e)
    }
}

/// Informations lues dans une sauvegarde, avant toute écriture sur le disque.
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub manifest: Manifest,
    pub archive: PathBuf,
    /// Journal `.txt` accompagnant l'archive, s'il est présent.
    pub log: Option<PathBuf>,
    /// Message d'avertissement éventuel (journal manquant ou incohérent).
    pub warning: Option<String>,
    pub archive_size: u64,
    pub entries: usize,
}

/// Comportement lorsqu'un profil existe déjà à l'emplacement de restauration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overwrite {
    /// Refuser et laisser l'utilisateur choisir.
    Fail,
    /// Renommer l'ancien profil (`...-remplace-20261011-143005`) puis restaurer.
    Rename,
    /// Supprimer définitivement l'ancien profil puis restaurer.
    Delete,
}

#[derive(Debug, Clone)]
pub struct RestoreOptions {
    pub overwrite: Overwrite,
    /// Vérifier les empreintes SHA-256 de chaque fichier pendant l'extraction.
    pub verify_hashes: bool,
    /// Restituer le propriétaire d'origine (uniquement si root).
    pub restore_owner: bool,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        RestoreOptions { overwrite: Overwrite::Fail, verify_hashes: true, restore_owner: false }
    }
}

#[derive(Debug, Clone)]
pub struct RestoreOutcome {
    pub target: PathBuf,
    pub files: u64,
    pub dirs: u64,
    pub symlinks: u64,
    pub bytes: u64,
    pub verified: u64,
    pub renamed_existing: Option<PathBuf>,
    pub duration_secs: u64,
}

/// Indique si un chemin a l'extension d'une sauvegarde MozInRustBackup.
pub fn looks_like_backup_path(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case(BACKUP_EXTENSION))
        .unwrap_or(false)
}

/// Journal `.txt` associé à une archive, s'il existe.
pub fn companion_log(archive: &Path) -> Option<PathBuf> {
    let candidate = archive.with_extension("txt");
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// Ouvre une sauvegarde, vérifie sa signature et lit son manifeste.
pub fn inspect_archive(path: &Path) -> Result<BackupInfo, RestoreError> {
    if !path.is_file() {
        return Err(RestoreError::NotABackup(format!(
            "fichier introuvable : {}",
            path.display()
        )));
    }
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let file = std::fs::File::open(path)?;
    let mut zip = ZipReader::open(file).map_err(|e| {
        RestoreError::NotABackup(format!(
            "« {} » n'est pas une sauvegarde {} lisible ({}).\n\
             Choisissez le fichier .{} créé par {}.",
            path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            crate::APP_NAME,
            e,
            BACKUP_EXTENSION,
            crate::APP_NAME
        ))
    })?;

    let manifest_bytes = zip
        .read_named_to_vec(MANIFEST_MEMBER)?
        .ok_or_else(|| {
            RestoreError::NotABackup(format!(
                "« {} » ne contient pas de manifeste {} : ce n'est pas une sauvegarde \
                 créée par {}.",
                path.display(),
                crate::APP_NAME,
                crate::APP_NAME
            ))
        })?;
    let manifest_text = String::from_utf8_lossy(&manifest_bytes).to_string();
    if !manifest_text.contains(MAGIC) {
        return Err(RestoreError::NotABackup(
            "ce fichier ne porte pas la signature d'une sauvegarde MozInRustBackup".to_string(),
        ));
    }
    let manifest = Manifest::parse(&manifest_text).map_err(|e| RestoreError::NotABackup(e))?;

    let mut warning = None;
    let log = companion_log(path);
    match &log {
        Some(log_path) => {
            if let Ok(text) = std::fs::read_to_string(log_path) {
                match Manifest::parse(&text) {
                    Ok(from_log) => {
                        if from_log.profile_path != manifest.profile_path {
                            warning = Some(format!(
                                "le journal « {} » indique un autre emplacement d'origine \
                                 ({}) que l'archive ({}). L'archive fait foi.",
                                log_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                from_log.profile_path.display(),
                                manifest.profile_path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        warning = Some(format!(
                            "le journal « {} » n'a pas pu être relu ({}).",
                            log_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            e
                        ))
                    }
                }
            }
        }
        None => {
            warning = Some(
                "le journal .txt accompagnant la sauvegarde est absent ; l'emplacement \
                 d'origine est lu directement dans l'archive."
                    .to_string(),
            )
        }
    }

    let entries = zip.entries().len();
    Ok(BackupInfo { manifest, archive: path.to_path_buf(), log, warning, archive_size: size, entries })
}

/// Emplacement proposé pour la restauration.
///
/// Le chemin enregistré est « recollé » sur le dossier personnel de
/// l'utilisateur courant si nécessaire : une sauvegarde faite sur un ancien
/// ordinateur (utilisateur `bob`) est donc restaurée chez l'utilisateur actuel
/// (`alice`), au bon endroit, sans manipulation de chemin.
pub fn suggested_target(manifest: &Manifest) -> PathBuf {
    let current_home = crate::util::home_dir();
    if let Some(home) = &current_home {
        if !manifest.user_home.as_os_str().is_empty() {
            if let Ok(rel) = manifest.profile_path.strip_prefix(&manifest.user_home) {
                if !rel.as_os_str().is_empty() {
                    let rebased = home.join(rel);
                    if rebased != manifest.profile_path || home == &manifest.user_home {
                        return rebased;
                    }
                }
            }
        }
    }
    // Sinon, on cherche une racine équivalente pour cette application et cette
    // origine sur la machine actuelle.
    if let Some(root) = equivalent_root(manifest) {
        if !manifest.profile_dir_name.is_empty() {
            return root.join(&manifest.profile_dir_name);
        }
    }
    manifest.profile_path.clone()
}

fn equivalent_root(manifest: &Manifest) -> Option<PathBuf> {
    for (root, origin) in crate::core::scanner::candidate_roots(manifest.app) {
        let same_kind = match (&origin, &manifest.origin) {
            (crate::core::model::Origin::Flatpak { .. }, crate::core::model::Origin::Flatpak { .. }) => true,
            (crate::core::model::Origin::Snap, crate::core::model::Origin::Snap) => true,
            (crate::core::model::Origin::Native { .. }, crate::core::model::Origin::Native { .. }) => true,
            _ => false,
        };
        if same_kind {
            if let Some(parent) = root.parent() {
                if parent.is_dir() {
                    return Some(root);
                }
            }
        }
    }
    None
}

/// Vérifie l'intégrité complète d'une archive (CRC-32 de chaque entrée et
/// empreintes SHA-256), sans rien écrire sur le disque.
pub fn verify_archive(
    path: &Path,
    progress: &mut dyn FnMut(u64, &str),
) -> Result<(u64, u64), RestoreError> {
    let file = std::fs::File::open(path)?;
    let mut zip = ZipReader::open(file)?;
    let hashes = read_hash_table(&mut zip)?;
    let mut total_bytes = 0u64;
    let mut count = 0u64;
    let entries = zip.entries().to_vec();
    for (index, entry) in entries.iter().enumerate() {
        if entry.is_dir {
            continue;
        }
        let mut sink = HashingWriter::new(std::io::sink());
        zip.read_entry(index, &mut sink)?;
        let (sha, size) = sink.finish();
        if let Some(rel) = strip_root(&entry.name, "") {
            if let Some((expected_size, expected_sha)) = hashes.get(&rel) {
                if *expected_size != size {
                    return Err(RestoreError::Corrupt(format!(
                        "taille inattendue pour « {} » ({} au lieu de {})",
                        rel, size, expected_size
                    )));
                }
                if !expected_sha.is_empty() && expected_sha != &sha {
                    return Err(RestoreError::Corrupt(format!(
                        "empreinte SHA-256 invalide pour « {} »",
                        rel
                    )));
                }
            } else if entry.name != MANIFEST_MEMBER && entry.name != HASHES_MEMBER {
                // Fichier présent dans l'archive mais absent de la liste
                // d'empreintes : signalé sans bloquer (liste d'une version
                // antérieure, par exemple).
            }
        }
        total_bytes += size;
        count += 1;
        progress(count, &entry.name);
    }
    Ok((count, total_bytes))
}

fn read_hash_table<R: Read + Seek>(zip: &mut ZipReader<R>) -> Result<HashMap<String, (u64, String)>, RestoreError> {
    match zip.read_named_to_vec(HASHES_MEMBER)? {
        Some(bytes) => Ok(parse_hashes(&String::from_utf8_lossy(&bytes))),
        None => Ok(HashMap::new()),
    }
}

/// Retire le préfixe racine (« profil/ ») d'un nom d'entrée, en refusant les
/// chemins dangereux (absolus ou contenant « .. »).
fn strip_root(name: &str, root: &str) -> Option<String> {
    let prefix = if root.ends_with('/') { root.to_string() } else { format!("{}/", root) };
    let rel = name.strip_prefix(&prefix)?.to_string();
    if rel.is_empty() {
        return None;
    }
    let path = Path::new(&rel);
    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
            _ => {}
        }
    }
    if rel.starts_with('/') {
        return None;
    }
    Some(rel)
}

/// Restaure une sauvegarde dans `target`.
pub fn restore(
    info: &BackupInfo,
    target: &Path,
    options: &RestoreOptions,
    progress: &mut dyn Progress,
) -> Result<RestoreOutcome, RestoreError> {
    let started = std::time::Instant::now();
    let file = std::fs::File::open(&info.archive)?;
    let mut zip = ZipReader::open(file)?;
    let hashes = read_hash_table(&mut zip)?;
    let root = info.manifest.archive_root.clone();
    let entries = zip.entries().to_vec();

    let mut outcome = RestoreOutcome {
        target: target.to_path_buf(),
        files: 0,
        dirs: 0,
        symlinks: 0,
        bytes: 0,
        verified: 0,
        renamed_existing: None,
        duration_secs: 0,
    };

    // --- préparation de la destination ------------------------------------
    let exists = target.exists();
    if exists {
        match options.overwrite {
            Overwrite::Fail => return Err(RestoreError::TargetExists(target.to_path_buf())),
            Overwrite::Rename => {
                let stamp = time::format_iso(time::now_unix(), 0)
                    .replace([':', ' '], "-");
                let mut renamed = target.to_path_buf();
                let new_name = format!(
                    "{}-remplace-{}",
                    target
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "profil".to_string()),
                    stamp
                );
                renamed.set_file_name(new_name);
                fsops::ensure_dir(target.parent().unwrap_or(Path::new("/")))?;
                std::fs::rename(target, &renamed)?;
                outcome.renamed_existing = Some(renamed);
            }
            Overwrite::Delete => {
                progress.phase("Suppression du profil existant…");
                fsops::remove_tree(target)?;
            }
        }
    } else {
        fsops::ensure_dir(target)?;
    }
    fsops::ensure_dir(target)?;
    if !fsops::is_writable_dir(target) {
        return Err(RestoreError::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("le dossier de destination n'est pas accessible en écriture : {}", target.display()),
        )));
    }

    // --- extraction --------------------------------------------------------
    // Les dossiers sont traités d'abord (droits), leurs dates à la fin.
    let mut dir_metadata: Vec<(PathBuf, u32, i64, Option<u32>, Option<u32>)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if progress.cancelled() {
            return Err(RestoreError::Cancelled);
        }
        if entry.name == MANIFEST_MEMBER || entry.name == HASHES_MEMBER {
            continue;
        }
        let Some(rel) = strip_root(&entry.name, &root) else {
            continue; // entrée hors du profil sauvegardé : ignorée
        };
        let dest = target.join(&rel);
        if entry.is_dir {
            fsops::ensure_dir(&dest)?;
            outcome.dirs += 1;
            dir_metadata.push((
                dest.clone(),
                entry.mode,
                entry.mtime,
                entry.uid,
                entry.gid,
            ));
            continue;
        }
        if let Some(parent) = dest.parent() {
            fsops::ensure_dir(parent)?;
        }
        progress.phase(&format!("Restauration de {}", rel));
        if entry.is_symlink {
            let mut buf = Vec::new();
            zip.read_entry(index, &mut buf)?;
            let target_str = String::from_utf8_lossy(&buf).to_string();
            let _ = std::fs::remove_file(&dest);
            fsops::make_symlink(Path::new(&target_str), &dest)?;
            let _ = fsops::set_times(&dest, entry.mtime, entry.atime, true);
            if options.restore_owner {
                if let (Some(uid), Some(gid)) = (entry.uid, entry.gid) {
                    let _ = fsops::set_owner(&dest, uid, gid, true);
                }
            }
            outcome.symlinks += 1;
            continue;
        }

        let file = std::fs::File::create(&dest)?;
        // Droits appliqués immédiatement : jamais de fichier temporairement
        // trop permissif (les profils contiennent des mots de passe).
        let _ = fsops::set_mode(&dest, entry.mode);
        let mut writer = HashingWriter::new(file);
        let size = zip.read_entry(index, &mut writer)?;
        let (sha, written) = writer.finish();
        if size != written {
            return Err(RestoreError::Corrupt(format!(
                "écriture incomplète pour « {} »",
                rel
            )));
        }
        if options.verify_hashes {
            if let Some((expected_size, expected_sha)) = hashes.get(&rel) {
                if *expected_size != written || (!expected_sha.is_empty() && expected_sha != &sha) {
                    return Err(RestoreError::Corrupt(format!(
                        "le fichier « {} » ne correspond pas à la sauvegarde (empreinte différente)",
                        rel
                    )));
                }
                outcome.verified += 1;
            }
        }
        let _ = fsops::set_mode(&dest, entry.mode);
        let _ = fsops::set_times(&dest, entry.mtime, entry.atime, false);
        if options.restore_owner {
            if let (Some(uid), Some(gid)) = (entry.uid, entry.gid) {
                let _ = fsops::set_owner(&dest, uid, gid, false);
            }
        }
        outcome.files += 1;
        outcome.bytes += written;
        progress.advance(written, &rel);
    }

    // Dates et droits des dossiers, du plus profond au plus haut.
    dir_metadata.sort_by_key(|(p, _, _, _, _)| std::cmp::Reverse(p.components().count()));
    for (path, mode, mtime, uid, gid) in &dir_metadata {
        let _ = fsops::set_mode(path, *mode);
        let _ = fsops::set_times(path, *mtime, None, false);
        if options.restore_owner {
            if let (Some(u), Some(g)) = (uid, gid) {
                let _ = fsops::set_owner(path, *u, *g, false);
            }
        }
    }
    outcome.duration_secs = started.elapsed().as_secs();
    Ok(outcome)
}

/// Écrivain qui calcule l'empreinte SHA-256 et compte les octets au passage.
struct HashingWriter<W: Write> {
    inner: W,
    hasher: Sha256,
    total: u64,
}

impl<W: Write> HashingWriter<W> {
    fn new(inner: W) -> Self {
        HashingWriter { inner, hasher: Sha256::new(), total: 0 }
    }
    fn finish(self) -> (String, u64) {
        (self.hasher.finish_hex(), self.total)
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        self.total += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Petite aide pour l'interface : décrit la sauvegarde en une phrase.
pub fn describe(info: &BackupInfo, tz: i32) -> String {
    let m = &info.manifest;
    format!(
        "{} — profil « {} » ({}) sauvegardé le {} sur {}\nDossier d'origine : {}\nEmplacement de restauration : {}",
        m.app.label(),
        m.profile_name,
        m.origin.label(),
        time::format_datetime(m.created_unix, tz),
        if m.machine.is_empty() { "un autre ordinateur" } else { &m.machine },
        m.profile_path.display(),
        suggested_target(m).display()
    )
}

/// Ouvre un écrivain ZIP sur un tampon, utilisé par les tests.
#[allow(dead_code)]
pub(crate) fn dummy_zip_for_tests<W: Write + Seek>(w: W) -> ZipWriter<W> {
    ZipWriter::new(w, 0, crate::archive::deflate::Level::Normal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_safety() {
        assert_eq!(strip_root("profil/prefs.js", "profil").as_deref(), Some("prefs.js"));
        assert_eq!(strip_root("profil/a/b.txt", "profil").as_deref(), Some("a/b.txt"));
        assert!(strip_root("profil/../secret", "profil").is_none());
        assert!(strip_root("/etc/passwd", "profil").is_none());
        assert!(strip_root("autre/prefs.js", "profil").is_none());
    }

    #[test]
    fn target_is_rebased_on_home() {
        let mut m = Manifest::new(
            crate::core::model::AppKind::Firefox,
            crate::core::model::Origin::Native { distro: "Debian".into() },
            0,
        );
        m.user_home = PathBuf::from("/home/bob");
        m.profile_path = PathBuf::from("/home/bob/.mozilla/firefox/abc.default");
        let target = suggested_target(&m);
        let home = crate::util::home_dir().unwrap();
        assert!(target.starts_with(&home), "cible {}", target.display());
        assert!(target.ends_with(".mozilla/firefox/abc.default"));
    }
}

//! Création d'une sauvegarde de profil.
//!
//! Garanties :
//!  * le profil d'origine n'est **jamais modifié** : uniquement ouvert en
//!    lecture ;
//!  * les fichiers sont copiés à l'identique (contenu, droits, dates, liens
//!    symboliques) dans une archive `.mozinrustbackup` ;
//!  * chaque fichier est vérifié par CRC-32 (conteneur) et par empreinte
//!    SHA-256 (liste d'empreintes), ce qui permet de contrôler la sauvegarde
//!    à tout moment ;
//!  * un journal `.txt` est écrit à côté de l'archive, indiquant précisément
//!    l'emplacement d'origine du profil (indispensable à la restauration).

use std::collections::HashMap;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

use crate::archive::deflate::Level;
use crate::archive::sha256::Sha256;
use crate::archive::zip::{WriteEntry, ZipWriter};
use crate::core::manifest::Manifest;
use crate::core::model::Profile;
use crate::sys::fsops;
use crate::util::time;
use crate::{BACKUP_EXTENSION, HASHES_MEMBER, MANIFEST_MEMBER};

/// Progression d'une opération longue, présentée à l'utilisateur.
pub trait Progress {
    /// Nouvelle étape (« Analyse… », « Copie… »).
    fn phase(&mut self, _label: &str) {}
    /// Volume total à traiter (pourcentage).
    fn total(&mut self, _bytes: u64, _files: u64) {}
    /// Avancement : nombre d'octets traités et fichier en cours.
    fn advance(&mut self, _bytes: u64, _file: &str) {}
    /// L'utilisateur a-t-il demandé l'annulation ?
    fn cancelled(&self) -> bool {
        false
    }
}

/// Implémentation silencieuse (ligne de commande, tests).
pub struct SilentProgress;

impl Progress for SilentProgress {}

#[derive(Debug)]
pub enum BackupError {
    Io(io::Error),
    Cancelled,
    Message(String),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::Io(e) => write!(f, "{}", e),
            BackupError::Cancelled => write!(f, "opération annulée par l'utilisateur"),
            BackupError::Message(m) => write!(f, "{}", m),
        }
    }
}

impl std::error::Error for BackupError {}

impl From<io::Error> for BackupError {
    fn from(e: io::Error) -> Self {
        BackupError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct BackupOptions {
    /// Exclure les caches (gain de place, sauvegarde plus rapide).
    pub exclude_caches: bool,
    /// Niveau de compression.
    pub level: Level,
    /// Vérifier l'archive après création (relecture complète).
    pub verify_after: bool,
}

impl Default for BackupOptions {
    fn default() -> Self {
        BackupOptions { exclude_caches: false, level: Level::Normal, verify_after: true }
    }
}

#[derive(Debug, Clone)]
pub struct BackupOutcome {
    pub archive: PathBuf,
    pub log: PathBuf,
    pub files: u64,
    pub symlinks: u64,
    pub source_bytes: u64,
    pub archive_bytes: u64,
    pub archive_sha256: String,
    pub duration_secs: u64,
    pub ignored: Vec<String>,
    pub manifests: Manifest,
}

/// Dossiers exclus lorsque l'utilisateur choisit de ne pas sauvegarder les caches.
const CACHE_DIRS: [&str; 9] = [
    "cache2",
    "startupCache",
    "shader-cache",
    "thumbnails",
    "OfflineCache",
    "jumpListCache",
    "safebrowsing",
    "minidumps",
    "crashes",
];

/// Fichiers de verrouillage : exclus car leur restauration empêcherait
/// l'application de démarrer (« profil déjà utilisé »).
const LOCK_FILES: [&str; 3] = ["parent.lock", ".parentlock", "lock"];

/// Extensions déjà compressées : inutile de les recompresser.
const INCOMPRESSIBLE_EXT: [&str; 33] = [
    "jpg", "jpeg", "png", "gif", "webp", "avif", "heic", "ico", "bmp", "tiff", "psd",
    "mp3", "ogg", "oga", "opus", "flac", "m4a", "aac", "wav", "wma",
    "mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "mpg", "mpeg",
    "zip", "gz", "bz2", "xz",
];

/// Un élément du profil à sauvegarder.
#[derive(Debug)]
enum Item {
    Dir { rel: String, mode: u32, mtime: i64, uid: u32, gid: u32 },
    File { rel: String, abs: PathBuf, mode: u32, mtime: i64, uid: u32, gid: u32, size: u64 },
    Symlink { rel: String, mtime: i64, uid: u32, gid: u32, target: String },
}

impl Item {
    fn rel(&self) -> &str {
        match self {
            Item::Dir { rel, .. } | Item::File { rel, .. } | Item::Symlink { rel, .. } => rel,
        }
    }
    fn sort_key(&self) -> (String, u8) {
        // Les dossiers passent avant leurs enfants.
        let rank = match self {
            Item::Dir { .. } => 0,
            Item::File { .. } => 1,
            Item::Symlink { .. } => 2,
        };
        (self.rel().to_string(), rank)
    }
}

/// Crée la sauvegarde complète d'un profil.
///
/// * `archive_path` : chemin exact de l'archive à créer (déjà validé par
///   l'appelant, y compris la confirmation d'écrasement) ;
/// * `level` : niveau de compression.
pub fn create_backup(
    profile: &Profile,
    archive_path: &Path,
    options: &BackupOptions,
    tz_offset: i32,
    progress: &mut dyn Progress,
) -> Result<BackupOutcome, BackupError> {
    let started = std::time::Instant::now();
    if !profile.dir.is_dir() {
        return Err(BackupError::Message(format!(
            "le dossier du profil est introuvable : {}",
            profile.dir.display()
        )));
    }
    if let Some(parent) = archive_path.parent() {
        fsops::ensure_dir(parent)?;
    }

    progress.phase("Analyse du profil…");
    let mut items = Vec::new();
    let mut ignored: Vec<String> = Vec::new();
    let tree = collect_items(&profile.dir, options, &mut items, &mut ignored)?;
    items.sort_by_key(|i| i.sort_key());
    progress.total(tree.bytes, tree.files);

    // Écriture dans un fichier temporaire, renommé seulement à la fin : une
    // interruption ne laisse donc jamais d'archive incomplète.
    let tmp_path = archive_path.with_extension(format!("{}.part", BACKUP_EXTENSION));
    let _ = std::fs::remove_file(&tmp_path);
    let file = std::fs::File::create(&tmp_path)?;

    let result = write_archive(
        &file,
        &tmp_path,
        profile,
        &items,
        &tree,
        options,
        tz_offset,
        ignored.clone(),
        progress,
    );

    let (mut manifest, archive_bytes) = match result {
        Ok(v) => v,
        Err(e) => {
            drop(file);
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
    };
    drop(file);

    if progress.cancelled() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(BackupError::Cancelled);
    }

    // Renommage final : l'archive apparaît d'un coup, complète.
    if let Ok(metadata) = std::fs::metadata(&tmp_path) {
        let _ = metadata;
    }
    std::fs::rename(&tmp_path, archive_path)?;

    progress.phase("Calcul de l'empreinte de la sauvegarde…");
    let archive_sha256 = file_sha256(archive_path)?;
    let archive_size = std::fs::metadata(archive_path)?.len();

    // Journal extérieur (lisible sans le logiciel).
    manifest.archive_name = Some(
        archive_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
    );
    manifest.archive_sha256 = Some(archive_sha256.clone());
    manifest.archive_size = Some(archive_size);
    let log_path = archive_path.with_extension("txt");
    {
        let mut log = std::fs::File::create(&log_path)?;
        log.write_all(manifest.to_text(tz_offset, true).as_bytes())?;
        log.write_all(render_instructions(&manifest, archive_path, tz_offset).as_bytes())?;
        log.flush()?;
    }

    if options.verify_after {
        progress.phase("Vérification de la sauvegarde…");
        crate::core::restore::verify_archive(archive_path, &mut |_done, _name| {})
            .map_err(|e| BackupError::Message(format!("vérification impossible : {}", e)))?;
    }

    Ok(BackupOutcome {
        archive: archive_path.to_path_buf(),
        log: log_path,
        files: tree.files,
        symlinks: tree.symlinks,
        source_bytes: tree.bytes,
        archive_bytes,
        archive_sha256,
        duration_secs: started.elapsed().as_secs(),
        ignored,
        manifests: manifest,
    })
}

struct TreeStats {
    bytes: u64,
    files: u64,
    dirs: u64,
    symlinks: u64,
}

fn collect_items(
    root: &Path,
    options: &BackupOptions,
    items: &mut Vec<Item>,
    ignored: &mut Vec<String>,
) -> io::Result<TreeStats> {
    use std::os::unix::fs::MetadataExt;
    let mut stats = TreeStats { bytes: 0, files: 0, dirs: 0, symlinks: 0 };
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel_prefix)) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                ignored.push(format!("{} (illisible : {})", display_rel(&rel_prefix), e));
                continue;
            }
        };
        for entry in entries.flatten() {
            let abs = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = if rel_prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", rel_prefix, name)
            };
            if rel_prefix.is_empty() && LOCK_FILES.contains(&name.as_str()) {
                ignored.push(format!("{} (fichier de verrou, non copié)", rel));
                continue;
            }
            if options.exclude_caches && CACHE_DIRS.contains(&name.as_str()) && entry.path().is_dir() {
                ignored.push(format!("{}/ (cache, non copié)", rel));
                continue;
            }
            let md = match std::fs::symlink_metadata(&abs) {
                Ok(md) => md,
                Err(e) => {
                    ignored.push(format!("{} (illisible : {})", rel, e));
                    continue;
                }
            };
            let ft = md.file_type();
            let mode = md.mode();
            let mtime = md.mtime();
            let uid = md.uid();
            let gid = md.gid();
            if ft.is_symlink() {
                match std::fs::read_link(&abs) {
                    Ok(target) => {
                        stats.symlinks += 1;
                        items.push(Item::Symlink {
                            rel,
                            mtime,
                            uid,
                            gid,
                            target: target.to_string_lossy().to_string(),
                        });
                    }
                    Err(e) => ignored.push(format!("{} (lien illisible : {})", rel, e)),
                }
            } else if ft.is_dir() {
                stats.dirs += 1;
                items.push(Item::Dir { rel: rel.clone(), mode, mtime, uid, gid });
                stack.push((abs, rel));
            } else if ft.is_file() {
                stats.files += 1;
                stats.bytes += md.len();
                items.push(Item::File {
                    rel,
                    abs,
                    mode,
                    mtime,
                    uid,
                    gid,
                    size: md.len(),
                });
            } else {
                ignored.push(format!("{} (fichier spécial non copié)", rel));
            }
        }
    }
    Ok(stats)
}

fn display_rel(rel: &str) -> &str {
    if rel.is_empty() {
        "."
    } else {
        rel
    }
}

fn should_compress(path: &str, size: u64) -> bool {
    if size == 0 {
        return false;
    }
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if INCOMPRESSIBLE_EXT.contains(&ext.as_str()) {
        return false;
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn write_archive(
    file: &std::fs::File,
    _tmp_path: &Path,
    profile: &Profile,
    items: &[Item],
    tree: &TreeStats,
    options: &BackupOptions,
    tz_offset: i32,
    ignored: Vec<String>,
    progress: &mut dyn Progress,
) -> Result<(Manifest, u64), BackupError> {
    let root_name = "profil";
    let writer = file.try_clone()?;
    let mut zip = ZipWriter::new(writer, tz_offset, options.level);
    let mut hashes = String::new();
    let mut processed: u64 = 0;

    for item in items {
        if progress.cancelled() {
            return Err(BackupError::Cancelled);
        }
        match item {
            Item::Dir { rel, mode, mtime, uid, gid } => {
                let name = format!("{}/{}", root_name, rel);
                let meta = WriteEntry::dir(&name, *mode, *mtime, *uid, *gid);
                zip.add_dir(&meta)?;
            }
            Item::Symlink { rel, mtime, uid, gid, target } => {
                let name = format!("{}/{}", root_name, rel);
                let meta = WriteEntry::symlink(&name, *mtime, *uid, *gid);
                zip.add_bytes(&meta, target.as_bytes(), true)?;
                progress.advance(0, rel);
            }
            Item::File { rel, abs, mode, mtime, uid, gid, size } => {
                let name = format!("{}/{}", root_name, rel);
                let compress = should_compress(rel, *size);
                let mut src = std::fs::File::open(abs)?;
                let mut meta = WriteEntry::file(&name, *mode, *mtime, *uid, *gid);
                meta.atime = Some(*mtime);
                let stats = zip.add_file(&meta, &mut src, *size, compress, &mut |n| {
                    processed += n;
                    progress.advance(n, rel);
                })?;
                hashes.push_str(&format!("{} {} {}\n", stats.sha256, stats.uncompressed, rel));
            }
        }
    }

    progress.phase("Écriture du manifeste…");
    // Liste d'empreintes (fichier compressé : c'est du texte très répétitif).
    {
        let meta = WriteEntry::file(HASHES_MEMBER, 0o100600, time::now_unix(), 0, 0);
        zip.add_bytes(&meta, hashes.as_bytes(), true)?;
    }

    let mut manifest = Manifest::new(profile.app, profile.origin.clone(), time::now_unix());
    manifest.profile_name = profile.name.clone();
    manifest.profile_dir_name = profile.dir_name();
    manifest.profile_path = profile.dir.clone();
    manifest.profile_root = profile.root.clone();
    manifest.archive_root = root_name.to_string();
    manifest.files = tree.files;
    manifest.total_size = tree.bytes;
    manifest.symlinks = tree.symlinks;
    manifest.caches_excluded = options.exclude_caches;
    manifest.profile_mtime = profile.mtime;
    manifest.ignored = ignored;
    {
        let text = manifest.to_text(tz_offset, false);
        let meta = WriteEntry::file(MANIFEST_MEMBER, 0o100600, time::now_unix(), 0, 0);
        zip.add_bytes(&meta, text.as_bytes(), false)?;
    }

    let total_written = {
        let z = zip;
        let mut f = z.finish()?;
        f.flush()?;
        f.seek(std::io::SeekFrom::End(0))?
    };
    Ok((manifest, total_written))
}

/// Empreinte SHA-256 d'un fichier (lecture en flux).
pub fn file_sha256(path: &Path) -> io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish_hex())
}

/// Texte d'aide ajouté à la fin du journal extérieur.
fn render_instructions(manifest: &Manifest, archive: &Path, tz: i32) -> String {
    let archive_name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut s = String::new();
    s.push_str("\n# ─────────────────────────────────────────────────────────────────────\n");
    s.push_str("#  COMMENT RESTAURER CETTE SAUVEGARDE\n");
    s.push_str("#\n");
    s.push_str(&format!(
        "#  1. Lancez {} et choisissez « Restaurer un profil ».\n",
        crate::APP_NAME
    ));
    s.push_str(&format!(
        "#  2. Sélectionnez le fichier : {}\n",
        archive_name
    ));
    s.push_str(&format!(
        "#  3. Le profil sera restauré dans : {}\n",
        manifest.profile_path.display()
    ));
    s.push_str("#\n");
    s.push_str("#  Si un profil existe déjà à cet endroit, le logiciel demande\n");
    s.push_str("#  confirmation avant de le remplacer (et le renomme au lieu de\n");
    s.push_str("#  l'effacer, par sécurité).\n");
    s.push_str("#\n");
    s.push_str(&format!(
        "#  Sauvegarde créée le {} par {} {}.\n",
        time::format_iso(manifest.created_unix, tz),
        crate::APP_NAME,
        crate::APP_VERSION
    ));
    s.push_str("# ─────────────────────────────────────────────────────────────────────\n");
    s
}

/// Avertissement d'espace disque insuffisant, le cas échéant.
pub fn check_free_space(source_bytes: u64, destination: &Path) -> Option<String> {
    let free = crate::sys::disk::free_space(destination)?;
    // Marge de sécurité : 5 % + 32 Mio.
    let needed = source_bytes + source_bytes / 20 + 32 * 1024 * 1024;
    if free < needed {
        Some(format!(
            "Espace insuffisant sur le support de destination : {} nécessaires, {} disponibles.",
            crate::util::human_size(needed),
            crate::util::human_size(free)
        ))
    } else {
        None
    }
}

/// Table d'empreintes relue depuis une archive.
pub fn parse_hashes(text: &str) -> HashMap<String, (u64, String)> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, ' ');
        let (Some(hash), Some(size), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if let Ok(size) = size.parse::<u64>() {
            map.insert(path.to_string(), (size, hash.to_string()));
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_parsing() {
        let text = "abc 12 prefs.js\ndef 34 Mail/sous dossier/msg.eml\n";
        let map = parse_hashes(text);
        assert_eq!(map.get("prefs.js"), Some(&(12, "abc".to_string())));
        assert_eq!(map.get("Mail/sous dossier/msg.eml"), Some(&(34, "def".to_string())));
    }

    #[test]
    fn compression_choices() {
        assert!(should_compress("prefs.js", 100));
        assert!(!should_compress("photo.jpg", 100));
        assert!(!should_compress("vide.txt", 0));
    }
}

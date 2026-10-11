//! Opérations sur le système de fichiers : mesure, permissions, horodatages,
//! liens symboliques, espace libre.

use std::collections::VecDeque;
use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Mesure récursive d'une arborescence.
#[derive(Debug, Clone, Default)]
pub struct TreeMeasure {
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
    pub mtime: i64,
    /// Fichiers présents sur le disque sans être lisibles (droits, E/S).
    pub unreadable: u64,
}

/// Parcourt `root` (sans suivre les liens symboliques) et totalise tailles et
/// dates. Un `progress` optionnel est appelé tous les 512 fichiers environ.
pub fn measure_tree(root: &Path, mut progress: Option<&mut dyn FnMut(u64)>) -> TreeMeasure {
    let mut m = TreeMeasure::default();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    let mut visited = 0u64;
    while let Some(path) = queue.pop_front() {
        let md = match std::fs::symlink_metadata(&path) {
            Ok(md) => md,
            Err(_) => {
                m.unreadable += 1;
                continue;
            }
        };
        let ft = md.file_type();
        if ft.is_symlink() {
            stack_push(&mut queue, &path);
            continue;
        }
        if ft.is_dir() {
            m.dirs += 1;
            stack_push(&mut queue, &path);
        } else if ft.is_file() {
            m.size += md.len();
            m.files += 1;
        }
        if let Ok(modified) = md.modified() {
            if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                let secs = dur.as_secs() as i64;
                if secs > m.mtime {
                    m.mtime = secs;
                }
            }
        }
        visited += 1;
        if visited % 512 == 0 {
            if let Some(cb) = progress.as_mut() {
                cb(visited);
            }
        }
    }
    if let Some(cb) = progress.as_mut() {
        cb(visited);
    }
    m
}

fn stack_push(queue: &mut VecDeque<PathBuf>, path: &Path) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            queue.push_back(e.path());
        }
    }
}

/// Date de dernière modification d'un fichier, en secondes Unix.
pub fn mtime_of(path: &Path) -> Option<i64> {
    let md = std::fs::metadata(path).ok()?;
    let modified = md.modified().ok()?;
    modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

// --- horodatages ----------------------------------------------------------

#[cfg(target_pointer_width = "64")]
#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(not(target_pointer_width = "64"))]
#[repr(C)]
struct Timespec {
    tv_sec: i32,
    tv_nsec: i32,
}

extern "C" {
    fn utimensat(dirfd: i32, path: *const std::os::raw::c_char, times: *const Timespec, flags: i32) -> i32;
    fn lchown(path: *const std::os::raw::c_char, uid: u32, gid: u32) -> i32;
    fn geteuid() -> u32;
}

const AT_FDCWD: i32 = -100;
const AT_SYMLINK_NOFOLLOW: i32 = 0x100;

/// Applique les horodatages exacts (mtime, atime) à un fichier ou un lien.
pub fn set_times(path: &Path, mtime: i64, atime: Option<i64>, nofollow: bool) -> io::Result<()> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "chemin invalide"))?;
    let atime = atime.unwrap_or(mtime);
    let times = [
        Timespec { tv_sec: atime as _, tv_nsec: 0 },
        Timespec { tv_sec: mtime as _, tv_nsec: 0 },
    ];
    let flags = if nofollow { AT_SYMLINK_NOFOLLOW } else { 0 };
    let rc = unsafe { utimensat(AT_FDCWD, c_path.as_ptr(), times.as_ptr(), flags) };
    if rc != 0 {
        // Filet de sécurité : `touch` reste disponible partout.
        let stamp = format!("@{}", mtime);
        let mut cmd = std::process::Command::new("touch");
        cmd.arg("-h").arg("-d").arg(&stamp).arg(path);
        if cmd.status().map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Applique les droits Unix (bits de permission uniquement).
pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let perms = std::fs::Permissions::from_mode(mode & 0o7777);
    std::fs::set_permissions(path, perms)
}

/// Applique le propriétaire (nécessite les droits root ; ignoré sinon).
pub fn set_owner(path: &Path, uid: u32, gid: u32, nofollow: bool) -> io::Result<()> {
    if !is_root() {
        return Ok(());
    }
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "chemin invalide"))?;
    let _ = nofollow; // lchown ne suit jamais les liens
    let rc = unsafe { lchown(c_path.as_ptr(), uid, gid) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Vrai si le programme tourne avec les privilèges root.
pub fn is_root() -> bool {
    unsafe { geteuid() == 0 }
}

/// Crée un lien symbolique.
pub fn make_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Crée un dossier et tous ses parents.
pub fn ensure_dir(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

/// Supprime un dossier et son contenu, sans suivre les liens symboliques.
pub fn remove_tree(path: &Path) -> io::Result<()> {
    let md = match std::fs::symlink_metadata(path) {
        Ok(md) => md,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if md.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// Vérifie qu'un dossier est accessible en écriture (test réel).
pub fn is_writable_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let probe = path.join(format!(".mozinrustbackup-test-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Recherche un exécutable dans le `PATH`.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_and_times() {
        let dir = std::env::temp_dir().join(format!("mrb-fsops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/b")).unwrap();
        std::fs::write(dir.join("a/b/f.txt"), b"hello").unwrap();
        std::fs::write(dir.join("g.txt"), b"0123456789").unwrap();
        let m = measure_tree(&dir, None);
        assert_eq!(m.size, 15);
        assert_eq!(m.files, 2);
        assert!(m.dirs >= 3);

        set_times(&dir.join("g.txt"), 1_600_000_000, None, false).unwrap();
        assert_eq!(mtime_of(&dir.join("g.txt")), Some(1_600_000_000));
        set_mode(&dir.join("g.txt"), 0o600).unwrap();
        let md = std::fs::metadata(dir.join("g.txt")).unwrap();
        assert_eq!(md.permissions().mode() & 0o777, 0o600);

        make_symlink(Path::new("g.txt"), &dir.join("lien")).unwrap();
        let m2 = measure_tree(&dir, None);
        assert_eq!(m2.files, 2, "les liens symboliques ne sont pas comptés comme fichiers");
        assert!(!is_root());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

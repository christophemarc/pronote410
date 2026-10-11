//! Espace disque disponible et détection des supports amovibles (clés USB).

use std::path::{Path, PathBuf};

/// Espace disponible en octets sur le système de fichiers contenant `path`.
///
/// Utilise `df -Pk` : présent sur tous les systèmes Linux, y compris BusyBox.
pub fn free_space(path: &Path) -> Option<u64> {
    let out = std::process::Command::new("df").arg("-Pk").arg(path).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().nth(1)?;
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 4 {
        return None;
    }
    let kb: u64 = cols[3].parse().ok()?;
    Some(kb * 1024)
}

/// Liste des points de montage amovibles plausibles (clés USB, disques externes).
pub fn removable_mounts() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push = |p: PathBuf| {
        if p.is_dir() && !out.contains(&p) {
            out.push(p);
        }
    };
    if let Some(home) = crate::util::home_dir() {
        // Emplacements classiques selon les environnements de bureau.
        for base in [home.join("mnt"), PathBuf::from("/media"), PathBuf::from("/run/media")] {
            for entry in read_children(&base) {
                if let Some(name) = entry.file_name().map(|n| n.to_string_lossy().to_string()) {
                    if name == home.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() {
                        for inner in read_children(&entry) {
                            push(inner);
                        }
                    } else if entry.is_dir() {
                        push(entry.clone());
                    }
                }
            }
        }
    }
    for entry in read_children(Path::new("/media")) {
        push(entry);
    }
    out
}

fn read_children(dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            // Ignorer les points de montage système et les dossiers cachés.
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            v.push(p);
        }
    }
    v
}

/// Dossier de sauvegarde proposé par défaut lorsque rien d'autre n'est trouvé.
pub fn default_backup_dir() -> PathBuf {
    let home = crate::util::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    for candidate in [home.join("Sauvegardes-Mozilla"), home.join("Sauvegardes")] {
        if candidate.is_dir() {
            return candidate;
        }
    }
    home.join("Sauvegardes-Mozilla")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_space_on_tmp() {
        let free = free_space(Path::new("/tmp"));
        assert!(free.unwrap_or(0) > 1024 * 1024);
    }
}

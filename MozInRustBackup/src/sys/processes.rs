//! Détection et arrêt des applications Mozilla en cours d'exécution.
//!
//! Un profil ouvert pendant la sauvegarde peut être incohérent (fichiers
//! modifiés en cours de lecture) : le programme prévient donc l'utilisateur et
//! lui propose de fermer proprement l'application avant de continuer.

use std::path::Path;

use crate::core::model::AppKind;

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

const SIGTERM: i32 = 15;
const SIGKILL: i32 = 9;

#[derive(Debug, Clone)]
pub struct RunningApp {
    pub pid: i32,
    pub name: String,
    pub app: Option<AppKind>,
    /// Chemin du profil utilisé, si détectable dans la ligne de commande.
    pub profile: Option<String>,
}

fn read_cmdline(pid: i32) -> Option<Vec<String>> {
    let path = format!("/proc/{}/cmdline", pid);
    let data = std::fs::read(path).ok()?;
    let parts: Vec<String> = data
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).to_string())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

fn classify(cmdline: &[String]) -> (String, Option<AppKind>, Option<String>) {
    let exe = cmdline
        .first()
        .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
        .unwrap_or_default();
    let lower = exe.to_ascii_lowercase();
    let app = if lower.contains("firefox") {
        Some(AppKind::Firefox)
    } else if lower.contains("thunderbird") {
        Some(AppKind::Thunderbird)
    } else {
        None
    };
    let mut profile = None;
    let mut iter = cmdline.iter();
    while let Some(arg) = iter.next() {
        if arg == "-profile" || arg == "--profile" {
            profile = iter.next().cloned();
            break;
        } else if let Some(rest) = arg.strip_prefix("-profile=") {
            profile = Some(rest.to_string());
            break;
        } else if let Some(rest) = arg.strip_prefix("--profile=") {
            profile = Some(rest.to_string());
            break;
        }
    }
    (exe, app, profile)
}

/// Liste les processus Mozilla (Firefox, Thunderbird) de l'utilisateur courant.
pub fn running_apps() -> Vec<RunningApp> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(pid) = name.parse::<i32>() else { continue };
        let Some(cmdline) = read_cmdline(pid) else { continue };
        let (exe, app, profile) = classify(&cmdline);
        if app.is_some() {
            out.push(RunningApp { pid, name: exe, app, profile });
        }
    }
    out
}

/// Applications en cours d'exécution pour l'application demandée.
pub fn running_for(app: AppKind) -> Vec<RunningApp> {
    running_apps().into_iter().filter(|r| r.app == Some(app)).collect()
}

/// Vrai si le dossier de profil contient un verrou actif.
pub fn profile_locked(profile_dir: &Path) -> bool {
    for lock in ["parent.lock", ".parentlock", "lock"] {
        let p = profile_dir.join(lock);
        if let Ok(md) = std::fs::symlink_metadata(&p) {
            // Un verrou de taille nulle ou un lien symbolique signale une session
            // ouverte (Firefox supprime ces fichiers en quittant proprement).
            if md.is_file() || md.file_type().is_symlink() {
                return true;
            }
        }
    }
    false
}

/// Demande l'arrêt propre du processus (SIGTERM).
pub fn request_terminate(pid: i32) -> bool {
    unsafe { kill(pid, SIGTERM) == 0 }
}

/// Force l'arrêt (SIGKILL), après un délai d'attente.
pub fn force_terminate(pid: i32) -> bool {
    unsafe { kill(pid, SIGKILL) == 0 }
}

/// Attend la fin du processus pendant au plus `timeout_ms` millisecondes.
pub fn wait_exit(pid: i32, timeout_ms: u64) -> bool {
    let step = 100u64;
    let mut waited = 0u64;
    while waited < timeout_ms {
        if !Path::new(&format!("/proc/{}", pid)).exists() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(step));
        waited += step;
    }
    !Path::new(&format!("/proc/{}", pid)).exists()
}

/// Ferme proprement toutes les instances de l'application, en attendant leur
/// sortie complète (l'écriture des fichiers doit être terminée).
/// Renvoie la liste des PID qui n'ont pas pu être arrêtés.
pub fn close_application(app: AppKind, force_after_ms: u64) -> Vec<i32> {
    let running = running_for(app);
    let mut pids = Vec::new();
    for r in &running {
        request_terminate(r.pid);
        pids.push(r.pid);
    }
    let mut stuck = Vec::new();
    for pid in pids {
        if !wait_exit(pid, force_after_ms) {
            force_terminate(pid);
            if !wait_exit(pid, 2_000) {
                stuck.push(pid);
            }
        }
    }
    stuck
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_does_not_crash() {
        let apps = running_apps();
        for a in apps {
            assert!(a.pid > 0);
        }
    }

    #[test]
    fn classify_command_lines() {
        let mut c = classify(&[
            "/usr/lib/firefox/firefox".to_string(),
            "-profile".to_string(),
            "/home/moi/.mozilla/firefox/abc.default".to_string(),
        ]);
        assert_eq!(c.1, Some(AppKind::Firefox));
        assert_eq!(c.2.as_deref(), Some("/home/moi/.mozilla/firefox/abc.default"));
        c = classify(&["/usr/bin/thunderbird".to_string()]);
        assert_eq!(c.1, Some(AppKind::Thunderbird));
        assert!(c.2.is_none());
        c = classify(&["/usr/bin/nano".to_string()]);
        assert_eq!(c.1, None);
    }
}

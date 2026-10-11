//! Détection des profils Firefox et Thunderbird présents sur le disque.
//!
//! Emplacements inspectés :
//!
//! | Type d'installation | Firefox | Thunderbird |
//! |---|---|---|
//! | native (distribution) | `~/.mozilla/firefox` | `~/.thunderbird` |
//! | Flatpak | `~/.var/app/org.mozilla.firefox/.mozilla/firefox` | `~/.var/app/org.mozilla.Thunderbird/.thunderbird` |
//! | Snap | `~/snap/firefox/common/.mozilla/firefox` | `~/snap/thunderbird/common/.thunderbird` |
//!
//! Les profils sont lus dans `profiles.ini` (méthode actuelle) ou dans
//! `profiles.json` (anciennes versions Snap/Flatpak) ; à défaut, les dossiers
//! contenant un profil sont reconnus à leurs fichiers caractéristiques.

use std::path::{Path, PathBuf};

use crate::core::ini::{parse_profiles_json, IniFile};
use crate::core::model::{AppKind, Origin, Profile, ScannedRoot};
use crate::sys::fsops;
use crate::sys::osrelease;

/// Marqueurs attestant la présence d'un profil Mozilla dans un dossier.
const PROFILE_MARKERS: [&str; 12] = [
    "prefs.js",
    "times.json",
    "compatibility.ini",
    "places.sqlite",
    "key4.db",
    "cert9.db",
    "logins.json",
    "abook.sqlite",
    "xulstore.json",
    "extensions.json",
    "Mail",
    "ImapMail",
];

#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Applications à rechercher.
    pub apps: Vec<AppKind>,
    /// Emplacements supplémentaires (autre disque, sauvegarde restaurée…).
    pub extra_roots: Vec<PathBuf>,
    /// Rechercher en profondeur dans les emplacements supplémentaires.
    pub deep_search: bool,
    /// Calculer taille / date / verrou de chaque profil.
    pub measure: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            apps: vec![AppKind::Firefox, AppKind::Thunderbird],
            extra_roots: Vec::new(),
            deep_search: false,
            measure: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanReport {
    pub profiles: Vec<Profile>,
    pub roots: Vec<ScannedRoot>,
}

/// Emplacements standards d'un profil pour l'application demandée.
pub fn candidate_roots(app: AppKind) -> Vec<(PathBuf, Origin)> {
    let mut out = Vec::new();
    let distro = osrelease::distro_label();
    if let Some(home) = crate::util::home_dir() {
        // Installation native de la distribution.
        let native_root = home.join(app.relative_profile_root());
        out.push((native_root, Origin::Native { distro: distro.clone() }));

        // Flatpak (dossier utilisateur ~/.var/app/<identifiant>).
        for app_id in app.default_roots() {
            let base = home.join(".var/app").join(app_id);
            if base.is_dir() {
                let root = base.join(app.relative_profile_root());
                out.push((root, Origin::Flatpak { app_id: app_id.to_string() }));
            }
        }

        // Snap (dossier ~/snap/<nom>/common).
        let snap_name = app.app_dir();
        let snap_base = home.join("snap").join(snap_name).join("common");
        if snap_base.is_dir() {
            let root = snap_base.join(app.relative_profile_root());
            out.push((root, Origin::Snap));
        }
        // Certaines versions de Snap utilisent ~/snap/<nom>/current.
        let snap_current = home.join("snap").join(snap_name).join("current");
        if snap_current.is_dir() {
            let root = snap_current.join(app.relative_profile_root());
            if root.is_dir() && !out.iter().any(|(p, _)| p == &root) {
                out.push((root, Origin::Snap));
            }
        }
    }
    out
}

/// Analyse complète du disque.
pub fn scan(opts: &ScanOptions, progress: &mut dyn FnMut(&str)) -> ScanReport {
    let mut report = ScanReport { profiles: Vec::new(), roots: Vec::new() };
    for app in &opts.apps {
        progress(&format!("Recherche des profils {}…", app.label()));
        for (root, origin) in candidate_roots(*app) {
            let found = collect_root(*app, &root, origin.clone(), &mut report.profiles);
            report.roots.push(ScannedRoot {
                path: root.clone(),
                origin: origin.clone(),
                app: *app,
                found,
            });
        }
    }

    // Emplacements supplémentaires fournis par l'utilisateur.
    for extra in &opts.extra_roots {
        progress(&format!("Analyse de {}…", extra.display()));
        if opts.deep_search {
            let found = deep_search(extra, &opts.apps, 6);
            report.profiles.extend(found);
        } else {
            let origin = infer_origin_from_path(extra);
            if let Some(app) = infer_app_from_path(extra) {
                collect_root(app, extra, origin.clone(), &mut report.profiles);
            } else {
                for app in &opts.apps {
                    collect_root(*app, extra, origin.clone(), &mut report.profiles);
                }
            }
        }
    }

    // Dédoublonnage (le même dossier peut être trouvé par deux chemins).
    let mut seen: Vec<PathBuf> = Vec::new();
    report.profiles.retain(|p| {
        let key = p.dir.clone();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });

    if opts.measure {
        let total = report.profiles.len();
        for (i, profile) in report.profiles.iter_mut().enumerate() {
            progress(&format!("Analyse du profil {} / {} : {}", i + 1, total, profile.name));
            measure_profile(profile);
        }
    }

    sort_profiles(&mut report.profiles);
    report
}

/// Tri : Firefox puis Thunderbird, profils les plus récents d'abord.
pub fn sort_profiles(profiles: &mut [Profile]) {
    // Les anciennes copies créées lors d'une restauration passent en dernier.
    let legacy = |p: &Profile| u8::from(p.name.contains("(ancienne copie)"));
    profiles.sort_by(|a, b| {
        legacy(a)
            .cmp(&legacy(b))
            .then_with(|| {
        let ka = match a.app {
            AppKind::Firefox => 0,
            AppKind::Thunderbird => 1,
        };
        let kb = match b.app {
            AppKind::Firefox => 0,
            AppKind::Thunderbird => 1,
        };
        ka.cmp(&kb)
            .then_with(|| b.mtime.unwrap_or(0).cmp(&a.mtime.unwrap_or(0)))
            .then_with(|| a.name.cmp(&b.name))
            })
    });
}

/// Remplit taille, nombre de fichiers, dates et état de verrouillage.
pub fn measure_profile(profile: &mut Profile) {
    let m = fsops::measure_tree(&profile.dir, None);
    profile.size = Some(m.size);
    profile.file_count = Some(m.files);
    profile.mtime = Some(m.mtime);
    profile.locked = crate::sys::processes::profile_locked(&profile.dir);
    let compatibility = profile.dir.join("compatibility.ini");
    profile.last_used = fsops::mtime_of(&compatibility).or(Some(m.mtime));
}

/// Cherche les profils d'une racine donnée ; renvoie vrai si au moins un
/// profil a été trouvé.
pub fn collect_root(app: AppKind, root: &Path, origin: Origin, out: &mut Vec<Profile>) -> bool {
    let before = out.len();
    if !root.is_dir() {
        return false;
    }

    let ini = root.join("profiles.ini");
    let json = root.join("profiles.json");
    if ini.is_file() {
        if let Ok(parsed) = IniFile::read(&ini) {
            profiles_from_ini(app, root, &ini, &parsed, origin.clone(), out);
        }
    } else if json.is_file() {
        if let Ok(text) = std::fs::read_to_string(&json) {
            profiles_from_json(app, root, &json, &text, origin.clone(), out);
        }
    }
    // Dossiers de profil présents mais non déclarés (copie manuelle, profil
    // laissé par une version antérieure, sauvegarde restaurée…).
    profiles_from_dirs(app, root, &ini, origin, out);

    out.len() > before
}

fn profiles_from_ini(
    app: AppKind,
    root: &Path,
    ini_path: &Path,
    ini: &IniFile,
    origin: Origin,
    out: &mut Vec<Profile>,
) {
    let install_default = ini.install_default();
    for section in ini.profile_sections() {
        let Some(path_value) = section.get("Path") else { continue };
        let path_value = path_value.trim();
        if path_value.is_empty() {
            continue;
        }
        if section.get_bool("Locked") == Some(true) {
            continue;
        }
        let is_relative = section.get_bool("IsRelative").unwrap_or(true);
        let dir = if is_relative {
            root.join(path_value)
        } else {
            PathBuf::from(path_value)
        };
        let name = section
            .get("Name")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            });
        let section_is_default = section.get_bool("Default") == Some(true);
        let path_matches_install_default = install_default
            .as_deref()
            .map(|d| d.trim() == path_value || d.trim().ends_with(path_value))
            .unwrap_or(false);
        out.push(Profile {
            app,
            origin: origin.clone(),
            name,
            root: root.to_path_buf(),
            dir,
            ini_path: ini_path.to_path_buf(),
            is_default: section_is_default || path_matches_install_default,
            size: None,
            file_count: None,
            mtime: None,
            last_used: None,
            locked: false,
        });
    }
}

fn profiles_from_json(
    app: AppKind,
    root: &Path,
    json_path: &Path,
    text: &str,
    origin: Origin,
    out: &mut Vec<Profile>,
) {
    for entry in parse_profiles_json(text) {
        let p = PathBuf::from(&entry.path);
        let dir = if p.is_absolute() { p } else { root.join(&entry.path) };
        out.push(Profile {
            app,
            origin: origin.clone(),
            name: if entry.name.is_empty() {
                dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            } else {
                entry.name.clone()
            },
            root: root.to_path_buf(),
            dir,
            ini_path: json_path.to_path_buf(),
            is_default: entry.is_default,
            size: None,
            file_count: None,
            mtime: None,
            last_used: None,
            locked: false,
        });
    }
}

/// Reconnaît les dossiers de profil non déclarés dans un fichier d'index.
fn profiles_from_dirs(
    app: AppKind,
    root: &Path,
    ini_path: &Path,
    origin: Origin,
    out: &mut Vec<Profile>,
) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if out.iter().any(|p| p.dir == path) {
            continue;
        }
        if looks_like_profile(&path) {
            // Profil renommé par une restauration : on le signale clairement,
            // pour ne pas le confondre avec un profil vraiment utilisé.
            let name = if name.contains("-remplace-") {
                format!("{} (ancienne copie)", name)
            } else {
                name
            };
            out.push(Profile {
                app,
                origin: origin.clone(),
                name,
                root: root.to_path_buf(),
                dir: path,
                ini_path: ini_path.to_path_buf(),
                is_default: false,
                size: None,
                file_count: None,
                mtime: None,
                last_used: None,
                locked: false,
            });
        }
    }
}

/// Vrai si le dossier ressemble à un profil Mozilla.
pub fn looks_like_profile(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let mut score = 0;
    for marker in PROFILE_MARKERS {
        if dir.join(marker).exists() {
            score += 1;
            if marker == "prefs.js" {
                score += 2; // fichier emblématique d'un profil
            }
        }
    }
    score >= 2
}

/// Déduit le type d'application d'après un chemin.
pub fn infer_app_from_path(path: &Path) -> Option<AppKind> {
    let s = path.to_string_lossy().to_ascii_lowercase();
    if s.contains("thunderbird") {
        Some(AppKind::Thunderbird)
    } else if s.contains("firefox") || s.contains(".mozilla") {
        Some(AppKind::Firefox)
    } else {
        None
    }
}

/// Déduit l'origine d'après un chemin (Flatpak, Snap, native…).
pub fn infer_origin_from_path(path: &Path) -> Origin {
    let s = path.to_string_lossy().to_string();
    for app_id in ["org.mozilla.firefox", "org.mozilla.Thunderbird", "org.mozilla.Firefox"] {
        if s.contains(&format!("/.var/app/{}", app_id)) || s.contains(&format!("/app/{}", app_id)) {
            return Origin::Flatpak { app_id: app_id.to_string() };
        }
    }
    for snap in ["/snap/firefox/", "/snap/thunderbird/"] {
        if s.contains(snap) {
            return Origin::Snap;
        }
    }
    if s.contains("/.mozilla/") || s.contains("/.thunderbird/") {
        return Origin::Native { distro: osrelease::distro_label() };
    }
    Origin::Other { label: "dossier choisi".to_string() }
}

/// Recherche récursive d'un ou plusieurs profils dans une arborescence
/// (autre disque dur, clé USB, sauvegarde d'un ancien ordinateur…).
pub fn deep_search(root: &Path, apps: &[AppKind], max_depth: usize) -> Vec<Profile> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    let mut inspected = 0u64;
    while let Some((dir, depth)) = stack.pop() {
        inspected += 1;
        if inspected > 400_000 {
            break; // garde-fou : on ne bloque pas le programme indéfiniment
        }
        let ini = dir.join("profiles.ini");
        let json = dir.join("profiles.json");
        let origin = infer_origin_from_path(&dir);
        if ini.is_file() {
            if let Ok(parsed) = IniFile::read(&ini) {
                let app = infer_app_from_path(&dir).unwrap_or(AppKind::Firefox);
                if apps.contains(&app) {
                    profiles_from_ini(app, &dir, &ini, &parsed, origin.clone(), &mut out);
                }
                continue;
            }
        } else if json.is_file() {
            if let Ok(text) = std::fs::read_to_string(&json) {
                let app = infer_app_from_path(&dir).unwrap_or(AppKind::Firefox);
                if apps.contains(&app) {
                    profiles_from_json(app, &dir, &json, &text, origin.clone(), &mut out);
                }
                continue;
            }
        }
        if looks_like_profile(&dir) {
            let app = infer_app_from_path(&dir).unwrap_or(AppKind::Firefox);
            if apps.contains(&app) {
                let name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                out.push(Profile {
                    app,
                    origin: origin.clone(),
                    name,
                    root: dir.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| dir.clone()),
                    dir: dir.clone(),
                    ini_path: ini.clone(),
                    is_default: false,
                    size: None,
                    file_count: None,
                    mtime: None,
                    last_used: None,
                    locked: false,
                });
            }
            continue; // ne pas descendre dans un profil
        }
        if depth >= max_depth {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') && name != ".mozilla" && name != ".thunderbird" && name != ".var" {
                    continue;
                }
                if path.is_dir() && !entry.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
                    stack.push((path, depth + 1));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mrb-scan-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_ini_and_marks_default() {
        let root = temp_dir("ini");
        std::fs::create_dir_all(root.join("abc.default-release")).unwrap();
        std::fs::write(root.join("abc.default-release/prefs.js"), b"// prefs").unwrap();
        std::fs::create_dir_all(root.join("def.perso")).unwrap();
        std::fs::write(root.join("def.perso/prefs.js"), b"// prefs").unwrap();
        std::fs::write(
            root.join("profiles.ini"),
            "[General]\nVersion=2\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath=abc.default-release\n\n[Profile1]\nName=perso\nIsRelative=1\nPath=def.perso\n\n[InstallABC]\nDefault=abc.default-release\n",
        )
        .unwrap();

        let mut out = Vec::new();
        assert!(collect_root(
            AppKind::Firefox,
            &root,
            Origin::Native { distro: "Debian".into() },
            &mut out
        ));
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|p| p.name == "default-release" && p.is_default));
        assert!(out.iter().any(|p| p.name == "perso" && !p.is_default));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_undeclared_profile() {
        let root = temp_dir("sans-ini");
        let prof = root.join("profil-copie");
        std::fs::create_dir_all(&prof).unwrap();
        std::fs::write(prof.join("prefs.js"), b"").unwrap();
        std::fs::write(prof.join("places.sqlite"), b"").unwrap();
        let mut out = Vec::new();
        collect_root(AppKind::Firefox, &root, Origin::Other { label: "test".into() }, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "profil-copie");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn infer_origins() {
        assert!(matches!(
            infer_origin_from_path(Path::new("/home/moi/.var/app/org.mozilla.firefox/.mozilla/firefox")),
            Origin::Flatpak { .. }
        ));
        assert_eq!(
            infer_origin_from_path(Path::new("/home/moi/snap/firefox/common/.mozilla/firefox")),
            Origin::Snap
        );
        assert!(matches!(
            infer_origin_from_path(Path::new("/home/moi/.mozilla/firefox")),
            Origin::Native { .. }
        ));
    }

    #[test]
    fn deep_search_finds_profiles() {
        let root = temp_dir("profond");
        let nested = root.join("media/cle-usb/home/ancien/.mozilla/firefox/p1");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("prefs.js"), b"").unwrap();
        std::fs::write(nested.join("times.json"), b"{}").unwrap();
        let found = deep_search(&root, &[AppKind::Firefox], 10);
        assert_eq!(found.len(), 1);
        assert!(matches!(found[0].origin, Origin::Native { .. }));
        let _ = std::fs::remove_dir_all(&root);
    }
}

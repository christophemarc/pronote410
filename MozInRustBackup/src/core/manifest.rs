//! Manifeste de sauvegarde : le « journal » qui accompagne chaque archive.
//!
//! Ce fichier existe en deux exemplaires :
//!  * **à l'intérieur** de l'archive (`mozinrustbackup.manifeste.txt`) : la
//!    sauvegarde reste donc autosuffisante, même si le journal a été perdu ;
//!  * **à côté** de l'archive, sous forme d'un fichier texte portant le même nom
//!    qu'elle, par exemple `Firefox_11102026_Flatpak.txt`. Il indique en
//!    évidence où restaurer le profil et reste lisible avec un simple éditeur
//!    de texte, sans le logiciel.
//!
//! Le format est volontairement textuel (`clé = valeur`) : il est stable d'une
//! version du logiciel à l'autre et facile à vérifier à la main.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::core::model::{AppKind, Origin};
use crate::util::time;
use crate::{FORMAT_VERSION, MAGIC};

#[derive(Debug, Clone)]
pub struct Manifest {
    pub format_version: u32,
    pub software: String,
    pub software_version: String,
    pub app: AppKind,
    pub origin: Origin,
    pub profile_name: String,
    pub profile_dir_name: String,
    /// Chemin absolu du profil sur la machine d'origine.
    pub profile_path: PathBuf,
    /// Dossier racine de l'application d'origine (contient `profiles.ini`).
    pub profile_root: PathBuf,
    /// Dossier du profil à l'intérieur de l'archive.
    pub archive_root: String,
    pub user_name: String,
    pub user_home: PathBuf,
    pub machine: String,
    pub distro: String,
    pub created_unix: i64,
    pub profile_mtime: Option<i64>,
    pub files: u64,
    pub total_size: u64,
    pub symlinks: u64,
    pub caches_excluded: bool,
    pub ignored: Vec<String>,
    /// Renseignés uniquement dans le journal extérieur.
    pub archive_name: Option<String>,
    pub archive_sha256: Option<String>,
    pub archive_size: Option<u64>,
}

impl Manifest {
    pub fn new(app: AppKind, origin: Origin, now: i64) -> Manifest {
        let home = crate::util::home_dir().unwrap_or_default();
        Manifest {
            format_version: FORMAT_VERSION,
            software: crate::APP_NAME.to_string(),
            software_version: crate::APP_VERSION.to_string(),
            app,
            origin,
            profile_name: String::new(),
            profile_dir_name: String::new(),
            profile_path: PathBuf::new(),
            profile_root: PathBuf::new(),
            archive_root: "profil".to_string(),
            user_name: user_name(),
            user_home: home,
            machine: hostname(),
            distro: crate::sys::osrelease::distro_pretty(),
            created_unix: now,
            profile_mtime: None,
            files: 0,
            total_size: 0,
            symlinks: 0,
            caches_excluded: false,
            ignored: Vec::new(),
            archive_name: None,
            archive_sha256: None,
            archive_size: None,
        }
    }

    /// Contenu textuel (identique pour le journal extérieur et pour le
    /// manifeste embarqué, à l'exception des lignes spécifiques au fichier
    /// d'archive).
    pub fn to_text(&self, tz: i32, external: bool) -> String {
        let mut s = String::new();
        s.push_str("# ─────────────────────────────────────────────────────────────────────\n");
        s.push_str("#  MozInRustBackup — journal de sauvegarde de profil Mozilla\n");
        s.push_str("#\n");
        s.push_str("#  Ce fichier est INDISPENSABLE pour restaurer la sauvegarde au bon\n");
        s.push_str("#  endroit. Conservez-le à côté de l'archive, sans le renommer.\n");
        s.push_str("#  Il peut être ouvert avec n'importe quel éditeur de texte.\n");
        s.push_str("# ─────────────────────────────────────────────────────────────────────\n");
        s.push_str(&format!("magic = {}\n", MAGIC));
        s.push_str(&format!("format_version = {}\n", self.format_version));
        s.push_str(&format!("logiciel = {}\n", self.software));
        s.push_str(&format!("logiciel_version = {}\n", self.software_version));
        s.push_str(&format!("application = {}\n", self.app.label()));
        s.push_str(&format!("type_application = {}\n", self.app.app_dir()));
        s.push_str(&format!("origine = {}\n", self.origin.label()));
        s.push_str(&format!("origine_type = {}\n", self.origin.key()));
        if let Some(d) = self.origin.detail() {
            s.push_str(&format!("origine_detail = {}\n", d));
        }
        s.push_str(&format!("profil_nom = {}\n", self.profile_name));
        s.push_str(&format!("profil_dossier = {}\n", self.profile_dir_name));
        s.push_str(&format!("profil_chemin = {}\n", self.profile_path.display()));
        s.push_str(&format!("profil_racine = {}\n", self.profile_root.display()));
        s.push_str(&format!("dossier_racine_archive = {}\n", self.archive_root));
        s.push_str(&format!("utilisateur = {}\n", self.user_name));
        s.push_str(&format!("dossier_utilisateur = {}\n", self.user_home.display()));
        s.push_str(&format!("machine = {}\n", self.machine));
        s.push_str(&format!("distribution = {}\n", self.distro));
        s.push_str(&format!("date_sauvegarde = {}\n", time::format_iso(self.created_unix, tz)));
        s.push_str(&format!("horodatage = {}\n", self.created_unix));
        if let Some(m) = self.profile_mtime {
            s.push_str(&format!(
                "profil_derniere_modification = {}\n",
                time::format_iso(m, tz)
            ));
            // Double écriture : lisible par un humain, exacte pour le logiciel.
            s.push_str(&format!("profil_derniere_modification_horodatage = {}\n", m));
        }
        s.push_str(&format!("fichiers = {}\n", self.files));
        s.push_str(&format!("taille_totale = {}\n", self.total_size));
        s.push_str(&format!("liens_symboliques = {}\n", self.symlinks));
        s.push_str(&format!(
            "caches_exclus = {}\n",
            if self.caches_excluded { "oui" } else { "non" }
        ));
        s.push_str(&format!("elements_ignores = {}\n", self.ignored.len()));
        for entry in &self.ignored {
            s.push_str(&format!("element_ignore = {}\n", entry));
        }
        if external {
            if let Some(name) = &self.archive_name {
                s.push_str(&format!("archive = {}\n", name));
            }
            if let Some(hash) = &self.archive_sha256 {
                s.push_str(&format!("archive_sha256 = {}\n", hash));
            }
            if let Some(size) = self.archive_size {
                s.push_str(&format!("archive_taille = {}\n", size));
            }
        }
        s
    }

    /// Analyse d'un manifeste (déjà lu depuis l'archive ou depuis le journal).
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        let mut ignored: Vec<String> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(pos) = line.find('=') else { continue };
            let key = line[..pos].trim().to_ascii_lowercase();
            let value = line[pos + 1..].trim().to_string();
            if key == "element_ignore" {
                ignored.push(value);
            } else {
                map.insert(key, value);
            }
        }
        if map.get("magic").map(|m| m.as_str()) != Some(MAGIC) {
            return Err(
                "ce fichier ne contient pas la signature d'une sauvegarde MozInRustBackup".to_string(),
            );
        }
        let format_version: u32 = map
            .get("format_version")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if format_version == 0 {
            return Err("version de format illisible".to_string());
        }
        if format_version > FORMAT_VERSION {
            return Err(format!(
                "cette sauvegarde a été créée par une version plus récente du logiciel \
                 (format {}, alors que ce programme gère le format {})",
                format_version, FORMAT_VERSION
            ));
        }
        let app = map
            .get("type_application")
            .and_then(|v| AppKind::from_key(v))
            .or_else(|| map.get("application").and_then(|v| AppKind::from_key(v)))
            .ok_or_else(|| "type d'application (Firefox / Thunderbird) absent du journal".to_string())?;
        let origin = origin_from_manifest(&map);
        let profile_path = map
            .get("profil_chemin")
            .map(PathBuf::from)
            .ok_or_else(|| "chemin d'origine du profil absent du journal".to_string())?;

        Ok(Manifest {
            format_version,
            software: map.get("logiciel").cloned().unwrap_or_default(),
            software_version: map.get("logiciel_version").cloned().unwrap_or_default(),
            app,
            origin,
            profile_name: map.get("profil_nom").cloned().unwrap_or_default(),
            profile_dir_name: map
                .get("profil_dossier")
                .cloned()
                .unwrap_or_else(|| {
                    profile_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                }),
            profile_path,
            profile_root: map.get("profil_racine").map(PathBuf::from).unwrap_or_default(),
            archive_root: map
                .get("dossier_racine_archive")
                .cloned()
                .unwrap_or_else(|| "profil".to_string()),
            user_name: map.get("utilisateur").cloned().unwrap_or_default(),
            user_home: map.get("dossier_utilisateur").map(PathBuf::from).unwrap_or_default(),
            machine: map.get("machine").cloned().unwrap_or_default(),
            distro: map.get("distribution").cloned().unwrap_or_default(),
            created_unix: map.get("horodatage").and_then(|v| v.parse().ok()).unwrap_or(0),
            profile_mtime: map
                .get("profil_derniere_modification_horodatage")
                .and_then(|v| v.parse().ok())
                .or_else(|| map.get("profil_derniere_modification").and_then(|v| parse_datetime(v))),
            files: map.get("fichiers").and_then(|v| v.parse().ok()).unwrap_or(0),
            total_size: map.get("taille_totale").and_then(|v| v.parse().ok()).unwrap_or(0),
            symlinks: map.get("liens_symboliques").and_then(|v| v.parse().ok()).unwrap_or(0),
            caches_excluded: map.get("caches_exclus").map(|v| v == "oui").unwrap_or(false),
            ignored,
            archive_name: map.get("archive").cloned(),
            archive_sha256: map.get("archive_sha256").cloned(),
            archive_size: map.get("archive_taille").and_then(|v| v.parse().ok()),
        })
    }
}

fn origin_from_manifest(map: &BTreeMap<String, String>) -> Origin {
    let kind = map.get("origine_type").map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let detail = map.get("origine_detail").cloned().unwrap_or_default();
    match kind.as_str() {
        "flatpak" => Origin::Flatpak {
            app_id: if detail.is_empty() { "org.mozilla.firefox".into() } else { detail },
        },
        "snap" => Origin::Snap,
        "native" | "natif" => Origin::Native {
            distro: if detail.is_empty() { "Linux".into() } else { detail },
        },
        _ => Origin::Other {
            label: map.get("origine").cloned().unwrap_or_else(|| "Autre".to_string()),
        },
    }
}

fn parse_datetime(text: &str) -> Option<i64> {
    // Formats acceptés : « 2026-10-05 14:32:07 » ou un horodatage brut.
    if let Ok(v) = text.trim().parse::<i64>() {
        return Some(v);
    }
    let t = text.trim();
    if t.len() < 10 {
        return None;
    }
    let year: i32 = t[0..4].parse().ok()?;
    let month: u32 = t[5..7].parse().ok()?;
    let day: u32 = t[8..10].parse().ok()?;
    let (hour, minute, second) = if t.len() >= 19 {
        (
            t[11..13].parse().unwrap_or(0),
            t[14..16].parse().unwrap_or(0),
            t[17..19].parse().unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };
    Some(time::days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

fn user_name() -> String {
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            return user;
        }
    }
    if let Ok(user) = std::env::var("LOGNAME") {
        if !user.is_empty() {
            return user;
        }
    }
    crate::util::home_dir()
        .and_then(|h| h.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default()
}

fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_manifest() {
        let mut m = Manifest::new(AppKind::Firefox, Origin::Flatpak { app_id: "org.mozilla.firefox".into() }, 1_791_000_000);
        m.profile_name = "default-release".into();
        m.profile_dir_name = "abc123.default-release".into();
        m.profile_path = PathBuf::from("/home/alice/.var/app/org.mozilla.firefox/.mozilla/firefox/abc123.default-release");
        m.profile_root = PathBuf::from("/home/alice/.var/app/org.mozilla.firefox/.mozilla/firefox");
        m.user_home = PathBuf::from("/home/alice");
        m.user_name = "alice".into();
        m.files = 42;
        m.total_size = 123_456;
        m.profile_mtime = Some(1_790_999_000);
        m.ignored.push("profil/parent.lock (verrou)".to_string());

        let text = m.to_text(7200, true);
        assert!(text.contains("profil_chemin = /home/alice"));
        let parsed = Manifest::parse(&text).expect("manifeste relu");
        assert_eq!(parsed.app, AppKind::Firefox);
        assert_eq!(parsed.origin, Origin::Flatpak { app_id: "org.mozilla.firefox".into() });
        assert_eq!(parsed.files, 42);
        assert_eq!(parsed.total_size, 123_456);
        assert_eq!(parsed.profile_mtime, Some(1_790_999_000), "horodatage exact conservé");
        assert!(text.contains("profil_derniere_modification = "), "date lisible présente");
        assert_eq!(parsed.ignored.len(), 1);
    }

    #[test]
    fn rejects_foreign_file() {
        assert!(Manifest::parse("bonjour = monde").is_err());
        let mut m = Manifest::new(AppKind::Thunderbird, Origin::Snap, 0);
        m.format_version = FORMAT_VERSION + 5;
        let text = m.to_text(0, false);
        assert!(Manifest::parse(&text).is_err(), "format futur refusé");
    }
}

//! Modèle de données : applications, origines d'installation, profils.

use std::path::{Path, PathBuf};

/// Application Mozilla concernée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppKind {
    Firefox,
    Thunderbird,
}

impl AppKind {
    pub fn label(self) -> &'static str {
        match self {
            AppKind::Firefox => "Firefox",
            AppKind::Thunderbird => "Thunderbird",
        }
    }

    /// Nom du dossier racine (celui qui contient `profiles.ini`).
    pub fn app_dir(self) -> &'static str {
        match self {
            AppKind::Firefox => "firefox",
            AppKind::Thunderbird => "thunderbird",
        }
    }

    /// Nom du dossier « racine historique » (`~/.mozilla` ou `~/.thunderbird`).
    pub fn dot_dir(self) -> &'static str {
        match self {
            AppKind::Firefox => ".mozilla",
            AppKind::Thunderbird => ".thunderbird",
        }
    }

    /// Chemin du dossier qui contient `profiles.ini`, relatif au dossier
    /// personnel (ou au dossier `common` d'un paquet Snap, ou à la racine d'un
    /// paquet Flatpak).
    ///
    /// * Firefox : `~/.mozilla/firefox`
    /// * Thunderbird : `~/.thunderbird` (il n'y a pas de sous-dossier)
    pub fn relative_profile_root(self) -> &'static str {
        match self {
            AppKind::Firefox => ".mozilla/firefox",
            AppKind::Thunderbird => ".thunderbird",
        }
    }

    pub fn from_key(s: &str) -> Option<AppKind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "firefox" => Some(AppKind::Firefox),
            "thunderbird" => Some(AppKind::Thunderbird),
            _ => None,
        }
    }

    pub fn default_roots(self) -> [&'static str; 3] {
        match self {
            AppKind::Firefox => [
                "org.mozilla.firefox",
                "org.mozilla.firefox.BaseApp",
                "org.mozilla.Firefox",
            ],
            AppKind::Thunderbird => [
                "org.mozilla.Thunderbird",
                "org.mozilla.thunderbird",
                "org.mozilla.Thunderbird.BaseApp",
            ],
        }
    }
}

/// Provenance de l'installation, qui détermine l'emplacement du profil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Installation native de la distribution (« natif Debian », « natif Arch »…).
    Native { distro: String },
    /// Installation Flatpak (identifiant de l'application).
    Flatpak { app_id: String },
    /// Installation Snap.
    Snap,
    /// Autre emplacement (dossier choisi à la main, restauration…).
    Other { label: String },
}

impl Origin {
    /// Libellé affiché à l'utilisateur.
    pub fn label(&self) -> String {
        match self {
            Origin::Native { distro } => format!("natif {}", distro),
            Origin::Flatpak { .. } => "Flatpak".to_string(),
            Origin::Snap => "Snap".to_string(),
            Origin::Other { label } => label.clone(),
        }
    }

    /// Forme courte utilisée dans les noms de fichiers :
    /// `Flatpak`, `Snap`, `natif-Debian`, `natif-Arch`…
    pub fn slug(&self) -> String {
        match self {
            Origin::Native { distro } => format!("natif-{}", distro.replace(' ', "-")),
            Origin::Flatpak { .. } => "Flatpak".to_string(),
            Origin::Snap => "Snap".to_string(),
            Origin::Other { label } => {
                let cleaned: String = label
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
                    .collect();
                if cleaned.is_empty() {
                    "Autre".to_string()
                } else {
                    cleaned
                }
            }
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Origin::Native { .. } => "native",
            Origin::Flatpak { .. } => "flatpak",
            Origin::Snap => "snap",
            Origin::Other { .. } => "other",
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Origin::Native { distro } => Some(distro.clone()),
            Origin::Flatpak { app_id } => Some(app_id.clone()),
            Origin::Snap => Some("snap".to_string()),
            Origin::Other { label } => Some(label.clone()),
        }
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Un profil détecté sur le disque.
#[derive(Debug, Clone)]
pub struct Profile {
    pub app: AppKind,
    pub origin: Origin,
    /// Nom du profil tel qu'il apparaît dans `profiles.ini` (« default-release »).
    pub name: String,
    /// Dossier racine de l'application (contient `profiles.ini`).
    pub root: PathBuf,
    /// Dossier du profil.
    pub dir: PathBuf,
    /// Chemin du fichier `profiles.ini` (ou `profiles.json`) d'origine.
    pub ini_path: PathBuf,
    /// Profil utilisé par défaut au lancement de l'application.
    pub is_default: bool,
    /// Taille en octets (remplie lors de l'analyse).
    pub size: Option<u64>,
    /// Nombre de fichiers (remplie lors de l'analyse).
    pub file_count: Option<u64>,
    /// Dernière modification (la plus récente de l'arborescence).
    pub mtime: Option<i64>,
    /// Dernière utilisation connue (fichier `compatibility.ini`).
    pub last_used: Option<i64>,
    /// Vrai si un verrou indique que le profil est ouvert dans l'application.
    pub locked: bool,
}

impl Profile {
    pub fn dir_name(&self) -> String {
        self.dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    pub fn dir_string(&self) -> String {
        self.dir.to_string_lossy().to_string()
    }

    /// Titre lisible : « Firefox — default-release (Flatpak) ».
    pub fn display_name(&self) -> String {
        format!("{} — {} ({})", self.app.label(), self.name, self.origin.label())
    }

    /// Base du nom de sauvegarde : `Firefox_11102026_Flatpak`.
    pub fn backup_stem(&self, date_suffix: &str) -> String {
        format!(
            "{}_{}_{}",
            self.app.label(),
            date_suffix,
            crate::util::sanitize_component(&self.origin.slug())
        )
    }
}

/// Message d'information sur une racine inspectée.
#[derive(Debug, Clone)]
pub struct ScannedRoot {
    pub path: PathBuf,
    pub origin: Origin,
    pub app: AppKind,
    pub found: bool,
}

/// Empêche les avertissements « champ jamais lu » pour des usages futurs.
impl ScannedRoot {
    pub fn describe(&self) -> String {
        format!(
            "{} ({}) : {}",
            self.path.display(),
            self.origin.label(),
            if self.found { "profil trouvé" } else { "aucun profil" }
        )
    }
}

/// Normalise un chemin pour comparaison (sans résolution de liens).
pub fn same_path(a: &Path, b: &Path) -> bool {
    a == b
}

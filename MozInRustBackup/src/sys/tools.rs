//! Outils externes : boîtes de dialogue, ouverture de dossiers, installation.

use std::path::PathBuf;
use std::process::Command;

use super::fsops::which;
use super::osrelease;

/// Moteur de boîtes de dialogue disponible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogTool {
    Zenity,
    Yad,
    KDialog,
}

impl DialogTool {
    pub fn binary(self) -> &'static str {
        match self {
            DialogTool::Zenity => "zenity",
            DialogTool::Yad => "yad",
            DialogTool::KDialog => "kdialog",
        }
    }

    pub fn friendly_name(self) -> &'static str {
        match self {
            DialogTool::Zenity => "zenity (boîtes de dialogue GNOME/GTK)",
            DialogTool::Yad => "yad (boîtes de dialogue GTK)",
            DialogTool::KDialog => "kdialog (boîtes de dialogue KDE)",
        }
    }
}

/// Vrai si une session graphique est disponible.
pub fn graphical_session() -> bool {
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Détecte le meilleur moteur de dialogue installé.
pub fn detect_dialog_tool() -> Option<(DialogTool, PathBuf)> {
    if let Ok(forced) = std::env::var("MRB_UI") {
        let f = forced.to_ascii_lowercase();
        let tool = match f.as_str() {
            "zenity" => Some(DialogTool::Zenity),
            "yad" => Some(DialogTool::Yad),
            "kdialog" => Some(DialogTool::KDialog),
            _ => None,
        };
        if let Some(t) = tool {
            if let Some(path) = which(t.binary()) {
                return Some((t, path));
            }
        }
    }
    for tool in [DialogTool::Zenity, DialogTool::Yad, DialogTool::KDialog] {
        if let Some(path) = which(tool.binary()) {
            return Some((tool, path));
        }
    }
    None
}

/// Nom du paquet à installer pour obtenir les boîtes de dialogue.
pub fn dialog_package() -> Option<&'static str> {
    if osrelease::is_family("arch") {
        Some("zenity")
    } else if osrelease::is_family("fedora") || osrelease::is_family("rhel") {
        Some("zenity")
    } else if osrelease::is_family("suse") {
        Some("zenity")
    } else if osrelease::is_family("debian") {
        Some("zenity")
    } else if osrelease::is_family("alpine") {
        Some("zenity")
    } else if osrelease::is_family("void") {
        Some("zenity")
    } else if osrelease::is_family("gentoo") {
        Some("gnome-extra/zenity")
    } else if osrelease::distro_id() == "nixos" {
        Some("nixpkgs.zenity")
    } else if osrelease::distro_id() == "solus" {
        Some("zenity")
    } else if osrelease::is_family("slackware") {
        None
    } else {
        Some("zenity")
    }
}

/// Commande d'installation correspondante (gestionnaire de paquets + droits).
pub fn install_command(package: &str) -> Option<(String, Vec<String>)> {
    let needs_root = true;
    let (prog, args): (String, Vec<String>) = if osrelease::is_family("arch") {
        ("pacman".into(), vec!["-S".into(), "--needed".into(), "--noconfirm".into(), package.into()])
    } else if osrelease::is_family("fedora") || osrelease::is_family("rhel") {
        ("dnf".into(), vec!["install".into(), "-y".into(), package.into()])
    } else if osrelease::is_family("suse") {
        ("zypper".into(), vec!["--non-interactive".into(), "install".into(), package.into()])
    } else if osrelease::is_family("debian") {
        ("apt-get".into(), vec!["install".into(), "-y".into(), package.into()])
    } else if osrelease::is_family("alpine") {
        ("apk".into(), vec!["add".into(), package.into()])
    } else if osrelease::is_family("void") {
        ("xbps-install".into(), vec!["-y".into(), package.into()])
    } else if osrelease::is_family("gentoo") {
        ("emerge".into(), vec![package.into()])
    } else if osrelease::distro_id() == "nixos" {
        ("nix-env".into(), vec!["-iA".into(), package.into()])
    } else if osrelease::distro_id() == "solus" {
        ("eopkg".into(), vec!["install".into(), "-y".into(), package.into()])
    } else {
        return None;
    };
    if needs_root && !super::fsops::is_root() && which("sudo").is_some() {
        let mut full = vec![prog];
        full.extend(args);
        return Some(("sudo".into(), full));
    }
    Some((prog, args))
}

/// Ouvre un dossier dans le gestionnaire de fichiers de l'utilisateur.
pub fn open_directory(path: &std::path::Path) -> bool {
    if which("xdg-open").is_none() {
        return false;
    }
    Command::new("xdg-open")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

/// Vérifie la présence d'une commande (utilisé par l'installateur).
pub fn has_command(name: &str) -> bool {
    which(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_ne_crashes_pas() {
        let _ = detect_dialog_tool();
        let _ = dialog_package();
        let _ = graphical_session();
    }
}

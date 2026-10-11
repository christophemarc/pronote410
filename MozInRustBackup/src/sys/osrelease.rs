//! Identification de la distribution Linux, à partir de `/etc/os-release`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

fn parse_os_release() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for candidate in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Ok(text) = std::fs::read_to_string(candidate) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(eq) = line.find('=') {
                    let key = line[..eq].trim().to_string();
                    let mut value = line[eq + 1..].trim().to_string();
                    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                        value = value[1..value.len() - 1].to_string();
                    }
                    map.insert(key, value);
                }
            }
            break;
        }
    }
    map
}

fn shorten(id: &str) -> Option<&'static str> {
    Some(match id {
        "debian" => "Debian",
        "ubuntu" => "Ubuntu",
        "linuxmint" | "mint" => "Mint",
        "lmde" => "LMDE",
        "zorin" => "Zorin",
        "pop" => "Pop!_OS",
        "elementary" => "elementary",
        "kali" => "Kali",
        "raspbian" => "Raspbian",
        "devuan" => "Devuan",
        "arch" => "Arch",
        "cachyos" => "CachyOS",
        "manjaro" | "manjaro-arm" => "Manjaro",
        "endeavouros" => "EndeavourOS",
        "garuda" => "Garuda",
        "artix" => "Artix",
        "arcolinux" => "ArcoLinux",
        "omarchy" => "Omarchy",
        "fedora" | "fedora-asahi-remix" => "Fedora",
        "rhel" => "RHEL",
        "centos" => "CentOS",
        "rocky" => "Rocky",
        "almalinux" => "AlmaLinux",
        "oracle" => "Oracle Linux",
        "nobara" => "Nobara",
        "bazzite" => "Bazzite",
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" | "sles" => "openSUSE",
        "nixos" => "NixOS",
        "gentoo" | "funtoo" => "Gentoo",
        "alpine" => "Alpine",
        "void" => "Void",
        "solus" => "Solus",
        "slackware" | "salix" => "Slackware",
        "mageia" => "Mageia",
        "openmandriva" | "mandriva" => "Mandriva",
        "clear-linux-os" => "Clear Linux",
        "deepin" => "Deepin",
        "mx" => "MX Linux",
        "antix" => "antiX",
        "pclinuxos" => "PCLinuxOS",
        "pureos" => "PureOS",
        "endless" => "Endless OS",
        "steamos" => "SteamOS",
        "chimera" => "Chimera",
        _ => return None,
    })
}

struct DistroInfo {
    id: String,
    label: String,
    pretty: String,
    family: String,
    like: Vec<String>,
}

fn info() -> &'static DistroInfo {
    static INFO: OnceLock<DistroInfo> = OnceLock::new();
    INFO.get_or_init(|| {
        let map = parse_os_release();
        let id = map.get("ID").cloned().unwrap_or_else(|| "inconnue".to_string());
        let like: Vec<String> = map
            .get("ID_LIKE")
            .map(|v| v.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let label = shorten(&id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| capitalize(&id));
        // Famille = premier ID_LIKE connu, sinon l'identifiant lui-même.
        let mut family = id.clone();
        for candidate in itertools_like(&like) {
            if shorten(&candidate).is_some() || is_known_family(&candidate) {
                family = candidate;
                break;
            }
        }
        let pretty = map
            .get("PRETTY_NAME")
            .cloned()
            .unwrap_or_else(|| label.clone());
        DistroInfo { id, label, pretty, family, like }
    })
}

fn itertools_like(like: &[String]) -> Vec<String> {
    like.to_vec()
}

fn is_known_family(id: &str) -> bool {
    matches!(
        id,
        "debian" | "arch" | "fedora" | "suse" | "rhel" | "gentoo" | "alpine" | "void"
    )
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => "Linux".to_string(),
    }
}

/// Nom court et lisible : « Debian », « Arch », « Mint », « CachyOS »…
pub fn distro_label() -> String {
    info().label.clone()
}

/// Identifiant brut (`ID` de os-release) : « debian », « cachyos »…
pub fn distro_id() -> String {
    info().id.clone()
}

/// Nom complet : « Debian GNU/Linux 12 (bookworm) ».
pub fn distro_pretty() -> String {
    info().pretty.clone()
}

/// Famille technique, utilisée pour choisir le gestionnaire de paquets.
pub fn distro_family() -> String {
    info().family.clone()
}

/// Vraie si la distribution descend de la famille donnée.
pub fn is_family(name: &str) -> bool {
    let i = info();
    i.family == name || i.id == name || i.like.iter().any(|l| l == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_something() {
        let label = distro_label();
        assert!(!label.is_empty());
        let id = distro_id();
        assert!(!id.is_empty());
    }
}

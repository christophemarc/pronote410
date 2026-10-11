//! Lecture des fichiers `profiles.ini` (format INI) et `profiles.json`
//! (anciennes versions des paquets Snap/Flatpak).

use std::collections::BTreeMap;
use std::path::Path;

/// Section INI : liste ordonnée de paires clé/valeur.
#[derive(Debug, Clone, Default)]
pub struct Section {
    pub name: String,
    pub values: Vec<(String, String)>,
}

impl Section {
    pub fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_ascii_lowercase();
        self.values
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true"
        })
    }
}

/// Contenu d'un `profiles.ini`.
#[derive(Debug, Clone, Default)]
pub struct IniFile {
    pub sections: Vec<Section>,
}

impl IniFile {
    pub fn parse(text: &str) -> IniFile {
        let mut sections: Vec<Section> = Vec::new();
        let mut current = Section { name: String::new(), values: Vec::new() };
        for raw in text.lines() {
            let line = raw.trim_end_matches(['\r', '\n']);
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if !current.name.is_empty() || !current.values.is_empty() {
                    sections.push(current);
                }
                current = Section {
                    name: trimmed[1..trimmed.len() - 1].to_string(),
                    values: Vec::new(),
                };
                continue;
            }
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_string();
                let value = trimmed[pos + 1..].trim().to_string();
                current.values.push((key, value));
            }
        }
        if !current.name.is_empty() || !current.values.is_empty() {
            sections.push(current);
        }
        IniFile { sections }
    }

    pub fn read(path: &Path) -> std::io::Result<IniFile> {
        let text = std::fs::read_to_string(path)?;
        Ok(IniFile::parse(&text))
    }

    /// Sections `[ProfileN]`.
    pub fn profile_sections(&self) -> Vec<&Section> {
        self.sections
            .iter()
            .filter(|s| s.name.to_ascii_lowercase().starts_with("profile"))
            .collect()
    }

    /// Le profil par défaut d'après les sections `[InstallXXXX]`.
    pub fn install_default(&self) -> Option<String> {
        for s in &self.sections {
            if s.name.to_ascii_lowercase().starts_with("install") {
                if let Some(v) = s.get("Default") {
                    let v = v.trim();
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
        None
    }

    /// Tableau associatif des sections (pratique pour les tests).
    pub fn as_map(&self) -> BTreeMap<String, Vec<(String, String)>> {
        let mut m = BTreeMap::new();
        for s in &self.sections {
            m.insert(s.name.clone(), s.values.clone());
        }
        m
    }
}

/// Entrée extraite d'un `profiles.json`.
#[derive(Debug, Clone)]
pub struct JsonProfile {
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

/// Extraction minimaliste des profils déclarés dans un `profiles.json`.
///
/// Le fichier ressemble à :
/// ```json
/// { "profiles": [ { "name": "default", "path": "abcd.default", "default": true } ],
///   "default": "abcd.default" }
/// ```
pub fn parse_profiles_json(text: &str) -> Vec<JsonProfile> {
    // L'analyseur JSON complet vit dans `core::json` ; on s'en sert ici pour
    // rester robuste (espaces, ordre des clés, valeurs absentes).
    let value = match crate::core::json::parse(text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    let default_path = value
        .get("default")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    if let Some(profiles) = value.get("profiles").and_then(|v| v.as_array()) {
        for p in profiles {
            let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let path = p.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if path.is_empty() {
                continue;
            }
            let is_default = p.get("default").and_then(|v| v.as_bool()).unwrap_or(false)
                || (!default_path.is_empty() && path == default_path);
            out.push(JsonProfile { name, path, is_default });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = "\
[General]
StartWithLastProfile=1
Version=2

[Profile0]
Name=default
IsRelative=1
Path=abc123.default
Default=1

[Profile1]
Name=perso
IsRelative=1
Path=def456.perso

[Install4F96D1932A9F858E]
Default=def456.perso
Locked=1
";

    #[test]
    fn parse_ini() {
        let ini = IniFile::parse(INI);
        let profiles = ini.profile_sections();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].get("Name"), Some("default"));
        assert_eq!(profiles[0].get_bool("Default"), Some(true));
        assert_eq!(profiles[1].get("Path"), Some("def456.perso"));
        assert_eq!(ini.install_default().as_deref(), Some("def456.perso"));
    }

    #[test]
    fn parse_json_profiles() {
        let text = r#"{
          "default": "abcd.default",
          "profiles": [
             {"name": "default", "path": "abcd.default", "isRelative": true},
             {"name": "autre",  "path": "/home/alice/profil-absolu"}
          ]
        }"#;
        let p = parse_profiles_json(text);
        assert_eq!(p.len(), 2);
        assert!(p[0].is_default);
        assert_eq!(p[1].path, "/home/alice/profil-absolu");
    }
}

//! Fonctions utilitaires : dates, tailles lisibles, noms de fichiers,
//! encapsulation shell.

use std::path::{Path, PathBuf};

pub mod time;

/// Taille lisible à la française : « 1,4 Go », « 812 Ko », « 17 octets ».
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["octets", "Ko", "Mo", "Go", "To", "Po"];
    if bytes < 1000 {
        return format!("{} octets", bytes);
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    let txt = if value < 10.0 {
        format!("{:.2}", value)
    } else if value < 100.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.0}", value)
    };
    let txt = txt.replace('.', ",");
    format!("{} {}", txt, UNITS[unit])
}

/// Séparateur de milliers à la française : 12 345.
pub fn human_number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(*b as char);
    }
    out
}

/// Rend une chaîne utilisable comme nom de fichier (sans séparateur de chemin).
pub fn sanitize_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            out.push('-');
        } else if c == ' ' {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    out.trim_matches('.').to_string()
}

/// Dossier personnel de l'utilisateur.
pub fn home_dir() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    None
}

/// Décalage horaire local par rapport à UTC, en secondes (obtenu une seule fois).
///
/// ZIP stocke les dates en heure locale ; ce décalage sert uniquement à
/// l'affichage et aux horodatages DOS (l'horodatage exact en UTC est toujours
/// conservé dans les champs supplémentaires de l'archive).
pub fn tz_offset_secs() -> i32 {
    use std::sync::OnceLock;
    static OFFSET: OnceLock<i32> = OnceLock::new();
    *OFFSET.get_or_init(|| {
        if let Ok(tz) = std::env::var("MRB_TZ_OFFSET") {
            if let Ok(v) = tz.trim().parse::<i32>() {
                return v;
            }
        }
        if let Ok(out) = std::process::Command::new("date").arg("+%z").output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                let s = s.trim();
                // Format : +0200 / -0530 / +02:00
                let clean: String = s.chars().filter(|c| *c != ':').collect();
                if clean.len() >= 5 {
                    let sign = if clean.starts_with('-') { -1 } else { 1 };
                    let h: i32 = clean[1..3].parse().unwrap_or(0);
                    let m: i32 = clean[3..5].parse().unwrap_or(0);
                    return sign * (h * 3600 + m * 60);
                }
            }
        }
        0
    })
}

/// Renvoie un chemin unique dans `dir` en ajoutant « -2 », « -3 »… si besoin.
pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let candidate = if ext.is_empty() {
        dir.join(stem)
    } else {
        dir.join(format!("{}.{}", stem, ext))
    };
    if !candidate.exists() {
        return candidate;
    }
    for n in 2..9999 {
        let candidate = if ext.is_empty() {
            dir.join(format!("{}-{}", stem, n))
        } else {
            dir.join(format!("{}-{}.{}", stem, n, ext))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(human_size(0), "0 octets");
        assert_eq!(human_size(999), "999 octets");
        assert_eq!(human_size(1000), "1,00 Ko");
        assert_eq!(human_size(1_500_000), "1,50 Mo");
        assert_eq!(human_size(2_500_000_000), "2,50 Go");
    }

    #[test]
    fn numbers() {
        assert_eq!(human_number(1), "1");
        assert_eq!(human_number(1234), "1\u{202f}234");
    }

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_component("Firefox natif/Debian"), "Firefox_natif-Debian");
    }
}

//! Calculs de dates civiles (calendrier grégorien) sans dépendance externe.

/// Secondes depuis l'époque Unix.
pub fn now_unix() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}

/// Convertit un nombre de secondes Unix en (année, mois, jour, heure, minute, seconde).
pub fn civil_from_unix(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (y, m, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32)
}

/// Algorithme de Howard Hinnant : jours depuis 1970-01-01 -> (année, mois, jour).
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// Jours depuis 1970-01-01 à partir d'une date civile.
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

/// Date/heure locale lisible : « 05/10/2026 14:32 ».
pub fn format_datetime(secs: i64, tz_offset: i32) -> String {
    let (y, m, d, h, mi, _s) = civil_from_unix(secs + tz_offset as i64);
    format!("{:02}/{:02}/{:04} {:02}:{:02}", d, m, y, h, mi)
}

/// Date locale seule : « 05/10/2026 ».
pub fn format_date(secs: i64, tz_offset: i32) -> String {
    let (y, m, d, _, _, _) = civil_from_unix(secs + tz_offset as i64);
    format!("{:02}/{:02}/{:04}", d, m, y)
}

/// Suffixe JJMMAAAA utilisé dans les noms de sauvegarde.
pub fn date_suffix(secs: i64, tz_offset: i32) -> String {
    let (y, m, d, _, _, _) = civil_from_unix(secs + tz_offset as i64);
    format!("{:02}{:02}{:04}", d, m, y)
}

/// Horodatage complet pour les journaux : « 2026-10-05 14:32:07 ».
pub fn format_iso(secs: i64, tz_offset: i32) -> String {
    let (y, m, d, h, mi, s) = civil_from_unix(secs + tz_offset as i64);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
}

/// Durée lisible : « 1 min 12 s », « 8 s ».
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{} s", secs)
    } else if secs < 3600 {
        format!("{} min {:02} s", secs / 60, secs % 60)
    } else {
        format!("{} h {:02} min", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(civil_from_unix(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil_from_unix(1_000_000_000), (2001, 9, 9, 1, 46, 40));
        assert_eq!(civil_from_unix(1_700_000_000), (2023, 11, 14, 22, 13, 20));
    }

    #[test]
    fn roundtrip_days() {
        for days in [-100_000i64, -1, 0, 1, 19_000, 100_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }

    #[test]
    fn formatting() {
        // 2026-10-05 12:00:00 UTC
        let t = days_from_civil(2026, 10, 5) * 86_400 + 12 * 3600;
        assert_eq!(date_suffix(t, 0), "05102026");
        assert_eq!(format_date(t, 0), "05/10/2026");
        assert_eq!(format_date(t, 7200), "05/10/2026");
        assert_eq!(format_datetime(t, 0), "05/10/2026 12:00");
    }
}

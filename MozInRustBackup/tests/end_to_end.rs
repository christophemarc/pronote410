//! Test de bout en bout : création d'un profil Firefox complet, sauvegarde,
//! contrôle d'intégrité, restauration, comparaison octet à octet.
//!
//! C'est le test qui valide la promesse centrale du logiciel : « le fichier
//! compressé est une copie conforme du profil complet ».

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use mozinrustbackup::core::backup::{create_backup, BackupError, BackupOptions, SilentProgress};
use mozinrustbackup::core::model::{AppKind, Origin, Profile};
use mozinrustbackup::core::restore::{
    self, inspect_archive, restore, verify_archive, Overwrite, RestoreOptions,
};
use mozinrustbackup::core::scanner;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mrb-e2e-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Crée un profil réaliste et renvoie son chemin.
fn make_profile(base: &Path) -> PathBuf {
    let profile = base.join("home/alice/.mozilla/firefox/ab12cd34.default-release");
    std::fs::create_dir_all(profile.join("cache2/entries")).unwrap();
    std::fs::create_dir_all(profile.join("storage/default/https+++exemple.fr")).unwrap();
    std::fs::create_dir_all(profile.join("dossier vide")).unwrap();
    std::fs::create_dir_all(profile.join("ImapMail/exemple.fr")).unwrap();

    std::fs::write(profile.join("prefs.js"), b"user_pref(\"browser.startup.homepage\", \"https://exemple.fr\");\n").unwrap();
    std::fs::write(profile.join("places.sqlite"), vec![0xA5u8; 200_000]).unwrap();
    std::fs::write(profile.join("key4.db"), b"donnees sensibles").unwrap();
    std::fs::write(profile.join("storage/default/https+++exemple.fr/idb.sqlite"), b"indexeddb").unwrap();
    std::fs::write(profile.join("cache2/entries/AAAA"), vec![0u8; 50_000]).unwrap();
    std::fs::write(profile.join("journal du jour.txt"), "fichier avec espaces".as_bytes()).unwrap();
    std::fs::write(profile.join("téléchargements—accents.txt"), "accents: éàüç").unwrap();
    std::fs::write(profile.join("photo.jpg"), vec![0x42u8; 30_000]).unwrap();

    // Fichier texte volumineux et répétitif (la compression doit gagner).
    let big: Vec<u8> = b"<message>Bonjour, ceci est un message de test.</message>\n"
        .iter()
        .cloned()
        .cycle()
        .take(1_000_000)
        .collect();
    std::fs::write(profile.join("ImapMail/exemple.fr/INBOX"), &big).unwrap();

    // Verrou présent (doit être ignoré par la sauvegarde).
    std::fs::write(profile.join("parent.lock"), b"").unwrap();
    let _ = std::os::unix::fs::symlink("key4.db", profile.join("lien-vers-key4"));

    // Droits et dates particuliers.
    std::fs::set_permissions(profile.join("prefs.js"), std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(profile.join("places.sqlite"), std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(profile.join("journal du jour.txt"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_650_000_000);
    let times = std::fs::FileTimes::new().set_modified(stamp).set_accessed(stamp);
    for name in ["prefs.js", "places.sqlite", "journal du jour.txt", "photo.jpg"] {
        let f = std::fs::OpenOptions::new().write(true).open(profile.join(name)).unwrap();
        f.set_times(times).unwrap();
    }
    profile
}

/// Photographie récursive : chemin relatif -> (contenu, mode, date, type).
fn snapshot(root: &Path) -> BTreeMap<String, SnapshotEntry> {
    let mut map = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().to_string();
            let md = std::fs::symlink_metadata(&path).unwrap();
            if md.file_type().is_symlink() {
                map.insert(
                    rel,
                    SnapshotEntry::Link(std::fs::read_link(&path).unwrap().to_string_lossy().to_string()),
                );
            } else if md.is_dir() {
                map.insert(rel, SnapshotEntry::Dir(md.permissions().mode() & 0o7777));
                stack.push(path);
            } else {
                let content = std::fs::read(&path).unwrap();
                map.insert(
                    rel,
                    SnapshotEntry::File {
                        len: content.len() as u64,
                        crc: mozinrustbackup::archive::crc32::crc32(&content),
                        mode: md.permissions().mode() & 0o7777,
                        mtime: md.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                    },
                );
            }
        }
    }
    map
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum SnapshotEntry {
    Dir(u32),
    File { len: u64, crc: u32, mode: u32, mtime: u64 },
    Link(String),
}

fn profile_of(dir: &Path, root: &Path) -> Profile {
    Profile {
        app: AppKind::Firefox,
        origin: Origin::Native { distro: "Debian".into() },
        name: "default-release".into(),
        root: root.to_path_buf(),
        dir: dir.to_path_buf(),
        ini_path: root.join("profiles.ini"),
        is_default: true,
        size: None,
        file_count: None,
        mtime: None,
        last_used: None,
        locked: false,
    }
}

#[test]
fn backup_then_restore_is_identical() {
    let base = temp_dir("cycle");
    let profile_dir = make_profile(&base);
    let root = profile_dir.parent().unwrap().to_path_buf();
    // profiles.ini pour que la détection retrouve le profil.
    std::fs::write(
        root.join("profiles.ini"),
        format!(
            "[General]\nVersion=2\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath={}\nDefault=1\n",
            profile_dir.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();

    let profile = profile_of(&profile_dir, &root);
    let before = snapshot(&profile_dir);

    // --- sauvegarde -------------------------------------------------------
    let dest = base.join("cle-usb/Sauvegardes");
    let archive = dest.join("Firefox_11102026_natif-Debian.mozinrustbackup");
    let options = BackupOptions { exclude_caches: false, level: mozinrustbackup::archive::deflate::Level::Normal, verify_after: true };
    let outcome = create_backup(&profile, &archive, &options, 7200, &mut SilentProgress).expect("sauvegarde");
    assert!(outcome.archive.is_file());
    assert!(outcome.log.is_file(), "journal .txt créé à côté de l'archive");
    assert_eq!(outcome.log.file_name().unwrap(), "Firefox_11102026_natif-Debian.txt");
    assert!(outcome.archive_bytes > 0);
    assert!(outcome.archive_bytes < outcome.source_bytes, "la compression doit réduire la taille");
    assert!(outcome.ignored.iter().any(|i| i.contains("parent.lock")), "verrou ignoré : {:?}", outcome.ignored);

    let log_text = std::fs::read_to_string(&outcome.log).unwrap();
    assert!(log_text.contains("profil_chemin = "), "le journal indique l'emplacement d'origine");
    assert!(log_text.contains(&profile_dir.display().to_string()));
    assert!(log_text.contains("COMMENT RESTAURER CETTE SAUVEGARDE"));

    // --- inspection -------------------------------------------------------
    let info = inspect_archive(&archive).expect("archive légitime");
    assert_eq!(info.manifest.app, AppKind::Firefox);
    assert_eq!(info.manifest.profile_path, profile_dir);
    // Le verrou n'est pas sauvegardé : on le retire du décompte attendu.
    let expected_files = before
        .iter()
        .filter(|(name, e)| matches!(e, SnapshotEntry::File { .. }) && name.as_str() != "parent.lock")
        .count() as u64;
    assert_eq!(info.manifest.files, expected_files);
    assert!(info.warning.is_none());
    assert!(restore::describe(&info, 7200).contains("Emplacement de restauration"));

    // --- vérification d'intégrité complète --------------------------------
    let (count, bytes) = verify_archive(&archive, &mut |_, _| {}).expect("archive intègre");
    assert!(count > 5 && bytes > 1_000_000);

    // --- restauration -----------------------------------------------------
    // 1) Un profil « ancien » occupe déjà la place : sans autorisation,
    //    la restauration doit refuser d'écraser quoi que ce soit.
    let target = base.join("restauration/ab12cd34.default-release");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("ancien.txt"), b"ancien contenu").unwrap();
    let err = restore(&info, &target, &RestoreOptions::default(), &mut SilentProgress).unwrap_err();
    assert!(matches!(err, restore::RestoreError::TargetExists(_)));
    assert!(target.join("ancien.txt").is_file(), "rien n'est touché sans confirmation");

    // 2) Après confirmation, l'ancien profil est renommé (jamais effacé).
    let opts = RestoreOptions { overwrite: Overwrite::Rename, verify_hashes: true, restore_owner: false };
    let out = restore(&info, &target, &opts, &mut SilentProgress).expect("restauration");
    let renamed = out.renamed_existing.expect("l'ancien profil est renommé");
    assert!(renamed.join("ancien.txt").is_file(), "aucune donnée n'est perdue");
    assert!(renamed.file_name().unwrap().to_string_lossy().contains("-remplace-"));
    assert!(out.files >= 9);
    assert_eq!(out.symlinks, 1);
    assert_eq!(out.verified, out.files, "toutes les empreintes SHA-256 sont vérifiées");

    // --- comparaison ------------------------------------------------------
    let after = snapshot(&target);
    let mut after_filtered = after.clone();
    // Le verrou n'a pas été sauvegardé : il ne doit pas réapparaître.
    assert!(!after_filtered.contains_key("parent.lock"));
    after_filtered.remove("parent.lock");
    let mut before_filtered = before.clone();
    before_filtered.remove("parent.lock");
    assert_eq!(
        before_filtered, after_filtered,
        "le profil restauré doit être identique au profil d'origine \
         (contenu, droits, dates, liens symboliques)"
    );

    // --- détection du profil restauré ------------------------------------
    let report = scanner::scan(
        &scanner::ScanOptions {
            apps: vec![AppKind::Firefox],
            extra_roots: vec![target.parent().unwrap().to_path_buf()],
            deep_search: true,
            measure: true,
        },
        &mut |_| {},
    );
    assert!(
        report.profiles.iter().any(|p| p.dir == target),
        "le scanner doit retrouver le profil restauré : {:?}",
        report.profiles.iter().map(|p| p.dir.clone()).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn corrupted_archive_is_rejected() {
    let base = temp_dir("corruption");
    let profile_dir = make_profile(&base);
    let root = profile_dir.parent().unwrap().to_path_buf();
    let profile = profile_of(&profile_dir, &root);
    let archive = base.join("Firefox_11102026_natif-Debian.mozinrustbackup");
    create_backup(&profile, &archive, &BackupOptions::default(), 0, &mut SilentProgress).unwrap();

    // Corruption d'un octet au milieu des données du profil.
    let mut data = std::fs::read(&archive).unwrap();
    let pos = data.len() / 2;
    data[pos] ^= 0xFF;
    std::fs::write(&archive, &data).unwrap();

    let result = verify_archive(&archive, &mut |_, _| {});
    assert!(result.is_err(), "une archive corrompue doit être détectée");

    // Un fichier quelconque n'est pas accepté comme sauvegarde.
    let alien = base.join("quelque-chose.mozinrustbackup");
    std::fs::write(&alien, b"ceci n'est pas une archive").unwrap();
    assert!(inspect_archive(&alien).is_err());

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn excludes_caches_on_request() {
    let base = temp_dir("caches");
    let profile_dir = make_profile(&base);
    let root = profile_dir.parent().unwrap().to_path_buf();
    let profile = profile_of(&profile_dir, &root);
    let archive = base.join("sans-caches.mozinrustbackup");
    let options = BackupOptions { exclude_caches: true, ..Default::default() };
    let outcome = create_backup(&profile, &archive, &options, 0, &mut SilentProgress).unwrap();
    assert!(outcome.ignored.iter().any(|i| i.contains("cache2")));

    let info = inspect_archive(&archive).unwrap();
    assert!(info.manifest.caches_excluded);
    let log = std::fs::read_to_string(&outcome.log).unwrap();
    assert!(log.contains("caches_exclus = oui"));

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn refuses_to_backup_missing_profile() {
    let base = temp_dir("absent");
    let profile = profile_of(&base.join("inexistant"), &base);
    let err = create_backup(
        &profile,
        &base.join("x.mozinrustbackup"),
        &BackupOptions::default(),
        0,
        &mut SilentProgress,
    )
    .unwrap_err();
    assert!(matches!(err, BackupError::Message(_)));
    let _ = std::fs::remove_dir_all(&base);
}

//! Test du parcours complet de l'assistant, sans intervention humaine :
//! les réponses aux boîtes de dialogue sont fournies à l'avance.
//!
//! Ce test vérifie l'enchaînement des écrans (menu → profils → destination →
//! type de sauvegarde → résumé) et l'effet réel sur le disque.

use std::path::PathBuf;

use mozinrustbackup::core::restore::{inspect_archive, verify_archive};
use mozinrustbackup::ui::scripted::ScriptedDialogs;
use mozinrustbackup::ui::wizard::Wizard;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mrb-wizard-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn prepare_home(base: &PathBuf) -> PathBuf {
    let profile = base.join(".mozilla/firefox/abc.default-release");
    std::fs::create_dir_all(profile.join("storage/default")).unwrap();
    std::fs::write(profile.join("prefs.js"), b"user_pref(\"x\", 1);\n").unwrap();
    std::fs::write(profile.join("places.sqlite"), vec![7u8; 5000]).unwrap();
    std::fs::write(profile.join("storage/default/cache.sqlite"), b"donnees").unwrap();
    std::fs::write(profile.join("parent.lock"), b"").unwrap();
    std::fs::write(
        base.join(".mozilla/firefox/profiles.ini"),
        "[General]\nVersion=2\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath=abc.default-release\nDefault=1\n",
    )
    .unwrap();
    profile
}

#[test]
fn parcours_complet_de_l_assistant() {
    // Un dossier personnel isolé : l'assistant ne touche à rien d'autre.
    let base = temp_dir("flux");
    let home = base.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let profile_dir = prepare_home(&home);
    let usb = base.join("cle-usb");
    std::fs::create_dir_all(&usb).unwrap();
    std::env::set_var("HOME", &home);
    std::env::set_var("USER", "testeur");

    // --- 1. Sauvegarde ---------------------------------------------------
    let mut ui = ScriptedDialogs::new()
        .with_choice("sauver")
        .with_choice("0") // premier profil de la liste
        .with_choice("autre") // choisir un autre dossier…
        .with_dir(usb.clone())
        .with_choice("complet") // copie conforme
        .with_choice("ok") // détail des éléments ignorés (fichier de verrou)
        .with_choice("fin") // et maintenant ?
        .with_confirm(false) // pas d'autre profil
        .with_choice("quitter");
    {
        let mut wizard = Wizard::new(&mut ui);
        wizard.run().expect("parcours de sauvegarde");
    }

    let archives: Vec<PathBuf> = std::fs::read_dir(&usb)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "mozinrustbackup").unwrap_or(false))
        .collect();
    assert_eq!(archives.len(), 1, "une archive créée : {:?}", archives);
    let archive = archives[0].clone();
    let name = archive.file_name().unwrap().to_string_lossy().to_string();
    assert!(name.starts_with("Firefox_"), "nommage « Type_Date_Origine » : {}", name);
    assert!(name.ends_with(".mozinrustbackup"), "{}", name);
    assert!(
        name.contains("natif-"),
        "l'origine est indiquée dans le nom du fichier : {}",
        name
    );

    let log = archive.with_extension("txt");
    assert!(log.is_file(), "journal .txt à côté de l'archive");
    let log_text = std::fs::read_to_string(&log).unwrap();
    assert!(log_text.contains(&profile_dir.display().to_string()));
    assert!(log_text.contains("archive_sha256 = "));
    assert!(log_text.contains("COMMENT RESTAURER"));

    let info = inspect_archive(&archive).expect("archive lisible");
    assert_eq!(info.manifest.profile_path, profile_dir);
    assert!(info.manifest.files >= 3);
    verify_archive(&archive, &mut |_, _| {}).expect("archive intègre");
    assert!(ui.log_contains("Sauvegarde réussie"), "résumé affiché");
    assert!(ui.log_contains("parent.lock"), "le verrou est signalé comme ignoré");

    // --- 2. Restauration -------------------------------------------------
    let mut ui2 = ScriptedDialogs::new()
        .with_choice("restaurer")
        .with_file(archive.clone())
        .with_choice("propose") // emplacement indiqué par le journal
        .with_choice("renommer") // l'ancien profil est conservé
        .with_choice("fin")
        .with_confirm(false)
        .with_choice("quitter");
    {
        let mut wizard = Wizard::new(&mut ui2);
        wizard.run().expect("parcours de restauration");
    }
    assert!(ui2.log_contains("Restauration réussie"));
    assert!(
        std::fs::read(profile_dir.join("places.sqlite")).unwrap() == vec![7u8; 5000],
        "les données sont revenues à leur place"
    );
    assert!(
        !profile_dir.join("parent.lock").exists(),
        "le verrou n'est pas restauré (sinon l'application refuserait de démarrer)"
    );
    let copies: Vec<String> = std::fs::read_dir(profile_dir.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("-remplace-"))
        .collect();
    assert_eq!(copies.len(), 1, "l'ancien profil est conservé : {:?}", copies);

    // --- 3. Vérification -------------------------------------------------
    let mut ui3 = ScriptedDialogs::new()
        .with_choice("verifier")
        .with_file(archive.clone())
        .with_choice("quitter");
    {
        let mut wizard = Wizard::new(&mut ui3);
        wizard.run().expect("parcours de vérification");
    }
    assert!(ui3.log_contains("Sauvegarde valide"));

    // --- 4. Liste des profils --------------------------------------------
    let mut ui4 = ScriptedDialogs::new().with_choice("lister").with_choice("quitter");
    {
        let mut wizard = Wizard::new(&mut ui4);
        wizard.run().expect("parcours de liste");
    }
    assert!(ui4.log_contains("Profils détectés"));

    let _ = std::fs::remove_dir_all(&base);
}

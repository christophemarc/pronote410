//! L'assistant pas à pas, à la manière de MozBackup.
//!
//! Le déroulement est identique quelle que soit l'interface : les boîtes de
//! dialogue sont fournies par [`Dialogues`] (zenity, kdialog ou mode texte).

use std::path::{Path, PathBuf};

use crate::core::backup::{self, BackupOptions};
use crate::core::model::{AppKind, Profile};
use crate::core::restore::{self, Overwrite, RestoreOptions};
use crate::core::scanner::{self, ScanOptions};
use crate::sys::{disk, fsops, processes, tools};
use crate::ui::{Dialogues, Item, UiError};
use crate::util::{human_number, human_size, time};
use crate::{APP_NAME, APP_VERSION, BACKUP_EXTENSION};

/// Petit fichier de configuration (dernier dossier utilisé…).
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub last_destination: Option<PathBuf>,
    pub last_archive: Option<PathBuf>,
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        let home = crate::util::home_dir()?;
        Some(home.join(".config/mozinrustbackup/config.ini"))
    }

    pub fn load() -> Config {
        let mut config = Config::default();
        if let Some(path) = Config::path() {
            if let Ok(text) = std::fs::read_to_string(path) {
                for line in text.lines() {
                    if let Some(eq) = line.find('=') {
                        let key = line[..eq].trim();
                        let value = line[eq + 1..].trim();
                        if value.is_empty() {
                            continue;
                        }
                        match key {
                            "dernier_dossier" => config.last_destination = Some(PathBuf::from(value)),
                            "derniere_archive" => config.last_archive = Some(PathBuf::from(value)),
                            _ => {}
                        }
                    }
                }
            }
        }
        config
    }

    pub fn save(&self) {
        let Some(path) = Config::path() else { return };
        if let Some(parent) = path.parent() {
            let _ = fsops::ensure_dir(parent);
        }
        let mut text = String::new();
        if let Some(d) = &self.last_destination {
            text.push_str(&format!("dernier_dossier={}\n", d.display()));
        }
        if let Some(a) = &self.last_archive {
            text.push_str(&format!("derniere_archive={}\n", a.display()));
        }
        let _ = std::fs::write(path, text);
    }
}

/// Adaptateur de progression vers les boîtes de dialogue.
struct DialogProgress<'a, D: Dialogues> {
    ui: &'a mut D,
    total: u64,
    done: u64,
    last_refresh: std::time::Instant,
    cancelled: bool,
}

impl<'a, D: Dialogues> DialogProgress<'a, D> {
    fn new(ui: &'a mut D, _title: &str) -> Self {
        DialogProgress {
            ui,
            total: 0,
            done: 0,
            last_refresh: std::time::Instant::now() - std::time::Duration::from_secs(1),
            cancelled: false,
        }
    }
}

impl<'a, D: Dialogues> backup::Progress for DialogProgress<'a, D> {
    fn phase(&mut self, label: &str) {
        self.refresh(true, label);
    }

    fn total(&mut self, bytes: u64, files: u64) {
        self.total = bytes;
        self.refresh(true, &format!("{} fichier(s) à traiter", human_number(files)));
    }

    fn advance(&mut self, bytes: u64, file: &str) {
        self.done += bytes;
        self.refresh(false, file);
    }

    fn cancelled(&self) -> bool {
        self.cancelled
    }
}

impl<'a, D: Dialogues> DialogProgress<'a, D> {
    fn refresh(&mut self, force: bool, label: &str) {
        // On limite la fréquence des mises à jour : au maximum 5 par seconde.
        if !force && self.last_refresh.elapsed() < std::time::Duration::from_millis(200) {
            return;
        }
        self.last_refresh = std::time::Instant::now();
        let fraction = if self.total == 0 {
            0.0
        } else {
            (self.done as f64 / self.total as f64) as f32
        };
        let shown = if self.total == 0 {
            label.to_string()
        } else {
            format!(
                "{} sur {} — {}",
                human_size(self.done),
                human_size(self.total),
                label
            )
        };
        self.ui.progress(fraction, &shown);
        if self.ui.progress_cancelled() {
            self.cancelled = true;
        }
    }

    #[allow(dead_code)]
    fn finish(&mut self) {
        self.ui.progress(1.0, "Terminé");
        self.ui.progress_end();
    }
}

pub struct Wizard<'a, D: Dialogues> {
    ui: &'a mut D,
    tz: i32,
    config: Config,
}

impl<'a, D: Dialogues> Wizard<'a, D> {
    pub fn new(ui: &'a mut D) -> Self {
        Wizard { ui, tz: crate::util::tz_offset_secs(), config: Config::load() }
    }

    /// Boucle principale de l'assistant.
    pub fn run(&mut self) -> Result<(), UiError> {
        loop {
            let items = vec![
                Item::new("sauver", vec!["1".into(), "Sauvegarder un profil Firefox ou Thunderbird".into()]),
                Item::new("restaurer", vec!["2".into(), "Restaurer une sauvegarde".into()]),
                Item::new("verifier", vec!["3".into(), "Vérifier une sauvegarde".into()]),
                Item::new("lister", vec!["4".into(), "Afficher les profils détectés sur ce disque".into()]),
                Item::new("quitter", vec!["5".into(), "Quitter".into()]),
            ];
            let text = format!(
                "{} {} — assistant de sauvegarde et de restauration\n\
                 des profils Firefox et Thunderbird sous Linux.\n\n\
                 Que voulez-vous faire ?",
                APP_NAME, APP_VERSION
            );
            let choice = match self
                .ui
                .choose("MozInRustBackup", &text, &["N°", "Action"], &items)
            {
                Ok(Some(c)) => c,
                Ok(None) => return Ok(()),
                Err(UiError::Cancelled) => return Ok(()),
                Err(e) => return Err(e),
            };
            let result = match choice.as_str() {
                "sauver" => self.backup_flow(),
                "restaurer" => self.restore_flow(),
                "verifier" => self.verify_flow(),
                "lister" => self.list_flow(),
                _ => return Ok(()),
            };
            if let Err(UiError::Cancelled) = result {
                continue; // retour au menu principal
            }
            result?;
        }
    }

    // ------------------------------------------------------------------
    // Détection des profils
    // ------------------------------------------------------------------

    fn scan(&mut self, deep: bool, extra: Vec<PathBuf>) -> Result<Vec<Profile>, UiError> {
        self.ui.progress_begin(
            "MozInRustBackup",
            "Recherche des profils Firefox et Thunderbird…",
        )?;
        let options = ScanOptions {
            apps: vec![AppKind::Firefox, AppKind::Thunderbird],
            extra_roots: extra,
            deep_search: deep,
            measure: true,
        };
        let ui = &mut *self.ui;
        let mut step = 0.0f32;
        let report = scanner::scan(options_clone(&options), &mut |label: &str| {
            step = (step + 0.06).min(0.95);
            ui.progress(step, label);
        });
        let cancelled = ui.progress_cancelled();
        ui.progress(1.0, "Terminé");
        ui.progress_end();
        if cancelled || crate::sys::signals::interrupted() {
            return Err(UiError::Cancelled);
        }
        Ok(report.profiles)
    }

    fn pick_profiles(&mut self, profiles: &[Profile]) -> Result<Option<Vec<Profile>>, UiError> {
        let mut items: Vec<Item> = Vec::new();
        for (i, p) in profiles.iter().enumerate() {
            let size = p.size.map(human_size).unwrap_or_else(|| "?".into());
            let date = p.mtime.map(|m| time::format_datetime(m, self.tz)).unwrap_or_else(|| "?".into());
            let state = if p.locked { "🔒 en cours d'utilisation" } else { "" };
            items.push(Item::new(
                format!("{}", i),
                vec![
                    if p.is_default { "★".into() } else { " ".into() },
                    format!("{} {}", p.app.label(), p.origin.label()),
                    p.name.clone(),
                    size,
                    date,
                    state.to_string(),
                ],
            ));
        }
        // Proposition « tout sauvegarder », en fin de liste.
        items.push(Item::new(
            "__ALL__",
            vec![
                "🍀".into(),
                "Tous les profils".into(),
                format!("{} profils", profiles.len()),
                String::new(),
                String::new(),
                String::new(),
            ],
        ));
        let text = format!(
            "Profils trouvés sur ce disque : {}\n\
             ★ = profil par défaut. Sélectionnez le profil à sauvegarder :",
            profiles.len()
        );
        let columns = ["Déf.", "Type et origine", "Profil", "Taille", "Dernière modification", "État"];
        let choice = self.ui.choose("Profils détectés", &text, &columns, &items)?;
        let Some(choice) = choice else { return Ok(None) };
        if choice == "__ALL__" {
            return Ok(Some(profiles.to_vec()));
        }
        match choice.parse::<usize>() {
            Ok(index) if index < profiles.len() => Ok(Some(vec![profiles[index].clone()])),
            _ => Ok(None),
        }
    }

    fn backup_flow(&mut self) -> Result<(), UiError> {
        let profiles = self.scan(false, Vec::new())?;
        if profiles.is_empty() {
            return self.no_profile_found();
        }
        let Some(selection) = self.pick_profiles(&profiles)? else { return Ok(()) };
        for profile in selection {
            self.backup_one(&profile)?;
        }
        if self.ui.confirm(
            "Sauvegarde terminée",
            "Voulez-vous sauvegarder un autre profil ?",
            "Oui, un autre profil",
            "Non, terminer",
        )? {
            return self.backup_flow();
        }
        Ok(())
    }

    fn no_profile_found(&mut self) -> Result<(), UiError> {
        let items = vec![
            Item::new("disque", vec!["1".into(), "Chercher sur un autre disque ou une clé USB…".into()]),
            Item::new("dossier", vec!["2".into(), "Indiquer moi-même le dossier du profil…".into()]),
            Item::new("aide", vec!["3".into(), "Pourquoi aucun profil n'est trouvé ?".into()]),
            Item::new("retour", vec!["4".into(), "Revenir au menu principal".into()]),
        ];
        let text = format!(
            "Aucun profil Firefox ou Thunderbird n'a été trouvé sur ce disque.\n\n\
             Emplacements inspectés :\n\
              • {}\n\
              • {}\n\
              • {} (Flatpak)\n\
              • {} (Snap)\n\n\
             Un profil n'existe qu'après avoir lancé au moins une fois l'application.",
            "~/.mozilla/firefox",
            "~/.thunderbird",
            "~/.var/app/org.mozilla.firefox/…",
            "~/snap/firefox/common/…"
        );
        let choice = self.ui.choose("Aucun profil trouvé", &text, &["N°", "Action"], &items)?;
        match choice.as_deref() {
            Some("disque") => {
                let start = disk::removable_mounts()
                    .first()
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from("/media"));
                let Some(dir) = self.ui.choose_dir("Choisir le disque à analyser", &start)? else {
                    return Ok(());
                };
                let profiles = self.scan(true, vec![dir])?;
                if profiles.is_empty() {
                    self.ui.info(
                        "Aucun profil trouvé",
                        "Aucun profil n'a été trouvé sur ce support.\n\n\
                         Vérifiez que le disque est bien monté, puis réessayez.",
                    )?;
                    return Ok(());
                }
                if let Some(selection) = self.pick_profiles(&profiles)? {
                    for profile in selection {
                        self.backup_one(&profile)?;
                    }
                }
                Ok(())
            }
            Some("dossier") => {
                let start = crate::util::home_dir().unwrap_or_else(|| PathBuf::from("/"));
                let Some(dir) = self.ui.choose_dir("Choisir le dossier du profil", &start)? else {
                    return Ok(());
                };
                if !scanner::looks_like_profile(&dir) {
                    let ok = self.ui.confirm(
                        "Dossier inhabituel",
                        &format!(
                            "Le dossier\n{}\nne contient pas les fichiers habituels d'un profil.\n\n\
                             Voulez-vous l'utiliser quand même ?",
                            dir.display()
                        ),
                        "Utiliser ce dossier",
                        "Choisir un autre dossier",
                    )?;
                    if !ok {
                        return Ok(());
                    }
                }
                let app = if dir.to_string_lossy().to_ascii_lowercase().contains("thunderbird") {
                    AppKind::Thunderbird
                } else {
                    AppKind::Firefox
                };
                let mut profile = Profile {
                    app,
                    origin: scanner::infer_origin_from_path(&dir),
                    name: dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "profil".to_string()),
                    root: dir.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| dir.clone()),
                    dir: dir.clone(),
                    ini_path: dir.join("profiles.ini"),
                    is_default: false,
                    size: None,
                    file_count: None,
                    mtime: None,
                    last_used: None,
                    locked: false,
                };
                scanner::measure_profile(&mut profile);
                self.backup_one(&profile)
            }
            Some("aide") => {
                self.ui.text_page(
                    "Pourquoi aucun profil n'est trouvé ?",
                    "MozInRustBackup cherche les profils dans les emplacements utilisés par \
                     Firefox et Thunderbird :\n\n\
                      • installation classique : ~/.mozilla/firefox et ~/.thunderbird\n\
                      • Flatpak : ~/.var/app/org.mozilla.firefox/…\n\
                      • Snap : ~/snap/firefox/common/…\n\n\
                     Si l'application n'a jamais été lancée, aucun profil n'existe encore : \
                     il suffit d'ouvrir Firefox ou Thunderbird une première fois.\n\n\
                     Si le profil se trouve sur un autre disque (ancien ordinateur, clé USB, \
                     disque externe), utilisez l'option « Chercher sur un autre disque ».",
                )?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Vérifie qu'aucune instance ne tourne, en proposant de la fermer.
    fn ensure_application_closed(&mut self, app: AppKind) -> Result<bool, UiError> {
        let running = processes::running_for(app);
        if running.is_empty() {
            return Ok(true);
        }
        let noms = running
            .iter()
            .map(|r| format!("{} (PID {})", r.name, r.pid))
            .collect::<Vec<_>>()
            .join(", ");
        let ok = self.ui.confirm(
            &format!("{} est en cours d'exécution", app.label()),
            &format!(
                "{} est actuellement ouvert :\n{}\n\n\
                 Un profil utilisé pendant la sauvegarde peut être incohérent.\n\n\
                 Voulez-vous fermer {} maintenant ?\n\
                 (MozInRustBackup lui demande de se fermer proprement, comme si vous \
                 cliquiez sur « Quitter ».)",
                app.label(),
                noms,
                app.label()
            ),
            &format!("Fermer {}", app.label()),
            "Ne pas fermer, annuler",
        )?;
        if !ok {
            return Ok(false);
        }
        self.ui.progress_begin("Fermeture…", &format!("Fermeture de {}…", app.label()))?;
        let stuck = processes::close_application(app, 20_000);
        self.ui.progress_end();
        if !stuck.is_empty() {
            self.ui.error(
                "Fermeture impossible",
                &format!(
                    "{} n'a pas pu être fermé (PID {}).\n\n\
                     Fermez-le vous-même puis relancez la sauvegarde.",
                    app.label(),
                    stuck
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            return Ok(false);
        }
        // Petit délai de sécurité : Firefox termine l'écriture de ses fichiers.
        std::thread::sleep(std::time::Duration::from_millis(600));
        Ok(true)
    }

    fn choose_destination(&mut self, profile: &Profile) -> Result<Option<PathBuf>, UiError> {
        let mut items: Vec<Item> = Vec::new();
        let usb = disk::removable_mounts();
        for (i, mount) in usb.iter().enumerate() {
            let free = disk::free_space(mount)
                .map(human_size)
                .unwrap_or_else(|| "?".into());
            items.push(Item::new(
                format!("usb:{}", i),
                vec!["Clé USB / disque externe".into(), mount.display().to_string(), free],
            ));
        }
        if let Some(last) = self.config.last_destination.clone() {
            if last.is_dir() {
                items.push(Item::new(
                    "last",
                    vec!["Dernier dossier utilisé".into(), last.display().to_string(), String::new()],
                ));
            }
        }
        let suggested = disk::default_backup_dir();
        items.push(Item::new(
            "defaut",
            vec![
                "Dossier conseillé".into(),
                suggested.display().to_string(),
                disk::free_space(suggested.parent().unwrap_or(&suggested))
                    .map(human_size)
                    .unwrap_or_default(),
            ],
        ));
        items.push(Item::new(
            "autre",
            vec!["Choisir un autre dossier…".into(), String::new(), String::new()],
        ));

        let text = format!(
            "Sauvegarde de : {} — {}\n\n\
             Où faut-il enregistrer la sauvegarde ?\n\
             (une clé USB est le support le plus sûr contre une panne de disque)",
            profile.display_name(),
            human_size(profile.size.unwrap_or(0))
        );
        let choice = self.ui.choose(
            "Dossier de sauvegarde",
            &text,
            &["Destination", "Dossier", "Espace libre"],
            &items,
        )?;
        match choice.as_deref() {
            Some(c) if c.starts_with("usb:") => {
                let index: usize = c[4..].parse().unwrap_or(0);
                Ok(usb.get(index).cloned())
            }
            Some("last") => Ok(self.config.last_destination.clone()),
            Some("defaut") => {
                fsops::ensure_dir(&suggested).ok();
                Ok(Some(suggested))
            }
            Some("autre") => {
                let start = self
                    .config
                    .last_destination
                    .clone()
                    .or_else(crate::util::home_dir)
                    .unwrap_or_else(|| PathBuf::from("/"));
                self.ui.choose_dir("Choisir le dossier de sauvegarde", &start)
            }
            _ => Ok(None),
        }
    }

    fn backup_one(&mut self, profile: &Profile) -> Result<(), UiError> {
        // 1. L'application doit être fermée.
        if !self.ensure_application_closed(profile.app)? {
            return Ok(());
        }

        // 2. Dossier de destination.
        let Some(dest_dir) = self.choose_destination(profile)? else { return Ok(()) };
        if let Err(e) = fsops::ensure_dir(&dest_dir) {
            self.ui.error(
                "Dossier inaccessible",
                &format!("Impossible de créer ou d'ouvrir :\n{}\n\n{}", dest_dir.display(), e),
            );
            return Ok(());
        }
        if !fsops::is_writable_dir(&dest_dir) {
            self.ui.error(
                "Écriture impossible",
                &format!(
                    "Le dossier n'est pas accessible en écriture :\n{}\n\n\
                     Vérifiez qu'une clé USB n'est pas protégée en écriture.",
                    dest_dir.display()
                ),
            );
            return Ok(());
        }

        // 3. Type de sauvegarde.
        let options_items = vec![
            Item::new(
                "complet",
                vec![
                    "1".into(),
                    "Sauvegarde complète — copie conforme du profil, caches inclus (recommandé)".into(),
                ],
            ),
            Item::new(
                "rapide",
                vec![
                    "2".into(),
                    "Sauvegarde allégée — sans les caches (plus rapide, plus légère)".into(),
                ],
            ),
        ];
        let choice = self.ui.choose(
            "Type de sauvegarde",
            "Quel contenu faut-il sauvegarder ?",
            &["N°", "Type"],
            &options_items,
        )?;
        let Some(kind) = choice else { return Ok(()) };
        let exclude_caches = kind == "rapide";

        // 4. Nom de l'archive (et confirmation d'écrasement).
        let stem = profile.backup_stem(&time::date_suffix(time::now_unix(), self.tz));
        let mut archive = dest_dir.join(format!("{}.{}", stem, BACKUP_EXTENSION));
        if archive.exists() {
            let items = vec![
                Item::new("remplacer", vec!["1".into(), "Remplacer la sauvegarde existante".into()]),
                Item::new("numeroter", vec!["2".into(), "Créer une sauvegarde supplémentaire (suffixe -2)".into()]),
                Item::new("annuler", vec!["3".into(), "Annuler".into()]),
            ];
            let text = format!(
                "Une sauvegarde portant le même nom existe déjà :\n{}\n\n{}",
                archive.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                if let Ok(md) = std::fs::metadata(&archive) {
                    format!(
                        "Elle date du {} et pèse {}.\n(elle concerne le même type de profil et la même origine)",
                        time::format_datetime(
                            md.modified()
                                .ok()
                                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0),
                            self.tz
                        ),
                        human_size(md.len())
                    )
                } else {
                    String::new()
                }
            );
            let choice = self.ui.choose("Sauvegarde déjà présente", &text, &["N°", "Action"], &items)?;
            match choice.as_deref() {
                Some("remplacer") => {}
                Some("numeroter") => {
                    archive = crate::util::unique_path(
                        &dest_dir,
                        &stem,
                        BACKUP_EXTENSION,
                    );
                }
                _ => return Ok(()),
            }
        }

        // 5. Espace disque suffisant ?
        let source_bytes = profile.size.unwrap_or(0);
        if let Some(warning) = backup::check_free_space(source_bytes, &dest_dir) {
            let ok = self.ui.confirm(
                "Espace disque",
                &format!(
                    "{}\n\nSouhaitez-vous choisir un autre dossier ?",
                    warning
                ),
                "Changer de dossier",
                "Continuer quand même",
            )?;
            if ok {
                return self.backup_one(&profile);
            }
        }

        // 6. Sauvegarde proprement dite.
        let backup_options = BackupOptions {
            exclude_caches,
            level: crate::archive::deflate::Level::Normal,
            verify_after: true,
        };
        crate::sys::signals::reset();
        self.ui.progress_begin(
            "Sauvegarde en cours…",
            &format!("Copie du profil {}", profile.name),
        )?;
        let result = {
            let mut progress = DialogProgress::new(self.ui, "Sauvegarde");
            backup::create_backup(profile, &archive, &backup_options, self.tz, &mut progress)
        };
        self.ui.progress_end();

        match result {
            Ok(outcome) => {
                self.config.last_destination = Some(dest_dir.clone());
                self.config.save();
                let log_name = outcome
                    .log
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let archive_name = outcome
                    .archive
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let ratio = if outcome.archive_bytes > 0 {
                    outcome.source_bytes as f64 / outcome.archive_bytes as f64
                } else {
                    1.0
                };
                let text = format!(
                    "Sauvegarde de « {} » terminée.\n\n\
                     Fichier créé :\n{}\n{}/\n\n\
                     Profil sauvegardé : {} — {} ({})\n\
                     Fichiers copiés : {}   •   Liens symboliques : {}\n\
                     Taille d'origine : {}   •   Archive : {} (compression ×{:.1})\n\
                     Durée : {}\n\n\
                     Empreinte SHA-256 :\n{}\n\n\
                     Journal : {}\n\
                     Ce journal indique l'emplacement exact du profil d'origine :\n\
                     conservez-le à côté de l'archive, il est indispensable pour restaurer.",
                    profile.display_name(),
                    archive_name,
                    dest_dir.display(),
                    profile.app.label(),
                    profile.name,
                    profile.origin.label(),
                    human_number(outcome.files),
                    human_number(outcome.symlinks),
                    human_size(outcome.source_bytes),
                    human_size(outcome.archive_bytes),
                    ratio,
                    time::format_duration(outcome.duration_secs),
                    outcome.archive_sha256,
                    log_name
                );
                self.ui.info("Sauvegarde réussie", &text)?;
                if !outcome.ignored.is_empty() {
                    let mut detail = String::from(
                        "Éléments volontairement non copiés (sans conséquence pour la restauration) :\n\n",
                    );
                    for line in outcome.ignored.iter().take(30) {
                        detail.push_str(&format!("  • {}\n", line));
                    }
                    if outcome.ignored.len() > 30 {
                        detail.push_str(&format!("  … et {} autres\n", outcome.ignored.len() - 30));
                    }
                    match self.ui.choose(
                        "Détails de la sauvegarde",
                        &detail,
                        &["Action"],
                        &[Item::new("ok", vec!["Fermer".into()])],
                    ) {
                        Ok(_) | Err(UiError::Cancelled) => {}
                        Err(e) => return Err(e),
                    }
                }
                let items = vec![
                    Item::new("ouvrir", vec!["1".into(), "Ouvrir le dossier de la sauvegarde".into()]),
                    Item::new("fin", vec!["2".into(), "Terminer".into()]),
                ];
                if let Ok(Some(c)) = self
                    .ui
                    .choose("Et maintenant ?", "", &["N°", "Action"], &items)
                {
                    if c == "ouvrir" {
                        tools::open_directory(&dest_dir);
                    }
                }
                Ok(())
            }
            Err(backup::BackupError::Cancelled) => {
                self.ui.info(
                    "Sauvegarde annulée",
                    "La sauvegarde a été annulée. Aucun fichier incomplet n'a été conservé.",
                )?;
                Ok(())
            }
            Err(e) => {
                self.ui.error(
                    "Échec de la sauvegarde",
                    &format!("{}\n\nAucun fichier incomplet n'a été conservé.", e),
                );
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Restauration
    // ------------------------------------------------------------------

    fn restore_flow(&mut self) -> Result<(), UiError> {
        loop {
            let start = self
                .config
                .last_archive
                .as_ref()
                .and_then(|a| a.parent().map(|p| p.to_path_buf()))
                .or_else(|| self.config.last_destination.clone())
                .filter(|p| p.is_dir())
                .or_else(|| disk::removable_mounts().into_iter().next())
                .or_else(|| crate::util::home_dir().map(|h| h.join("Sauvegardes-Mozilla")))
                .filter(|p| p.is_dir())
                .unwrap_or_else(|| crate::util::home_dir().unwrap_or_else(|| PathBuf::from("/")));

            let filters: [(&str, &str); 2] = [
                ("Sauvegardes MozInRustBackup", "*.mozinrustbackup"),
                ("Tous les fichiers", "*"),
            ];
            let Some(path) = self.ui.choose_file(
                "Choisir une sauvegarde à restaurer",
                &start,
                &filters,
            )? else {
                return Ok(());
            };

            let info = match restore::inspect_archive(&path) {
                Ok(info) => info,
                Err(e) => {
                    let retry = self.ui.confirm(
                        "Fichier non reconnu",
                        &format!(
                            "{}\n\n\
                             Une sauvegarde MozInRustBackup porte l'extension .{}\n\
                             et se trouve sur le support choisi lors de la sauvegarde\n\
                             (clé USB, dossier personnel…).",
                            e, BACKUP_EXTENSION
                        ),
                        "Choisir un autre fichier",
                        "Annuler",
                    )?;
                    if retry {
                        continue;
                    }
                    return Ok(());
                }
            };
            self.config.last_archive = Some(path.clone());
            self.config.save();

            if let Err(UiError::Cancelled) = self.restore_one(&info) {
                return Ok(());
            }
            let again = self.ui.confirm(
                "Restauration",
                "Voulez-vous restaurer une autre sauvegarde ?",
                "Oui",
                "Non, terminer",
            )?;
            if !again {
                return Ok(());
            }
        }
    }

    fn restore_one(&mut self, info: &restore::BackupInfo) -> Result<(), UiError> {
        let manifest = &info.manifest;
        let suggested = restore::suggested_target(manifest);

        // 1. Récapitulatif + choix de l'emplacement.
        let mut text = format!(
            "Sauvegarde du {} ({})\n\
             Application : {}   •   Profil : {}   •   Origine : {}\n\
             Ordinateur d'origine : {}\n\
             Dossier d'origine :\n{}\n\n\
             Emplacement de restauration proposé :\n{}\n",
            time::format_datetime(manifest.created_unix, self.tz),
            human_size(info.archive_size),
            manifest.app.label(),
            manifest.profile_name,
            manifest.origin.label(),
            if manifest.machine.is_empty() { "inconnu" } else { &manifest.machine },
            manifest.profile_path.display(),
            suggested.display(),
        );
        if let Some(warning) = &info.warning {
            text.push_str(&format!("\n⚠ {}", warning));
        }
        let items = vec![
            Item::new("propose", vec!["1".into(), "Restaurer à l'emplacement proposé".into()]),
            Item::new("autre", vec!["2".into(), "Choisir un autre emplacement…".into()]),
            Item::new("annuler", vec!["3".into(), "Annuler".into()]),
        ];
        let choice = self.ui.choose("Restauration", &text, &["N°", "Action"], &items)?;
        let target = match choice.as_deref() {
            Some("propose") => suggested.clone(),
            Some("autre") => {
                let start = suggested
                    .parent()
                    .map(|p| p.to_path_buf())
                    .filter(|p| p.is_dir())
                    .or_else(crate::util::home_dir)
                    .unwrap_or_else(|| PathBuf::from("/"));
                match self.ui.choose_dir("Dossier de restauration", &start)? {
                    Some(mut dir) => {
                        // Le profil est recréé dans un sous-dossier s'il n'existe pas déjà.
                        if dir.file_name().map(|n| n.to_string_lossy().to_string())
                            != Some(manifest.profile_dir_name.clone())
                        {
                            dir = dir.join(&manifest.profile_dir_name);
                        }
                        dir
                    }
                    None => return Ok(()),
                }
            }
            _ => return Ok(()),
        };

        // 2. Un profil identique existe-t-il déjà ? (confirmation obligatoire)
        let mut overwrite = Overwrite::Fail;
        if target.exists() {
            let size = fsops::measure_tree(&target, None).size;
            let items = vec![
                Item::new(
                    "renommer",
                    vec![
                        "1".into(),
                        "Remplacer en conservant l'ancien profil (recommandé)".into(),
                    ],
                ),
                Item::new("supprimer", vec!["2".into(), "Remplacer définitivement (l'ancien est effacé)".into()]),
                Item::new("autre", vec!["3".into(), "Restaurer ailleurs".into()]),
                Item::new("annuler", vec!["4".into(), "Annuler".into()]),
            ];
            let text = format!(
                "Un profil existe déjà à cet emplacement :\n{}\n({}, {} fichier(s))\n\n\
                 Que faut-il faire ?\n\n\
                 • Option 1 : l'ancien profil est renommé « …-remplace-<date> » et conservé.\n\
                   Rien n'est perdu ; vous pourrez l'effacer plus tard si tout va bien.\n\
                 • Option 2 : l'ancien profil est supprimé avant la restauration.",
                target.display(),
                human_size(size),
                human_number(fsops::measure_tree(&target, None).files)
            );
            let choice = self.ui.choose("Profil existant", &text, &["N°", "Action"], &items)?;
            match choice.as_deref() {
                Some("renommer") => overwrite = Overwrite::Rename,
                Some("supprimer") => {
                    let sure = self.ui.confirm(
                        "Confirmation",
                        &format!(
                            "Supprimer définitivement le profil actuel ?\n\n{}\n\n\
                             Cette action est irréversible.\n\
                             (L'ancien profil peut aussi être conservé : choisissez « Annuler » \
                             puis l'option 1.)",
                            target.display()
                        ),
                        "Oui, supprimer",
                        "Non, conserver l'ancien",
                    )?;
                    if !sure {
                        return Ok(());
                    }
                    overwrite = Overwrite::Delete;
                }
                Some("autre") => return self.restore_one(info),
                _ => return Ok(()),
            }
        }

        // 3. L'application ne doit pas être ouverte.
        if !self.ensure_application_closed(manifest.app)? {
            return Ok(());
        }

        // 4. Restauration.
        let options = RestoreOptions {
            overwrite,
            verify_hashes: true,
            restore_owner: fsops::is_root(),
        };
        crate::sys::signals::reset();
        self.ui.progress_begin(
            "Restauration en cours…",
            &format!("Restauration du profil dans {}", target.display()),
        )?;
        let result = {
            let mut progress = DialogProgress::new(self.ui, "Restauration");
            restore::restore(info, &target, &options, &mut progress)
        };
        self.ui.progress_end();

        match result {
            Ok(outcome) => {
                let mut text = format!(
                    "Restauration terminée.\n\n\
                     Profil restauré :\n{}\n\n\
                     Fichiers : {}   •   Dossiers : {}   •   Liens symboliques : {}\n\
                     Volume écrit : {}   •   Durée : {}\n",
                    target.display(),
                    human_number(outcome.files),
                    human_number(outcome.dirs),
                    human_number(outcome.symlinks),
                    human_size(outcome.bytes),
                    time::format_duration(outcome.duration_secs)
                );
                if outcome.verified > 0 {
                    text.push_str(&format!(
                        "Intégrité : {} fichier(s) vérifié(s) par empreinte SHA-256 ✔\n",
                        human_number(outcome.verified)
                    ));
                }
                if let Some(renamed) = &outcome.renamed_existing {
                    text.push_str(&format!(
                        "\nL'ancien profil a été conservé ici :\n{}\n\
                         (vous pouvez le supprimer une fois que tout fonctionne)",
                        renamed.display()
                    ));
                }
                text.push_str(&format!(
                    "\nVous pouvez maintenant lancer {} : il retrouvera ce profil.",
                    manifest.app.label()
                ));
                self.ui.info("Restauration réussie", &text)?;
                let items = vec![
                    Item::new("ouvrir", vec!["1".into(), "Ouvrir le dossier du profil restauré".into()]),
                    Item::new("fin", vec!["2".into(), "Terminer".into()]),
                ];
                if let Ok(Some(c)) = self.ui.choose("Et maintenant ?", "", &["N°", "Action"], &items) {
                    if c == "ouvrir" {
                        tools::open_directory(&target);
                    }
                }
                Ok(())
            }
            Err(restore::RestoreError::Cancelled) => {
                self.ui.info(
                    "Restauration annulée",
                    "La restauration a été interrompue. Relancez-la si nécessaire.",
                )?;
                Err(UiError::Cancelled)
            }
            Err(e) => {
                self.ui.error("Échec de la restauration", &format!("{}", e));
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Vérification et liste
    // ------------------------------------------------------------------

    fn verify_flow(&mut self) -> Result<(), UiError> {
        let start = self
            .config
            .last_archive
            .as_ref()
            .and_then(|a| a.parent().map(|p| p.to_path_buf()))
            .filter(|p| p.is_dir())
            .or_else(crate::util::home_dir)
            .unwrap_or_else(|| PathBuf::from("/"));
        let filters: [(&str, &str); 2] = [
            ("Sauvegardes MozInRustBackup", "*.mozinrustbackup"),
            ("Tous les fichiers", "*"),
        ];
        let Some(path) = self.ui.choose_file("Choisir la sauvegarde à vérifier", &start, &filters)?
        else {
            return Ok(());
        };
        let info = match restore::inspect_archive(&path) {
            Ok(info) => info,
            Err(e) => {
                self.ui.error("Fichier non reconnu", &format!("{}", e));
                return Ok(());
            }
        };
        crate::sys::signals::reset();
        self.ui.progress_begin(
            "Vérification en cours…",
            "Contrôle de chaque fichier de la sauvegarde",
        )?;
        let total = info.entries.max(1) as f32;
        let mut checked = 0f32;
        let result = {
            let ui = &mut *self.ui;
            restore::verify_archive(&path, &mut |_done, name| {
                checked += 1.0;
                ui.progress(checked / total, name);
            })
        };
        self.ui.progress(1.0, "Terminé");
        self.ui.progress_end();
        match result {
            Ok((count, bytes)) => {
                self.ui.info(
                    "Sauvegarde valide ✔",
                    &format!(
                        "La sauvegarde est intègre.\n\n\
                         Fichier : {}\n\
                         Profil : {} — {} ({})\n\
                         Dossier d'origine :\n{}\n\n\
                         {} fichier(s) vérifié(s), {} contrôlés par empreinte SHA-256.\n\
                         Restauration possible à tout moment.",
                        info.archive
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        info.manifest.app.label(),
                        info.manifest.profile_name,
                        info.manifest.origin.label(),
                        info.manifest.profile_path.display(),
                        human_number(count),
                        human_size(bytes),
                    ),
                )?;
            }
            Err(e) => {
                self.ui.error(
                    "Sauvegarde endommagée",
                    &format!(
                        "{}\n\n\
                         La sauvegarde ne peut pas être utilisée en l'état.\n\
                         Essayez une autre copie (autre clé USB, autre fichier).",
                        e
                    ),
                );
            }
        }
        Ok(())
    }

    fn list_flow(&mut self) -> Result<(), UiError> {
        let profiles = self.scan(false, Vec::new())?;
        if profiles.is_empty() {
            self.ui.info(
                "Aucun profil",
                "Aucun profil Firefox ou Thunderbird n'a été détecté sur ce disque.",
            )?;
            return Ok(());
        }
        let mut text = format!("{} profil(s) détecté(s) :\n\n", profiles.len());
        for p in &profiles {
            text.push_str(&format!(
                "{}{} — {} ({})\n",
                if p.is_default { "★ " } else { "  " },
                p.app.label(),
                p.name,
                p.origin.label()
            ));
            text.push_str(&format!(
                "     {} — {} — {}\n",
                p.size.map(human_size).unwrap_or_else(|| "?".into()),
                p.file_count.map(|c| format!("{} fichiers", human_number(c))).unwrap_or_default(),
                p.mtime
                    .map(|m| format!("modifié le {}", time::format_datetime(m, self.tz)))
                    .unwrap_or_default()
            ));
            text.push_str(&format!("     {}\n", p.dir.display()));
            if p.locked {
                text.push_str("     (actuellement utilisé : fermez Firefox/Thunderbird avant la sauvegarde)\n");
            }
        }
        self.ui.text_page("Profils détectés", &text)?;
        Ok(())
    }

    /// Utilisé par la ligne de commande : sauvegarde sans dialogue.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Copie les options de balayage (le scanner consomme une référence partagée,
/// ce qui évite un conflit d'emprunt avec la boîte de dialogue de progression).
fn options_clone(options: &ScanOptions) -> &ScanOptions {
    options
}

/// Vérifie que le dossier d'une archive peut être ouvert par l'utilisateur.
pub fn parent_of(path: &Path) -> PathBuf {
    path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

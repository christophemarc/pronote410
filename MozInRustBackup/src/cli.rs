//! Ligne de commande : usage direct (scripts, dépannage) et lancement de
//! l'assistant.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::core::backup::{self, BackupOptions, SilentProgress};
use crate::core::model::{AppKind, Profile};
use crate::core::restore::{self, Overwrite, RestoreOptions};
use crate::core::scanner::{self, ScanOptions};
use crate::sys::fsops;
use crate::ui::scripted::ScriptedDialogs;
use crate::ui::wizard::{Config, Wizard};
use crate::ui::UiError;
use crate::util::{human_number, human_size, time};
use crate::{APP_NAME, APP_VERSION, BACKUP_EXTENSION};

const AIDE: &str = "\
MozInRustBackup — sauvegarde et restauration des profils Firefox/Thunderbird

Utilisation :
  mozinrustbackup                         Assistant pas à pas (fenêtres graphiques)
  mozinrustbackup --liste                 Affiche les profils trouvés sur le disque
  mozinrustbackup --sauvegarder --dossier <dossier> [options]
  mozinrustbackup --restaurer <fichier.mozinrustbackup> [options]
  mozinrustbackup --verifier <fichier.mozinrustbackup>

Options générales :
  --ui <zenity|kdialog|yad|tui>   Force le type de boîtes de dialogue
  --aide, -h                      Affiche cette aide
  --version, -V                   Affiche la version

Sauvegarde :
  --profil <numéro|chemin>        Profil à sauvegarder (numéro de --liste,
                                  ou chemin du dossier de profil)
  --app <firefox|thunderbird>     Limite la recherche à une application
  --dossier <dossier>             Dossier de destination (ex. clé USB)
  --nom <base>                    Force le nom du fichier de sauvegarde
  --sans-caches                   N'inclut pas les caches (sauvegarde allégée)
  --oui                           Ne pose aucune question (écrasement compris)

Restauration :
  --destination <dossier>         Où restaurer le profil (par défaut : le
                                  chemin indiqué dans le journal de sauvegarde)
  --mode <renommer|supprimer|refuser>
                                  Que faire si un profil existe déjà
                                  (par défaut : refuser)

Codes de retour : 0 succès, 1 erreur, 2 mauvaise utilisation, 3 annulation.
";

#[derive(Debug, Default)]
pub struct Args {
    pub help: bool,
    pub version: bool,
    pub list: bool,
    pub backup: bool,
    pub restore: Option<PathBuf>,
    pub verify: Option<PathBuf>,
    pub profile: Option<String>,
    pub app: Option<String>,
    pub destination: Option<PathBuf>,
    pub name: Option<String>,
    pub no_caches: bool,
    pub yes: bool,
    pub mode: Option<String>,
    pub ui: Option<String>,
    pub extra_search: Vec<PathBuf>,
}

pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].as_str();
        let next = |i: &mut usize| -> Result<String, String> {
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("valeur manquante après {}", arg))
        };
        match arg {
            "--aide" | "--help" | "-h" => out.help = true,
            "--version" | "-V" => out.version = true,
            "--liste" | "--list" | "-l" => out.list = true,
            "--sauvegarder" | "--backup" | "-s" => out.backup = true,
            "--restaurer" | "--restore" | "-r" => out.restore = Some(PathBuf::from(next(&mut i)?)),
            "--verifier" | "--verify" => out.verify = Some(PathBuf::from(next(&mut i)?)),
            "--profil" | "--profile" => out.profile = Some(next(&mut i)?),
            "--app" => out.app = Some(next(&mut i)?),
            "--dossier" | "--destination-dossier" => {
                out.destination = Some(PathBuf::from(next(&mut i)?))
            }
            "--destination" => out.destination = Some(PathBuf::from(next(&mut i)?)),
            "--nom" | "--name" => out.name = Some(next(&mut i)?),
            "--sans-caches" | "--no-caches" => out.no_caches = true,
            "--oui" | "--yes" | "-y" => out.yes = true,
            "--mode" => out.mode = Some(next(&mut i)?),
            "--ui" => out.ui = Some(next(&mut i)?),
            "--chercher" | "--search" => out.extra_search.push(PathBuf::from(next(&mut i)?)),
            other => {
                if other.starts_with('-') {
                    return Err(format!("option inconnue : {}", other));
                }
                // Chemin nu : considéré comme une sauvegarde à restaurer.
                out.restore = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }
    Ok(out)
}

pub fn run(args: Args) -> ExitCode {
    if args.help {
        println!("{}", AIDE);
        return ExitCode::SUCCESS;
    }
    if args.version {
        println!("{} {}", APP_NAME, APP_VERSION);
        println!("format de sauvegarde : {} (extension .{})", crate::FORMAT_VERSION, BACKUP_EXTENSION);
        return ExitCode::SUCCESS;
    }
    if let Some(ui) = &args.ui {
        std::env::set_var("MRB_UI", ui);
    }

    let code = if args.list {
        cmd_list(&args)
    } else if args.backup {
        cmd_backup(&args)
    } else if let Some(archive) = &args.restore {
        cmd_restore(&args, archive)
    } else if let Some(archive) = &args.verify {
        cmd_verify(archive)
    } else {
        cmd_wizard(&args)
    };
    code
}

fn app_filter(args: &Args) -> Vec<AppKind> {
    match args.app.as_deref() {
        Some(v) => match AppKind::from_key(v) {
            Some(a) => vec![a],
            None => vec![AppKind::Firefox, AppKind::Thunderbird],
        },
        None => vec![AppKind::Firefox, AppKind::Thunderbird],
    }
}

fn scan_all(args: &Args) -> Vec<Profile> {
    let report = scanner::scan(
        &ScanOptions {
            apps: app_filter(args),
            extra_roots: args.extra_search.clone(),
            deep_search: !args.extra_search.is_empty(),
            measure: true,
        },
        &mut |_| {},
    );
    report.profiles
}

/// Affichage texte d'un profil, aligné en colonnes.
fn describe_line(index: usize, p: &Profile, tz: i32) -> String {
    format!(
        "{:>2}. {} {:<11} {:<14} {:<22} {:>10}  {:>16}  {}",
        index + 1,
        if p.is_default { "★" } else { " " },
        p.app.label(),
        p.origin.label(),
        truncate(&p.name, 22),
        p.size.map(human_size).unwrap_or_else(|| "?".to_string()),
        p.mtime
            .map(|m| time::format_datetime(m, tz))
            .unwrap_or_else(|| "?".to_string()),
        p.dir.display()
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn cmd_list(args: &Args) -> ExitCode {
    let tz = crate::util::tz_offset_secs();
    let profiles = scan_all(args);
    if profiles.is_empty() {
        println!("Aucun profil Firefox ou Thunderbird trouvé sur ce disque.");
        println!("(lancez au moins une fois l'application, ou utilisez --chercher <dossier>)");
        return ExitCode::SUCCESS;
    }
    println!(
        "{} profil(s) trouvé(s) — ★ = profil par défaut\n",
        profiles.len()
    );
    println!(
        "  {:<13} {:<14} {:<22} {:>10}  {:>16}  {}",
        "Type", "Origine", "Profil", "Taille", "Dernière modif.", "Dossier"
    );
    for (i, p) in profiles.iter().enumerate() {
        println!("{}", describe_line(i, p, tz));
    }
    println!("\nLe profil d'origine et sa taille sont également affichés dans l'assistant.");
    ExitCode::SUCCESS
}

fn find_profile(args: &Args, profiles: &[Profile]) -> Result<usize, String> {
    let selector = args.profile.clone();
    match selector {
        None => {
            // Préférence : profil par défaut, sinon le plus récemment modifié.
            let index = profiles
                .iter()
                .position(|p| p.is_default)
                .unwrap_or(0);
            Ok(index)
        }
        Some(sel) => {
            if let Ok(n) = sel.parse::<usize>() {
                if n >= 1 && n <= profiles.len() {
                    return Ok(n - 1);
                }
                return Err(format!("numéro de profil hors limites : {}", sel));
            }
            let path = PathBuf::from(&sel);
            if let Some(index) = profiles.iter().position(|p| p.dir == path) {
                return Ok(index);
            }
            // Chemin direct : on l'accepte même s'il n'est pas déclaré.
            if path.is_dir() {
                return Err(format!(
                    "ce dossier n'est pas un profil déclaré : {}\n\
                     utilisez --chercher {} pour l'analyser",
                    path.display(),
                    path.display()
                ));
            }
            Err(format!("profil introuvable : {}", sel))
        }
    }
}

fn cmd_backup(args: &Args) -> ExitCode {
    let tz = crate::util::tz_offset_secs();
    let profiles = scan_all(args);
    if profiles.is_empty() {
        eprintln!("Aucun profil à sauvegarder n'a été trouvé.");
        return ExitCode::FAILURE;
    }
    let index = match find_profile(args, &profiles) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    let profile = &profiles[index];
    let dest = args
        .destination
        .clone()
        .or_else(|| Config::load().last_destination)
        .unwrap_or_else(crate::sys::disk::default_backup_dir);
    if let Err(e) = fsops::ensure_dir(&dest) {
        eprintln!("dossier de destination inaccessible ({}): {}", dest.display(), e);
        return ExitCode::FAILURE;
    }

    let stem = match &args.name {
        Some(name) => name.clone(),
        None => profile.backup_stem(&time::date_suffix(time::now_unix(), tz)),
    };
    let archive = dest.join(format!("{}.{}", stem, BACKUP_EXTENSION));
    if archive.exists() && !args.yes {
        eprintln!(
            "La sauvegarde existe déjà : {}\n(--oui pour la remplacer)",
            archive.display()
        );
        return ExitCode::FAILURE;
    }
    if let Some(warning) = backup::check_free_space(profile.size.unwrap_or(0), &dest) {
        eprintln!("Attention : {}", warning);
    }

    println!("Profil      : {} — {} ({})", profile.app.label(), profile.name, profile.origin.label());
    println!("Origine     : {}", profile.dir.display());
    println!("Destination : {}", archive.display());
    let options = BackupOptions {
        exclude_caches: args.no_caches,
        level: crate::archive::deflate::Level::Normal,
        verify_after: true,
    };
    match backup::create_backup(profile, &archive, &options, tz, &mut SilentProgress) {
        Ok(outcome) => {
            println!(
                "Sauvegarde terminée : {} fichiers, {} → {} (durée {}).",
                human_number(outcome.files),
                human_size(outcome.source_bytes),
                human_size(outcome.archive_bytes),
                time::format_duration(outcome.duration_secs)
            );
            println!("Journal     : {}", outcome.log.display());
            println!("SHA-256     : {}", outcome.archive_sha256);
            let mut config = Config::load();
            config.last_destination = Some(dest);
            config.last_archive = Some(archive);
            config.save();
            ExitCode::SUCCESS
        }
        Err(backup::BackupError::Cancelled) => {
            eprintln!("Sauvegarde annulée.");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("Échec de la sauvegarde : {}", e);
            ExitCode::FAILURE
        }
    }
}

fn cmd_restore(args: &Args, archive: &PathBuf) -> ExitCode {
    let tz = crate::util::tz_offset_secs();
    let info = match restore::inspect_archive(archive) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    let target = args
        .destination
        .clone()
        .unwrap_or_else(|| restore::suggested_target(&info.manifest));
    let overwrite = match args.mode.as_deref().unwrap_or("refuser") {
        "renommer" | "rename" => Overwrite::Rename,
        "supprimer" | "delete" | "remplacer" => Overwrite::Delete,
        _ => Overwrite::Fail,
    };

    println!("Sauvegarde  : {}", info.archive.display());
    println!(
        "Profil      : {} — {} ({})",
        info.manifest.app.label(),
        info.manifest.profile_name,
        info.manifest.origin.label()
    );
    println!("Créée le    : {}", time::format_datetime(info.manifest.created_unix, tz));
    println!("D'origine  : {}", info.manifest.profile_path.display());
    println!("Restauration vers : {}", target.display());
    if let Some(w) = &info.warning {
        println!("Remarque    : {}", w);
    }
    if target.exists() && overwrite == Overwrite::Fail {
        eprintln!(
            "\nUn profil existe déjà à cet endroit.\n\
             Utilisez --mode renommer (l'ancien profil est conservé) ou --mode supprimer."
        );
        return ExitCode::FAILURE;
    }

    let options = RestoreOptions {
        overwrite,
        verify_hashes: true,
        restore_owner: fsops::is_root(),
    };
    match restore::restore(&info, &target, &options, &mut SilentProgress) {
        Ok(outcome) => {
            println!(
                "\nRestauration terminée : {} fichiers, {} ({})",
                human_number(outcome.files),
                human_size(outcome.bytes),
                time::format_duration(outcome.duration_secs)
            );
            if let Some(renamed) = &outcome.renamed_existing {
                println!("Ancien profil conservé : {}", renamed.display());
            }
            let mut config = Config::load();
            config.last_archive = Some(archive.clone());
            config.save();
            ExitCode::SUCCESS
        }
        Err(restore::RestoreError::Cancelled) => {
            eprintln!("Restauration annulée.");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("Échec de la restauration : {}", e);
            ExitCode::FAILURE
        }
    }
}

fn cmd_verify(archive: &PathBuf) -> ExitCode {
    let info = match restore::inspect_archive(archive) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };
    println!(
        "Vérification de {} ({} profil « {} », {} entrées)…",
        archive.display(),
        info.manifest.app.label(),
        info.manifest.profile_name,
        info.entries
    );
    match restore::verify_archive(archive, &mut |_, _| {}) {
        Ok((count, bytes)) => {
            println!(
                "Sauvegarde valide ✔ — {} fichier(s) vérifié(s), {} de données.",
                human_number(count),
                human_size(bytes)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Sauvegarde endommagée : {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Lance l'assistant : boîtes de dialogue graphiques si possible, sinon texte.
fn cmd_wizard(_args: &Args) -> ExitCode {
    let tool = crate::sys::tools::detect_dialog_tool();
    if let Some((kind, path)) = tool {
        if crate::sys::tools::graphical_session() {
            let mut ui = crate::ui::zenity::ZenityDialogs::new(kind, path.clone());
            let mut wizard = Wizard::new(&mut ui);
            return finish(wizard.run());
        }
    }
    // Mode texte : aucune boîte de dialogue graphique disponible.
    let mut ui = crate::ui::tui::TuiDialogs::new();
    let mut wizard = Wizard::new(&mut ui);
    finish(wizard.run())
}

fn finish(result: Result<(), UiError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(UiError::Cancelled) => ExitCode::from(3),
        Err(e) => {
            eprintln!("{} : {}", APP_NAME, e);
            ExitCode::FAILURE
        }
    }
}

/// Mode entièrement scripté (utilisé par les tests et les démonstrations).
pub fn run_scripted(script: ScriptedDialogs) -> Result<(), UiError> {
    let mut ui = script;
    let mut wizard = Wizard::new(&mut ui);
    wizard.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_options() {
        let args: Vec<String> = ["--sauvegarder", "--dossier", "/tmp/x", "--sans-caches", "--oui"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.backup);
        assert_eq!(parsed.destination, Some(PathBuf::from("/tmp/x")));
        assert!(parsed.no_caches);
        assert!(parsed.yes);
    }

    #[test]
    fn rejects_unknown_option() {
        let args = vec!["--bidule".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn bare_path_means_restore() {
        let args = vec!["/media/usb/Firefox_11102026_Flatpak.mozinrustbackup".to_string()];
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.restore.is_some());
    }
}

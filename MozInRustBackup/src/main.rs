//! MozInRustBackup — point d'entrée du programme.

use std::process::ExitCode;

fn main() -> ExitCode {
    // Arrêt propre sur Ctrl+C : les fichiers temporaires sont supprimés.
    mozinrustbackup::sys::signals::install();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match mozinrustbackup::cli::parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{} : {}", mozinrustbackup::APP_NAME, message);
            eprintln!("Utilisez --aide pour afficher l'aide.");
            return ExitCode::from(2);
        }
    };
    mozinrustbackup::cli::run(parsed)
}

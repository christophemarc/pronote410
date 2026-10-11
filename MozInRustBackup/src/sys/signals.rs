//! Interruption propre (Ctrl+C, SIGTERM).
//!
//! Un simple drapeau atomique est positionné par le gestionnaire de signal :
//! les opérations longues peuvent ainsi s'arrêter d'elles-mêmes, supprimer le
//! fichier temporaire et laisser le disque dans un état propre.

use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;
const SIGPIPE: i32 = 13;

extern "C" {
    fn signal(sig: i32, handler: usize) -> usize;
}

extern "C" fn handler(_sig: i32) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Installe les gestionnaires (appelé au démarrage du programme).
pub fn install() {
    unsafe {
        signal(SIGINT, handler as usize);
        signal(SIGTERM, handler as usize);
        // Écrire dans une boîte de dialogue fermée ne doit pas tuer le programme.
        signal(SIGPIPE, 1 /* SIG_IGN */);
    }
}

/// Vrai si l'utilisateur a demandé l'interruption.
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Réinitialise le drapeau (avant une nouvelle opération longue).
pub fn reset() {
    INTERRUPTED.store(false, Ordering::SeqCst);
}

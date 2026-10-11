//! MozInRustBackup — sauvegarde et restauration des profils Firefox et
//! Thunderbird sous Linux.
//!
//! Bibliothèque interne du programme :
//!
//! * [`archive`] : format de sauvegarde `.mozinrustbackup` (ZIP + manifeste),
//!   compresseur/décompresseur DEFLATE, CRC-32 et SHA-256 écrits à la main ;
//! * [`core`] : détection des profils, sauvegarde, restauration, manifeste ;
//! * [`sys`] : système d'exploitation, distribution, processus, outils externes ;
//! * [`ui`] : boîtes de dialogue (zenity, kdialog ou terminal).

pub mod archive;
pub mod cli;
pub mod core;
pub mod sys;
pub mod ui;
pub mod util;

/// Nom du logiciel, affiché dans les dialogues.
pub const APP_NAME: &str = "MozInRustBackup";
/// Version du logiciel.
///
/// `CARGO_PKG_VERSION` n'existe que lors d'une compilation par cargo ; le
/// script `build.sh` sait aussi compiler avec `rustc` seul, d'où ce repli.
pub const APP_VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
    Some(v) => v,
    None => "1.0.0",
};
/// Extension des sauvegardes.
pub const BACKUP_EXTENSION: &str = "mozinrustbackup";
/// Identifiant du manifeste présent dans chaque archive.
pub const MANIFEST_MEMBER: &str = "mozinrustbackup.manifeste.txt";
/// Liste des empreintes SHA-256 des fichiers sauvegardés.
pub const HASHES_MEMBER: &str = "mozinrustbackup.empreintes.txt";
/// Signature interne permettant de reconnaître une sauvegarde légitime.
pub const MAGIC: &str = "MOZINRUSTBACKUP-SAUVERGARDE";
/// Version du *format* de sauvegarde. Elle ne change pas d'une version du
/// logiciel à l'autre : toutes les versions relisent toutes les sauvegardes.
pub const FORMAT_VERSION: u32 = 1;

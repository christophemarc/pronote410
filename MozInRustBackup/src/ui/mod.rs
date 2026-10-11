//! Interfaces avec l'utilisateur.
//!
//! Le programme utilise les boîtes de dialogue **natives** du bureau
//! (`zenity` sous GNOME/Cinnamon/XFCE/MATE, `kdialog` sous KDE, `yad` s'il est
//! installé) : l'utilisateur retrouve les fenêtres habituelles de son système,
//! comme dans MozBackup sous Windows.
//!
//! Si aucune boîte de dialogue n'est disponible (serveur, session en terminal,
//! SSH), un assistant en mode texte prend le relais : le logiciel reste
//! utilisable partout.

use std::path::{Path, PathBuf};

pub mod kdialog;
pub mod scripted;
pub mod tui;
pub mod wizard;
pub mod zenity;

/// Erreur d'interface.
#[derive(Debug)]
pub enum UiError {
    /// L'utilisateur a annulé la boîte de dialogue.
    Cancelled,
    /// La boîte de dialogue n'a pas pu être affichée.
    Backend(String),
    Io(std::io::Error),
}

impl std::fmt::Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiError::Cancelled => write!(f, "annulé par l'utilisateur"),
            UiError::Backend(m) => write!(f, "{}", m),
            UiError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for UiError {}

impl From<std::io::Error> for UiError {
    fn from(e: std::io::Error) -> Self {
        UiError::Io(e)
    }
}

/// Élément d'une liste de choix.
#[derive(Debug, Clone)]
pub struct Item {
    /// Valeur renvoyée lorsque l'élément est sélectionné.
    pub id: String,
    /// Colonnes affichées.
    pub cells: Vec<String>,
}

impl Item {
    pub fn new(id: impl Into<String>, cells: Vec<String>) -> Item {
        Item { id: id.into(), cells }
    }

    /// Version sur une seule ligne, pour les interfaces sans colonnes.
    pub fn flat(&self) -> String {
        self.cells.join(" — ")
    }
}

/// Boîte de dialogue générique, quelle que soit l'implémentation.
pub trait Dialogues {
    /// Affiche un message d'information.
    fn info(&mut self, title: &str, text: &str) -> Result<(), UiError>;
    /// Affiche une erreur (ne peut pas échouer : dernier recours sur stderr).
    fn error(&mut self, title: &str, text: &str);
    /// Question fermée : vrai si l'utilisateur confirme.
    fn confirm(
        &mut self,
        title: &str,
        text: &str,
        ok_label: &str,
        cancel_label: &str,
    ) -> Result<bool, UiError> {
        let choice = self.choose(
            title,
            text,
            &["Choix", "Action"],
            &[
                Item::new("ok", vec!["".into(), ok_label.to_string()]),
                Item::new("cancel", vec!["".into(), cancel_label.to_string()]),
            ],
        )?;
        Ok(matches!(choice.as_deref(), Some("ok")))
    }
    /// Liste de choix à colonnes. `Ok(None)` = annulation.
    fn choose(
        &mut self,
        title: &str,
        text: &str,
        columns: &[&str],
        items: &[Item],
    ) -> Result<Option<String>, UiError>;
    /// Choix d'un dossier.
    fn choose_dir(&mut self, title: &str, start: &Path) -> Result<Option<PathBuf>, UiError>;
    /// Choix d'un fichier ; `filters` décrit les types proposés.
    fn choose_file(
        &mut self,
        title: &str,
        start: &Path,
        filters: &[(&str, &str)],
    ) -> Result<Option<PathBuf>, UiError>;
    /// Saisie d'un chemin ou d'un texte.
    fn prompt(
        &mut self,
        title: &str,
        text: &str,
        default: &str,
    ) -> Result<Option<String>, UiError>;
    /// Affichage d'un texte long (défilable).
    fn text_page(&mut self, title: &str, text: &str) -> Result<(), UiError>;

    // --- progression ------------------------------------------------------
    fn progress_begin(&mut self, title: &str, text: &str) -> Result<(), UiError>;
    fn progress(&mut self, fraction: f32, text: &str);
    fn progress_end(&mut self);
    /// Vrai si l'utilisateur a demandé l'annulation.
    fn progress_cancelled(&mut self) -> bool;
}

/// Netttoie une cellule : les tabulations et sauts de ligne casseraient les
/// listes des boîtes de dialogue.
pub fn clean_cell(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ").trim().to_string()
}

/// Découpe un texte en lignes d'au plus `width` caractères (pour l'affichage).
pub fn wrap_text(text: &str, width: usize) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.chars().count() <= width {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let mut current = String::new();
        for word in line.split(' ') {
            if current.is_empty() {
                current.push_str(word);
            } else if current.chars().count() + 1 + word.chars().count() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                out.push_str(&current);
                out.push('\n');
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            out.push_str(&current);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleaning_and_wrapping() {
        assert_eq!(clean_cell("a\tb\nc"), "a b c");
        let wrapped = wrap_text("un deux trois quatre cinq six sept huit", 12);
        assert!(wrapped.lines().all(|l| l.chars().count() <= 12));
    }
}

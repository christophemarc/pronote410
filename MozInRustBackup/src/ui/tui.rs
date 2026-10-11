//! Assistant en mode texte : utilisé lorsqu'aucune boîte de dialogue graphique
//! n'est disponible (serveur, session SSH, terminal sans zenity/kdialog).
//!
//! L'ergonomie reste volontairement simple : numéros de choix, valeur par
//! défaut rappelée entre crochets, Entrée pour valider.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use super::{clean_cell, Dialogues, Item, UiError};

pub struct TuiDialogs {
    progress_shown: bool,
    last_percent: u32,
}

impl Default for TuiDialogs {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiDialogs {
    pub fn new() -> Self {
        TuiDialogs { progress_shown: false, last_percent: 101 }
    }

    fn read_line(&self) -> io::Result<Option<String>> {
        let mut line = String::new();
        io::stdout().flush().ok();
        let n = io::stdin().lock().read_line(&mut line)?;
        if n == 0 {
            return Ok(None); // fin d'entrée (pipe) : on annule proprement
        }
        Ok(Some(line.trim().to_string()))
    }

    fn pause(&self) {
        print!("\nAppuyez sur Entrée pour continuer… ");
        let _ = self.read_line();
    }

    fn frame(&self, title: &str, text: &str) {
        let width = 72usize;
        println!();
        println!("┌{}┐", "─".repeat(width));
        println!("│ {:<width$} │", truncate(title, width - 1), width = width - 1);
        println!("├{}┤", "─".repeat(width));
        for line in super::wrap_text(text, width - 2).lines() {
            println!("│ {:<width$} │", truncate(line, width - 1), width = width - 1);
        }
        println!("└{}┘", "─".repeat(width));
    }
}

fn truncate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

impl Dialogues for TuiDialogs {
    fn info(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.frame(title, text);
        self.pause();
        Ok(())
    }

    fn error(&mut self, title: &str, text: &str) {
        eprintln!();
        eprintln!("!!! {} !!!", title);
        for line in text.lines() {
            eprintln!("    {}", line);
        }
        self.pause();
    }

    fn confirm(
        &mut self,
        title: &str,
        text: &str,
        ok_label: &str,
        cancel_label: &str,
    ) -> Result<bool, UiError> {
        loop {
            self.frame(title, text);
            print!("{} ? [O/n] (o = {}, n = {}) : ", "Confirmation", ok_label, cancel_label);
            match self.read_line()? {
                None => return Ok(false),
                Some(answer) => {
                    let a = answer.trim().to_ascii_lowercase();
                    if a.is_empty() || a == "o" || a == "oui" || a == "y" || a == "yes" {
                        return Ok(true);
                    }
                    if a == "n" || a == "non" || a == "no" {
                        return Ok(false);
                    }
                }
            }
        }
    }

    fn choose(
        &mut self,
        title: &str,
        text: &str,
        columns: &[&str],
        items: &[Item],
    ) -> Result<Option<String>, UiError> {
        loop {
            self.frame(title, text);
            println!("  {}. {}", 0, "Annuler / revenir en arrière");
            let _ = columns;
            for (i, item) in items.iter().enumerate() {
                // En mode texte, les colonnes sont condensées sur une ligne.
                let label = item
                    .cells
                    .iter()
                    .map(|c| clean_cell(c))
                    .filter(|c| !c.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ");
                println!("  {}. {}", i + 1, label);
            }
            print!("Votre choix [1-{}], Entrée = 1, 0 = annuler : ", items.len());
            match self.read_line()? {
                None => return Ok(None),
                Some(answer) => {
                    let a = answer.trim();
                    if a.is_empty() {
                        return Ok(Some(items[0].id.clone()));
                    }
                    match a.parse::<usize>() {
                        Ok(0) => return Ok(None),
                        Ok(n) if n >= 1 && n <= items.len() => {
                            return Ok(Some(items[n - 1].id.clone()))
                        }
                        _ => continue,
                    }
                }
            }
        }
    }

    fn choose_dir(&mut self, title: &str, start: &Path) -> Result<Option<PathBuf>, UiError> {
        self.frame(title, &format!("Dossier proposé : {}", start.display()));
        print!("Chemin du dossier [{}] (Entrée = proposé, vide = annuler) : ", start.display());
        match self.read_line()? {
            None => Ok(None),
            Some(answer) => {
                if answer.trim().is_empty() {
                    Ok(Some(start.to_path_buf()))
                } else {
                    Ok(Some(PathBuf::from(shellexpand(&answer))))
                }
            }
        }
    }

    fn choose_file(
        &mut self,
        title: &str,
        start: &Path,
        _filters: &[(&str, &str)],
    ) -> Result<Option<PathBuf>, UiError> {
        if start.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(start)
                .map(|rd| {
                    rd.flatten()
                        .map(|e| e.path())
                        .filter(|p| {
                            p.is_file()
                                && p.extension()
                                    .map(|e| e.to_string_lossy().eq_ignore_ascii_case(crate::BACKUP_EXTENSION))
                                    .unwrap_or(false)
                        })
                        .collect()
                })
                .unwrap_or_default();
            found.sort();
            if !found.is_empty() {
                let items: Vec<Item> = found
                    .iter()
                    .map(|p| {
                        Item::new(
                            p.display().to_string(),
                            vec![p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()],
                        )
                    })
                    .collect();
                if let Some(choice) = self.choose(title, "Sauvegardes trouvées dans ce dossier :", &["Fichier"], &items)? {
                    return Ok(Some(PathBuf::from(choice)));
                }
                return Ok(None);
            }
        }
        self.frame(title, "Indiquez le chemin complet de la sauvegarde (.mozinrustbackup)");
        print!("Chemin du fichier (vide = annuler) : ");
        match self.read_line()? {
            None => Ok(None),
            Some(answer) if answer.trim().is_empty() => Ok(None),
            Some(answer) => Ok(Some(PathBuf::from(shellexpand(&answer)))),
        }
    }

    fn prompt(&mut self, title: &str, text: &str, default: &str) -> Result<Option<String>, UiError> {
        self.frame(title, text);
        print!("[{}] : ", default);
        match self.read_line()? {
            None => Ok(None),
            Some(answer) => {
                if answer.trim().is_empty() {
                    Ok(Some(default.to_string()))
                } else {
                    Ok(Some(answer))
                }
            }
        }
    }

    fn text_page(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.frame(title, "");
        println!("{}", text);
        self.pause();
        Ok(())
    }

    fn progress_begin(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        println!();
        println!("== {} ==", title);
        println!("   {} (Ctrl+C pour annuler)", text);
        self.progress_shown = true;
        self.last_percent = 101;
        Ok(())
    }

    fn progress(&mut self, fraction: f32, text: &str) {
        let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as u32;
        // On n'écrit que lorsque l'affichage change vraiment : les journaux de
        // terminal (et les enregistrements de session) restent lisibles.
        if percent == self.last_percent {
            return;
        }
        if percent % 5 == 0 || percent >= 100 {
            self.last_percent = percent;
            print!("\r   {:>3} % — {:<60}", percent, truncate(text, 60));
            let _ = io::stdout().flush();
        }
    }

    fn progress_end(&mut self) {
        if self.progress_shown {
            println!();
            self.progress_shown = false;
        }
    }

    fn progress_cancelled(&mut self) -> bool {
        crate::sys::signals::interrupted()
    }
}

/// Remplace `~` par le dossier personnel.
fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = crate::util::home_dir() {
            return home.join(rest).display().to_string();
        }
    }
    path.to_string()
}

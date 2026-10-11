//! Boîtes de dialogue `zenity` (GNOME, Cinnamon, XFCE, MATE, Unity, Budgie…)
//! et `yad`, qui accepte la même syntaxe.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::{clean_cell, Dialogues, Item, UiError};
use crate::sys::tools::DialogTool;

pub struct ZenityDialogs {
    program: PathBuf,
    kind: DialogTool,
    progress: Option<Child>,
    cancelled: bool,
    width: u32,
}

impl ZenityDialogs {
    pub fn new(kind: DialogTool, program: PathBuf) -> Self {
        ZenityDialogs { program, kind, progress: None, cancelled: false, width: 620 }
    }

    fn base(&self, mode: &str) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.arg(mode);
        cmd
    }

    fn run(&self, cmd: &mut Command) -> Result<(bool, String), UiError> {
        let out = cmd
            .stdin(Stdio::null())
            .output()
            .map_err(|e| UiError::Backend(format!("zenity indisponible : {}", e)))?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let code = out.status.code().unwrap_or(1);
        match code {
            0 => Ok((true, text)),
            1 => Ok((false, text)),
            _ => {
                let err = String::from_utf8_lossy(&out.stderr).to_string();
                Err(UiError::Backend(format!(
                    "la boîte de dialogue n'a pas pu s'afficher ({}) {}",
                    code,
                    err.trim()
                )))
            }
        }
    }

    fn is_yad(&self) -> bool {
        self.kind == DialogTool::Yad
    }

    /// Ajoute les colonnes et les éléments d'une liste.
    fn build_list(&self, cmd: &mut Command, columns: &[&str], items: &[Item]) {
        for c in columns {
            cmd.arg(format!("--column={}", clean_cell(c)));
        }
        for item in items {
            for cell in &item.cells {
                cmd.arg(clean_cell(cell));
            }
        }
    }
}

impl Drop for ZenityDialogs {
    fn drop(&mut self) {
        let _ = self.progress_end();
    }
}

impl Dialogues for ZenityDialogs {
    fn info(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        let mut cmd = self.base("--info");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--text={}", text))
            .arg(format!("--width={}", self.width));
        self.run(&mut cmd)?;
        Ok(())
    }

    fn error(&mut self, title: &str, text: &str) {
        let mut cmd = self.base("--error");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--text={}", text))
            .arg(format!("--width={}", self.width));
        if self.run(&mut cmd).is_err() {
            eprintln!("{} : {}", title, text);
        }
    }

    fn confirm(
        &mut self,
        title: &str,
        text: &str,
        ok_label: &str,
        cancel_label: &str,
    ) -> Result<bool, UiError> {
        let mut cmd = self.base("--question");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--text={}", text))
            .arg(format!("--ok-label={}", ok_label))
            .arg(format!("--cancel-label={}", cancel_label))
            .arg(format!("--width={}", self.width));
        let (ok, _) = self.run(&mut cmd)?;
        Ok(ok)
    }

    fn choose(
        &mut self,
        title: &str,
        text: &str,
        columns: &[&str],
        items: &[Item],
    ) -> Result<Option<String>, UiError> {
        let mut cmd = self.base("--list");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--text={}", text))
            .arg("--print-column=1")
            .arg("--hide-column=1")
            .arg(format!("--width={}", self.width))
            .arg(format!("--height={}", (120 + items.len() * 28).min(560)));
        self.build_list(&mut cmd, columns, items);
        let (ok, out) = self.run(&mut cmd)?;
        if !ok {
            return Ok(None);
        }
        Ok(Some(out))
    }

    fn choose_dir(&mut self, title: &str, start: &Path) -> Result<Option<PathBuf>, UiError> {
        let mut cmd = self.base("--file-selection");
        cmd.arg("--directory")
            .arg(format!("--title={}", title))
            .arg(format!("--filename={}/", start.display()))
            .arg(format!("--width={}", self.width));
        let (ok, out) = self.run(&mut cmd)?;
        if !ok || out.is_empty() {
            return Ok(None);
        }
        Ok(Some(PathBuf::from(out)))
    }

    fn choose_file(
        &mut self,
        title: &str,
        start: &Path,
        filters: &[(&str, &str)],
    ) -> Result<Option<PathBuf>, UiError> {
        let mut cmd = self.base("--file-selection");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--filename={}/", start.display()))
            .arg(format!("--width={}", self.width));
        for (label, patterns) in filters {
            cmd.arg(format!("--file-filter={} | {}", label, patterns));
        }
        let (ok, out) = self.run(&mut cmd)?;
        if !ok || out.is_empty() {
            return Ok(None);
        }
        Ok(Some(PathBuf::from(out)))
    }

    fn prompt(&mut self, title: &str, text: &str, default: &str) -> Result<Option<String>, UiError> {
        let mut cmd = self.base("--entry");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--text={}", text))
            .arg(format!("--entry-text={}", default))
            .arg(format!("--width={}", self.width));
        let (ok, out) = self.run(&mut cmd)?;
        if !ok || out.is_empty() {
            return Ok(None);
        }
        Ok(Some(out))
    }

    fn text_page(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        let mut cmd = self.base("--text-info");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--width={}", self.width))
            .arg("--height=520")
            .arg("--font=Monospace 10")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| UiError::Backend(format!("zenity indisponible : {}", e)))?;
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        drop(child.stdin.take());
        let _ = child.wait();
        Ok(())
    }

    fn progress_begin(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.progress_end();
        self.cancelled = false;
        let mut cmd = self.base("--progress");
        cmd.arg(format!("--title={}", title))
            .arg(format!("--text={}", text))
            .arg("--percentage=0")
            .arg("--auto-kill")
            .arg(format!("--width={}", self.width))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if self.is_yad() {
            // yad n'accepte pas toutes les options de zenity.
            cmd = Command::new(&self.program);
            cmd.arg("--progress")
                .arg(format!("--title={}", title))
                .arg(format!("--text={}", text))
                .arg("--percentage=0")
                .arg(format!("--width={}", self.width))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        }
        let child = cmd
            .spawn()
            .map_err(|e| UiError::Backend(format!("zenity indisponible : {}", e)))?;
        self.progress = Some(child);
        Ok(())
    }

    fn progress(&mut self, fraction: f32, text: &str) {
        let Some(child) = self.progress.as_mut() else { return };
        let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as u32;
        if let Some(stdin) = child.stdin.as_mut() {
            let line = format!("{}\n# {}\n", percent, clean_cell(text));
            if stdin.write_all(line.as_bytes()).is_err() || stdin.flush().is_err() {
                self.cancelled = true;
            }
        }
        // Détection de l'annulation (fermeture de la fenêtre par l'utilisateur).
        if let Ok(Some(status)) = child.try_wait() {
            if !status.success() {
                self.cancelled = true;
            }
        }
    }

    fn progress_end(&mut self) {
        if let Some(mut child) = self.progress.take() {
            drop(child.stdin.take());
            let _ = child.wait();
        }
        self.cancelled = false;
    }

    fn progress_cancelled(&mut self) -> bool {
        if let Some(child) = self.progress.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                if !status.success() {
                    self.cancelled = true;
                }
            }
        }
        self.cancelled
    }
}

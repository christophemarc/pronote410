//! Boîtes de dialogue `kdialog` (environnements KDE).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::{clean_cell, Dialogues, Item, UiError};

pub struct KDialogDialogs {
    program: PathBuf,
    progress: Option<Child>,
    cancelled: bool,
    service: Option<String>,
}

impl KDialogDialogs {
    pub fn new(program: PathBuf) -> Self {
        KDialogDialogs { program, progress: None, cancelled: false, service: None }
    }

    fn run(&self, args: &[String]) -> Result<(bool, String), UiError> {
        let out = Command::new(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| UiError::Backend(format!("kdialog indisponible : {}", e)))?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        match out.status.code().unwrap_or(1) {
            0 => Ok((true, text)),
            1 => Ok((false, text)),
            code => Err(UiError::Backend(format!(
                "la boîte de dialogue n'a pas pu s'afficher ({}) {}",
                code,
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }
}

impl Drop for KDialogDialogs {
    fn drop(&mut self) {
        self.progress_end();
    }
}

impl Dialogues for KDialogDialogs {
    fn info(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.run(&["--title".into(), title.into(), "--msgbox".into(), text.into()])?;
        Ok(())
    }

    fn error(&mut self, title: &str, text: &str) {
        if self
            .run(&["--title".into(), title.into(), "--error".into(), text.into()])
            .is_err()
        {
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
        let (ok, _) = self.run(&[
            "--title".into(),
            title.into(),
            "--yesno".into(),
            format!("{}\n\n[{}] / [{}]", text, ok_label, cancel_label),
        ])?;
        Ok(ok)
    }

    fn choose(
        &mut self,
        title: &str,
        text: &str,
        columns: &[&str],
        items: &[Item],
    ) -> Result<Option<String>, UiError> {
        // kdialog n'affiche qu'une colonne : les informations sont donc
        // condensées sur une seule ligne, précédée d'un rappel des colonnes.
        let mut args: Vec<String> = vec!["--title".into(), title.into(), "--menu".into()];
        let mut header = text.to_string();
        if columns.len() > 1 {
            header.push_str(&format!("\n({})", columns.join(" / ")));
        }
        args.push(header);
        for item in items {
            args.push(item.id.clone());
            args.push(clean_cell(&item.flat()));
        }
        let (ok, out) = self.run(&args)?;
        if !ok {
            return Ok(None);
        }
        if out.is_empty() {
            return Ok(None);
        }
        Ok(Some(out))
    }

    fn choose_dir(&mut self, title: &str, start: &Path) -> Result<Option<PathBuf>, UiError> {
        let (ok, out) = self.run(&[
            "--title".into(),
            title.into(),
            "--getexistingdirectory".into(),
            start.display().to_string(),
        ])?;
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
        let filter = filters
            .iter()
            .map(|(label, patterns)| format!("{} | {}", label, patterns))
            .collect::<Vec<_>>()
            .join("\n");
        let (ok, out) = self.run(&[
            "--title".into(),
            title.into(),
            "--getopenfilename".into(),
            start.display().to_string(),
            filter,
        ])?;
        if !ok || out.is_empty() {
            return Ok(None);
        }
        Ok(Some(PathBuf::from(out)))
    }

    fn prompt(&mut self, title: &str, text: &str, default: &str) -> Result<Option<String>, UiError> {
        let (ok, out) = self.run(&[
            "--title".into(),
            title.into(),
            "--inputbox".into(),
            text.into(),
            default.into(),
        ])?;
        if !ok || out.is_empty() {
            return Ok(None);
        }
        Ok(Some(out))
    }

    fn text_page(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        let path = std::env::temp_dir().join(format!("mozinrustbackup-{}.txt", std::process::id()));
        std::fs::write(&path, text).map_err(UiError::Io)?;
        let _ = self.run(&[
            "--title".into(),
            title.into(),
            "--textbox".into(),
            path.display().to_string(),
        ]);
        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    fn progress_begin(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.progress_end();
        self.cancelled = false;
        let child = Command::new(&self.program)
            .args([
                "--title",
                title,
                "--progressbar",
                text,
                "100",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        match child {
            Ok(c) => {
                self.progress = Some(c);
                Ok(())
            }
            // Pas de fenêtre de progression : ce n'est pas bloquant.
            Err(_) => Ok(()),
        }
    }

    fn progress(&mut self, fraction: f32, _text: &str) {
        let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as u32;
        if self.service.is_none() {
            if let Some(child) = self.progress.as_mut() {
                if let Some(stdout) = child.stdout.take() {
                    let mut buf = String::new();
                    use std::io::Read;
                    let mut reader = std::io::BufReader::new(stdout);
                    let _ = reader.read_to_string(&mut buf);
                    let line = buf.lines().next().unwrap_or("").trim().to_string();
                    if !line.is_empty() {
                        self.service = Some(line);
                    }
                }
            }
        }
        if let Some(service) = &self.service {
            let qdbus = crate::sys::fsops::which("qdbus6")
                .or_else(|| crate::sys::fsops::which("qdbus"))
                .or_else(|| crate::sys::fsops::which("qdbus-qt5"));
            if let Some(qdbus) = qdbus {
                let _ = Command::new(qdbus)
                    .args([service.as_str(), "/ProgressDialog", "Set", &percent.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }

    fn progress_end(&mut self) {
        if let Some(service) = self.service.take() {
            if let Some(qdbus) = crate::sys::fsops::which("qdbus6")
                .or_else(|| crate::sys::fsops::which("qdbus"))
                .or_else(|| crate::sys::fsops::which("qdbus-qt5"))
            {
                let _ = Command::new(qdbus)
                    .args([service.as_str(), "/ProgressDialog", "close"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
        if let Some(mut child) = self.progress.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.cancelled = false;
    }

    fn progress_cancelled(&mut self) -> bool {
        crate::sys::signals::interrupted() || self.cancelled
    }
}

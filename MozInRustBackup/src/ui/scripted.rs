//! Boîtes de dialogue « scriptées » : les réponses sont fournies à l'avance.
//!
//! Utilisé par les tests automatisés (et par un éventuel mode non interactif)
//! pour dérouler l'assistant complet sans intervention humaine.

use std::collections::VecDeque;
use std::path::PathBuf;

use super::{Dialogues, Item, UiError};

#[derive(Debug, Default, Clone)]
pub struct ScriptedDialogs {
    pub choices: VecDeque<Option<String>>,
    pub confirms: VecDeque<bool>,
    pub dirs: VecDeque<Option<PathBuf>>,
    pub files: VecDeque<Option<PathBuf>>,
    pub prompts: VecDeque<Option<String>>,
    /// Journal des appels, dans l'ordre (pour vérifier l'enchaînement).
    pub log: Vec<String>,
    pub progress_end_called: usize,
}

impl ScriptedDialogs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_choice(mut self, id: &str) -> Self {
        self.choices.push_back(Some(id.to_string()));
        self
    }

    pub fn with_confirm(mut self, value: bool) -> Self {
        self.confirms.push_back(value);
        self
    }

    pub fn with_dir(mut self, dir: PathBuf) -> Self {
        self.dirs.push_back(Some(dir));
        self
    }

    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.files.push_back(Some(file));
        self
    }

    pub fn with_prompt(mut self, value: &str) -> Self {
        self.prompts.push_back(Some(value.to_string()));
        self
    }

    pub fn log_contains(&self, needle: &str) -> bool {
        self.log.iter().any(|l| l.contains(needle))
    }
}

impl Dialogues for ScriptedDialogs {
    fn info(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.log.push(format!("info|{}|{}", title, text.replace('\n', " ⏎ ")));
        Ok(())
    }

    fn error(&mut self, title: &str, text: &str) {
        self.log.push(format!("error|{}|{}", title, text.replace('\n', " ⏎ ")));
    }

    fn confirm(
        &mut self,
        title: &str,
        text: &str,
        _ok_label: &str,
        _cancel_label: &str,
    ) -> Result<bool, UiError> {
        self.log.push(format!("confirm|{}|{}", title, text.replace('\n', " ⏎ ")));
        Ok(self.confirms.pop_front().unwrap_or(false))
    }

    fn choose(
        &mut self,
        title: &str,
        text: &str,
        _columns: &[&str],
        items: &[Item],
    ) -> Result<Option<String>, UiError> {
        let answer = self.choices.pop_front().unwrap_or(None);
        // Le texte et les éléments proposés sont journalisés : les tests
        // peuvent ainsi vérifier ce qui a été montré à l'utilisateur.
        let propositions = items
            .iter()
            .map(|i| i.id.clone())
            .collect::<Vec<_>>()
            .join(",");
        self.log.push(format!(
            "choose|{}|{}|{:?}|[{}]",
            title,
            text.replace('\n', " ⏎ "),
            answer,
            propositions
        ));
        Ok(answer)
    }

    fn choose_dir(&mut self, title: &str, _start: &std::path::Path) -> Result<Option<PathBuf>, UiError> {
        let answer = self.dirs.pop_front().unwrap_or(None);
        self.log.push(format!("choose_dir|{}|{:?}", title, answer));
        Ok(answer)
    }

    fn choose_file(
        &mut self,
        title: &str,
        _start: &std::path::Path,
        _filters: &[(&str, &str)],
    ) -> Result<Option<PathBuf>, UiError> {
        let answer = self.files.pop_front().unwrap_or(None);
        self.log.push(format!("choose_file|{}|{:?}", title, answer));
        Ok(answer)
    }

    fn prompt(
        &mut self,
        title: &str,
        _text: &str,
        _default: &str,
    ) -> Result<Option<String>, UiError> {
        let answer = self.prompts.pop_front().unwrap_or(None);
        self.log.push(format!("prompt|{}|{:?}", title, answer));
        Ok(answer)
    }

    fn text_page(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.log.push(format!("text_page|{}|{}", title, text.replace('\n', " ⏎ ")));
        Ok(())
    }

    fn progress_begin(&mut self, title: &str, text: &str) -> Result<(), UiError> {
        self.log.push(format!("progress_begin|{}|{}", title, text));
        Ok(())
    }

    fn progress(&mut self, fraction: f32, text: &str) {
        self.log.push(format!("progress|{:.2}|{}", fraction, text));
    }

    fn progress_end(&mut self) {
        self.progress_end_called += 1;
        self.log.push("progress_end".to_string());
    }

    fn progress_cancelled(&mut self) -> bool {
        false
    }
}

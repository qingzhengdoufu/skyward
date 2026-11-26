use super::*;

impl App {
    pub(super) fn text_context(&self) -> bool {
        match self.screen() {
            Screen::Search | Screen::Rename | Screen::NewFolder | Screen::WlanPassword | Screen::Terminal | Screen::BrowserAddress => true,
            Screen::WlanAdd => self.state.adding.is_some(),
            Screen::Editor => self.state.editor.is_some(),
            Screen::BrowserForm | Screen::Browser => {
                !self.state.browser.full_active
                    && self
                        .state
                        .browser
                        .field
                        .is_some_and(|(form, field)| self.state.browser.form.get(form).is_some_and(|form| field < form.len()))
            }
            Screen::Files => self.state.files_search,
            Screen::Decrypt => matches!(
                self.state.data,
                Data::Locked {
                    keys: Keys::Letters | Keys::Digits,
                    checking: false,
                    ..
                }
            ),
            _ => false,
        }
    }

    pub(super) fn ime_context(&self) -> bool {
        cfg!(feature = "pinyin-input")
            && self.text_context()
            && self.screen() != Screen::BrowserAddress
            && !matches!(self.state.data, Data::Locked { keys: Keys::Digits, .. } if self.screen() == Screen::Decrypt)
    }

    pub(super) fn ime_commit(&mut self, index: usize) -> Vec<Effect> {
        if !self.ime_context() || self.state.ime.spelling.is_empty() {
            return Vec::new();
        }
        let text = self.state.ime.choose(index).unwrap_or_else(|| {
            let text = self.state.ime.spelling.clone();
            self.state.ime.clear();
            text
        });
        match self.screen() {
            Screen::Terminal => {
                self.refresh();
                if self.term.as_ref().is_some_and(|term| term.running) {
                    vec![Effect::TermInput(text.into_bytes())]
                } else {
                    Vec::new()
                }
            }
            Screen::Editor => {
                if let Some(editor) = self.state.editor.as_mut() {
                    for ch in text.chars() {
                        editor.insert(ch);
                    }
                }
                self.edits += 1;
                self.refresh();
                Vec::new()
            }
            Screen::Files => {
                self.state.files_query.push_str(&text);
                self.edits += 1;
                self.find_files()
            }
            _ => {
                self.typing().push_str(&text);
                self.edits += 1;
                self.refresh();
                Vec::new()
            }
        }
    }

    pub(super) fn ime_action(&mut self, action: Action) -> Vec<Effect> {
        if !self.ime_context() {
            return Vec::new();
        }
        match action {
            Action::ImeToggle => {
                let effects = self.ime_commit(0);
                self.state.ime.enabled = !self.state.ime.enabled;
                self.refresh();
                effects
            }
            Action::ImePick(index) if index < self.state.ime.candidates.len() => self.ime_commit(index),
            Action::ImePage(next) => {
                let ime = &mut self.state.ime;
                let pages = ime.candidates.len().div_ceil(3).max(1);
                ime.page = if next {
                    (ime.page + 1) % pages
                } else {
                    (ime.page + pages - 1) % pages
                };
                self.refresh();
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn ime_key(&mut self, cap: Cap, now: u64) -> Option<Vec<Effect>> {
        if !self.ime_context() || !self.state.ime.enabled {
            return None;
        }
        // Shell shortcuts remain keys, never pinyin composition.
        if self.screen() == Screen::Terminal && self.term.as_ref().is_some_and(|term| term.ctrl) {
            self.state.ime.clear();
            return None;
        }
        match cap {
            Cap::Char(ch) if ch.is_ascii_alphabetic() || ch == '\'' => {
                if self.state.ime.spelling.len() < 32 {
                    self.state.ime.spelling.push(ch.to_ascii_lowercase());
                    self.state.ime.update();
                }
                if self.state.layer == Layer::Shifted {
                    self.state.layer = Layer::Letters;
                }
                self.refresh();
                Some(Vec::new())
            }
            Cap::Backspace if !self.state.ime.spelling.is_empty() => {
                self.state.ime.spelling.pop();
                self.state.ime.update();
                self.refresh();
                Some(Vec::new())
            }
            Cap::Space | Cap::Enter if !self.state.ime.spelling.is_empty() => Some(self.ime_commit(0)),
            Cap::Char(_) if !self.state.ime.spelling.is_empty() => {
                let mut effects = self.ime_commit(0);
                effects.extend(self.native_key(cap, now));
                Some(effects)
            }
            _ => None,
        }
    }

    pub(super) fn native_key(&mut self, cap: Cap, now: u64) -> Vec<Effect> {
        match self.screen() {
            Screen::Terminal => self.term_typed(cap),
            Screen::Editor => {
                self.edit_typed(cap);
                Vec::new()
            }
            _ => self.type_key(cap, now),
        }
    }
}

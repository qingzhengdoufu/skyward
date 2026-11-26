use super::*;

/// Why an entry was not renamed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameRefused {
    /// Another entry in the folder has the name, as the folder compares
    /// names: shared storage takes names in any case.
    Taken,
    /// What went wrong, as the system says it.
    Failed(String),
}

impl App {
    pub fn folder_created(&mut self, now: u64) {
        if self.screen() == Screen::NewFolder {
            let _ = self.back(now);
        }
        self.state.new_name.clear();
        self.refresh();
    }
    /// How renaming the entry the file page shows went: done, the folder
    /// comes back with the entry under its new name; refused, why.
    pub fn renamed(&mut self, outcome: Result<(), RenameRefused>, now: u64) {
        match outcome {
            Ok(()) => {
                self.leave(true);
                while matches!(self.screen(), Screen::Rename | Screen::FileInfo) {
                    self.stack.pop();
                }
                self.state.file = None;
                self.settle_page();
                self.refresh();
                self.say(self.state.lang.pick(texts::toast::RENAMED), now);
            }
            Err(RenameRefused::Taken) => self.say_taken(now),
            Err(RenameRefused::Failed(why)) => self.report(&why, now),
        }
    }
}

impl App {
    /// The entry the file page shows was to have the permission bits
    /// `wanted`: those it has now, or why it has not.
    pub fn mode_set(&mut self, wanted: u32, outcome: Result<u32, String>, now: u64) {
        let l = self.state.lang;
        let bits = match outcome {
            Ok(bits) => bits,
            Err(why) => return self.report(&texts::toast::permissions_failed(l, why), now),
        };
        let entry = self.state.file.as_mut().expect("the permissions page shows an entry");
        entry.mode = (entry.mode & !0o7777) | bits;
        let name = entry.name.clone();
        if let Ok(entries) = &mut self.state.files.entries
            && let Some(listed) = entries.iter_mut().find(|e| e.name == name)
        {
            listed.mode = (listed.mode & !0o7777) | bits;
        }
        if bits != wanted {
            self.say(l.pick(texts::toast::PERMISSIONS_UNSUPPORTED), now);
        }
        self.refresh();
    }
}

impl App {
    /// What the file at `path` holds, read to edit it: none if it holds
    /// more than [`editor::MAX_BYTES`]; or why it could not be read. Text
    /// opens on the editor page, wrapped to its card; anything else is
    /// turned down, saying why.
    pub fn text_opened(&mut self, f: &Fonts, path: String, read: Result<Option<Vec<u8>>, String>, now: u64) {
        let l = self.state.lang;
        let bytes = match read {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return self.say(l.pick(texts::toast::TOO_BIG_TO_EDIT), now),
            Err(why) => return self.report(&texts::toast::read_failed(l, why), now),
        };
        let Some(text) = String::from_utf8(bytes).ok().filter(|text| !text.contains('\0')) else {
            return self.say(l.pick(texts::toast::NOT_TEXT), now);
        };
        let l = editor::layout(&self.theme(), f);
        self.state.editor = Some(Editor::new(path, &text, l.cols as usize, l.rows as usize));
        self.state.layer = Layer::Letters;
        self.unsaved_warned = None;
        self.open(Screen::Editor);
    }
}

impl App {
    /// How writing the file the editor page shows went: done, it stands as
    /// saved; or why not.
    pub fn text_saved(&mut self, outcome: Result<(), String>, now: u64) {
        match outcome {
            Ok(()) => {
                self.state.editor.as_mut().expect("the editor page shows a file").saved();
                self.refresh();
                self.say(self.state.lang.pick(texts::toast::SAVED), now);
            }
            Err(why) => self.report(&texts::toast::save_failed(self.state.lang, why), now),
        }
    }
}

impl App {
    /// The name typed for an entry is another entry's in the folder.
    pub(super) fn say_taken(&mut self, now: u64) {
        let said = texts::toast::name_taken(self.state.lang, &self.state.new_name);
        self.say(&said, now);
    }
}

impl App {
    /// Where the entry the file page shows is, or `None` when no page is
    /// describing one any more.
    ///
    /// The entry a page describes is taken away the moment work on it
    /// starts, so a second touch that arrived from a frame drawn before
    /// that finds nothing here. Saying so leaves the phone where it is;
    /// ending the run over it would take the phone back to the home page
    /// with the work it just began still going.
    pub(super) fn entry_path(&self) -> Option<String> {
        let entry = self.state.file.as_ref()?;
        Some(match &self.state.file_path {
            Some(path) => path.clone(),
            None => self.state.files.inside(&entry.name),
        })
    }
}

impl App {
    /// A key of the keyboard on the editor page: a character or a space
    /// goes in at the cursor, Enter breaks the line there and backspace
    /// takes away what is before it; the layer keys change the keyboard.
    pub(super) fn edit_typed(&mut self, cap: Cap) {
        let layer = self.state.layer;
        let ed = self.state.editor.as_mut().expect("the editor page shows a file");
        match cap {
            Cap::Char(ch) if layer == Layer::Shifted => {
                ed.insert(ch.to_ascii_uppercase());
                self.state.layer = Layer::Letters;
            }
            Cap::Char(ch) => ed.insert(ch),
            Cap::Space => ed.insert(' '),
            Cap::Backspace => ed.backspace(),
            Cap::Enter => ed.newline(),
            Cap::Shift => {
                self.state.layer = if layer == Layer::Shifted { Layer::Letters } else { Layer::Shifted };
            }
            Cap::Show(shown) => self.state.layer = shown,
        }
        self.edits += 1;
        if self.state.layer == layer {
            self.refresh_text();
        } else {
            self.refresh();
        }
    }
}

impl App {
    /// The editor's text or cursor changed: draw again what lies above the
    /// keyboard, which stays as it was.
    pub(super) fn refresh_text(&mut self) {
        let t = self.theme();
        self.refresh_rows(0, (t.m.h as f32 - widgets::keyboard_height(&t)).floor() as u32);
    }
}

impl App {
    /// A tap at page `x`, `y` on the editor's card puts the cursor between
    /// the characters nearest it.
    pub(super) fn place_caret(&mut self, f: &Fonts, x: f32, y: f32) {
        let l = editor::layout(&self.theme(), f);
        if !l.card.contains(x, y) {
            return;
        }
        let ed = self.state.editor.as_mut().expect("the editor page shows a file");
        let row = (((y - l.y0) / l.pitch).floor().max(0.0) as usize).min(l.rows as usize - 1);
        ed.place(ed.top() + row, (x - l.x0) / l.cell);
        self.edits += 1;
        self.refresh_text();
    }
}

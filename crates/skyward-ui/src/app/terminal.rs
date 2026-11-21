use super::*;

/// How the terminal page's shell ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermEnd {
    /// It exited with this code.
    Exited(i32),
    /// This signal stopped it.
    Signalled(i32),
    /// Neither, as the system says it.
    Other(String),
}

impl App {
    /// How many rows and columns the terminal's screen holds.
    pub fn term_size(&self, f: &Fonts) -> (u16, u16) {
        let t = self.theme();
        let l = term::layout(&t, f, t.m.content_top);
        (l.rows, l.cols)
    }
}

impl App {
    /// A shell has started for the terminal page, on a blank screen of
    /// `rows` by `cols`.
    pub fn term_started(&mut self, rows: u16, cols: u16) {
        self.term = Some(Term::new(rows, cols));
        self.refresh();
    }
}

impl App {
    /// What the shell wrote.
    pub fn term_output(&mut self, bytes: &[u8]) {
        let term = self.term.as_mut().expect("only a shell that started writes");
        term.feed(bytes);
        if self.screen() == Screen::Terminal {
            self.refresh();
        }
    }
}

impl App {
    /// The shell has ended as `end` says; its screen says so, and that
    /// Enter starts another.
    pub fn term_ended(&mut self, end: TermEnd) {
        let (l, term) = (self.state.lang, self.term.as_mut().expect("only a shell that started ends"));
        let said = texts::terminal::restart(l, fmt::term_end(l, &end));
        term.feed(format!("\r\n[{said}]\r\n").as_bytes());
        term.running = false;
        if self.screen() == Screen::Terminal {
            self.refresh();
        }
    }
}

impl App {
    /// A key of the keyboard on the terminal page. The layer keys change
    /// the keyboard; every other key goes to the shell, and once the shell
    /// has ended, Enter starts another.
    pub(super) fn term_typed(&mut self, cap: Cap) -> Vec<Effect> {
        match cap {
            Cap::Shift => {
                self.state.layer = if self.state.layer == Layer::Shifted {
                    Layer::Letters
                } else {
                    Layer::Shifted
                };
            }
            Cap::Show(layer) => self.state.layer = layer,
            _ => {}
        }
        let shifted = self.state.layer == Layer::Shifted;
        if shifted && matches!(cap, Cap::Char(_)) {
            self.state.layer = Layer::Letters;
        }
        self.refresh();
        let Some(term) = self.term.as_mut() else { return Vec::new() };
        if !term.running {
            return if cap == Cap::Enter {
                vec![Effect::StartShell(self.shell_folder.clone())]
            } else {
                Vec::new()
            };
        }
        let bytes = term.typed(cap, shifted);
        if bytes.is_empty() {
            Vec::new()
        } else {
            vec![Effect::TermInput(bytes)]
        }
    }
}

impl App {
    /// Scroll the terminal's screen or the editor's text as a finger goes
    /// from `last` to `y`: a row for each row's height.
    pub(super) fn scroll_history(&mut self, last: f32, y: f32, carry: f32, pitch: f32) -> Drag {
        let carry = carry + (y - last);
        let rows = (carry / pitch).trunc();
        if rows != 0.0 {
            self.scroll_rows(rows as isize);
        }
        Drag::History {
            last: y,
            carry: carry - rows * pitch,
            pitch,
        }
    }
}

impl App {
    /// Scroll as a finger going down `rows` rows' heights does, or up for
    /// fewer than none: the terminal's screen back to what the shell wrote
    /// before, the editor's text to the lines before.
    pub(super) fn scroll_rows(&mut self, rows: isize) {
        if self.screen() == Screen::Editor {
            self.state.editor.as_mut().expect("the editor page shows a file").scroll(-rows);
            self.refresh_text();
        } else if let Some(term) = self.term.as_mut() {
            term.scroll(rows);
            self.refresh();
        }
    }
}

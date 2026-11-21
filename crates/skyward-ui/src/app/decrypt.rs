use super::*;

/// What work that needs the data partition is told while it is being
/// opened.
pub const DATA_OPENING: Text = texts::toast::DATA_OPENING;

/// What work on the data partition is told once it has been erased whole.
pub const DATA_ERASED: Text = texts::data::ERASED;

/// What work on internal storage is told before the data partition is
/// decrypted.
pub const DECRYPT_FIRST: Text = texts::toast::DECRYPT_FIRST;

/// Which control answers a user's screen lock, by the kind of lock it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keys {
    Pattern,
    Digits,
    Letters,
}

/// Where the data partition stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Data {
    /// Not opened; the decrypt page opens it as it shows.
    #[default]
    Closed,
    /// Being opened as far as it opens with nothing a user knows, which is
    /// also when each user's screen lock is read.
    Opening,
    /// Open, with this user's own files waiting on their screen lock,
    /// answered with `keys`; `checking` while what they entered is checked.
    Locked { user: u32, keys: Keys, checking: bool },
    /// Open, every user's own files with it.
    Open,
    /// Not opened, and why.
    Failed(String),
    /// Erased whole: nothing opens until the system makes it afresh, as it
    /// next starts.
    Erased,
}

impl Data {
    /// Whether the owner's internal storage can be read: the partition is
    /// open, and so are the owner's own files, which open first.
    pub fn storage_open(&self) -> bool {
        match self {
            Data::Open => true,
            Data::Locked { user, .. } => *user != 0,
            _ => false,
        }
    }
}

impl App {
    /// The data partition is open: the owner's own files with it, or
    /// `waiting`, the user the page asks a screen lock for and the control
    /// it is answered with, still closed. `others` are the users beyond it
    /// whose own files are still closed, offered by name rather than asked
    /// for: the owner came to open the owner's files, and a second space
    /// does not hold the recovery at its prompt.
    pub fn data_waiting(&mut self, waiting: Option<(u32, Keys)>, others: Vec<(u32, Keys)>) {
        self.state.data = match waiting {
            Some((user, keys)) => Data::Locked {
                user,
                keys,
                checking: false,
            },
            None => Data::Open,
        };
        self.state.others = others;
        self.state.code.clear();
        self.refresh();
    }
}

impl App {
    /// The data partition did not open, and why.
    pub fn data_failed(&mut self, why: String) {
        self.state.data = Data::Failed(why);
        self.refresh();
    }
}

impl App {
    /// What the user entered did not open their own files: the page asks
    /// again and says why.
    pub fn unlock_failed(&mut self, why: String, now: u64) {
        if let Data::Locked { checking, .. } = &mut self.state.data {
            *checking = false;
        }
        self.refresh();
        self.report(&why, now);
    }
}

impl App {
    /// Open the data partition, the decrypt page saying so meanwhile.
    pub(super) fn open_data(&mut self) -> Vec<Effect> {
        self.state.data = Data::Opening;
        self.refresh();
        vec![Effect::OpenData]
    }
}

impl App {
    /// Send what was entered at the screen lock of the user whose own
    /// files wait. It is taken as it is sent, so the dots clear while it is
    /// checked, and nothing more is entered until it has been.
    pub(super) fn unlock(&mut self, code: String) -> Vec<Effect> {
        let Data::Locked {
            user,
            keys,
            checking: false,
        } = self.state.data
        else {
            // Automatic unlocking can finish before a held key or pattern lifts.
            return Vec::new();
        };
        self.state.data = Data::Locked {
            user,
            keys,
            checking: true,
        };
        self.refresh();
        vec![Effect::Unlock { user, code }]
    }
}

impl App {
    /// Add the dot under the finger to the pattern being drawn.
    pub(super) fn trace(&mut self, t: &Theme, x: f32, y: f32) {
        let Some((cx, cy, span)) = self.frame.as_ref().and_then(|fr| fr.pattern) else {
            return;
        };
        let reach = t.m.dp(widgets::PATTERN_REACH_DP);
        let dots = widgets::pattern_dots(cx, cy, span);
        if let Some(i) = dots.iter().position(|&(dx, dy)| (dx - x).hypot(dy - y) < reach) {
            self.join(t, i);
        }
    }
}

impl App {
    /// Join dot `i` to the pattern being drawn, after the dot it passes
    /// straight over on its way from the last one, as the system's pattern
    /// lock joins that one too. A dot joined once is not joined again.
    pub(super) fn join(&mut self, t: &Theme, i: usize) {
        let drawn = &mut self.state.pattern;
        if drawn.contains(&i) {
            return;
        }
        if let Some(&last) = drawn.last() {
            let (row, col) = ((last / 3) as i32, (last % 3) as i32);
            let (rows, cols) = ((i / 3) as i32 - row, (i % 3) as i32 - col);
            let row = if rows.abs() == 2 && cols.abs() != 1 {
                row + rows.signum()
            } else {
                row
            };
            let col = if cols.abs() == 2 && rows.abs() != 1 {
                col + cols.signum()
            } else {
                col
            };
            let over = (row * 3 + col) as usize;
            if !drawn.contains(&over) {
                drawn.push(over);
            }
        }
        drawn.push(i);
        self.repaint_pattern(t);
    }
}

impl App {
    pub(super) fn repaint_pattern(&mut self, t: &Theme) {
        let Some(fr) = self.frame.as_mut() else { return };
        let Some((cx, cy, span)) = fr.pattern else { return };
        let r = span / 2.0 + t.m.dp(16.0);
        fr.canvas.fill_rect(cx - r, cy - r, r * 2.0, r * 2.0, t.pal.bg);
        widgets::pattern_lock(&mut fr.canvas, t, cx, cy, span, &self.state.pattern);
        self.layer = None;
        self.dirty = true;
    }
}

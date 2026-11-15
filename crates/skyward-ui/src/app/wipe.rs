use super::*;

/// What the wipe page erases: everything a factory reset erases, or any of
/// the parts after it on their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wipe {
    All,
    /// Each app's own data, for every user.
    AppData,
    /// The files users share between apps.
    Media,
    /// Each app's caches, for every user.
    AppCaches,
    SystemCaches,
    /// What ART compiled ahead of time.
    Art,
    /// The cache partition, on phones that have one.
    Cache,
    Logs,
    /// The updates the system downloaded.
    Updates,
}

/// What the wipe page offers, in the order it lists them.
pub const WIPES: [Wipe; 9] = [
    Wipe::All,
    Wipe::AppData,
    Wipe::Media,
    Wipe::AppCaches,
    Wipe::SystemCaches,
    Wipe::Art,
    Wipe::Logs,
    Wipe::Updates,
    Wipe::Cache,
];

/// A part of the phone that erasing goes through, in the order it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The data partition, whole.
    Data,
    Cache,
    /// /metadata, which holds the key the data partition is opened with.
    Metadata,
    /// The parts of the data partition the wipe page names.
    AppData,
    Media,
    AppCaches,
    SystemCaches,
    Art,
    Logs,
    Updates,
}

/// Erasing as its page shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Erase {
    /// Everything users keep on the phone, as a factory reset erases it.
    pub reset: bool,
    /// The system asked for it before the recovery started; once it is
    /// done, the phone leaves on its own.
    pub asked: bool,
    pub parts: Vec<Part>,
    /// How many of `parts` are erased; the one after them is under way.
    pub done: usize,
    /// How far the part under way has come, entries of how many, where it
    /// tells.
    pub progress: Option<(u64, u64)>,
    /// How erasing ended: done, or why it stopped.
    pub outcome: Option<Result<(), EraseStop>>,
}

/// Why erasing did not start, or stopped short.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EraseStop {
    /// An update has to be finished by the system first.
    Update(Hold),
    /// The system does not make an emptied data partition afresh as it
    /// starts, so it is not erased this way.
    NotRebuilt,
    /// The phone has no cache partition.
    NoCache,
    /// The data partition is erased already, and holds nothing until the
    /// system makes it afresh.
    Erased,
    /// The data partition did not open, so nothing in it was erased: why.
    Closed(String),
    /// These users' own files are locked, and what was chosen is partly in
    /// them: nothing was erased.
    Locked(Vec<u32>),
    /// Erasing this part went wrong: what the system said.
    Failed(Part, String),
}

impl App {
    /// Erasing has begun, going through `parts`: its page is the only one
    /// until it ends. `reset` for a factory reset, `asked` when the system
    /// asked for it before the recovery started.
    pub fn erase_started(&mut self, reset: bool, asked: bool, parts: Vec<Part>) {
        self.state.erase = Erase {
            reset,
            asked,
            parts,
            done: 0,
            progress: None,
            outcome: None,
        };
        self.stack = vec![(Screen::Erase, 0.0, Focus::Unset)];
        self.settle_page();
        self.refresh();
    }
}

impl App {
    /// Erasing has come to `part`.
    pub fn erase_at(&mut self, part: Part) {
        let erase = &mut self.state.erase;
        erase.done = erase
            .parts
            .iter()
            .position(|&p| p == part)
            .expect("erasing comes only to the parts it set out with");
        erase.progress = None;
        self.refresh();
    }
}

impl App {
    /// The part under way has come `done` entries of `total`.
    pub fn erase_progress(&mut self, done: u64, total: u64) {
        self.state.erase.progress = Some((done, total));
        self.refresh();
    }
}

impl App {
    /// Erasing has ended. What comes next is the user's to choose and the
    /// panel lights for it, unless the system asked for the erasing and it
    /// went well: the phone then leaves on its own. A data partition
    /// erased whole opens no more until the system makes it afresh.
    pub fn erase_ended(&mut self, outcome: Result<(), EraseStop>, now: u64) {
        let erase = &mut self.state.erase;
        if outcome.is_ok() {
            erase.done = erase.parts.len();
        }
        erase.progress = None;
        let data_gone = erase.parts.iter().position(|&p| p == Part::Data).is_some_and(|k| k < erase.done);
        if outcome.is_err() || !erase.asked {
            self.wake(now);
        }
        if data_gone {
            self.state.data = Data::Erased;
        }
        self.state.erase.outcome = Some(outcome);
        self.refresh();
    }
}

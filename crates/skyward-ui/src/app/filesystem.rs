use super::*;

/// The data partition's name, as the filesystem page lists it.
pub const DATA_PARTITION: &str = "data";

/// A partition the filesystem page lists, as the runtime read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsRow {
    /// Its name, such as data or metadata.
    pub name: String,
    pub mount_point: String,
    /// The filesystem it holds, where that can be read: the data
    /// partition's only once it is open.
    pub filesystem: Option<String>,
    /// Its size in bytes.
    pub size: u64,
    /// The filesystems the phone's own table lists for it, besides the one
    /// it holds, that the recovery can make.
    pub others: Vec<String>,
}

/// What the filesystem page does to the partition it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FsAction {
    /// Check the whole of the filesystem and fix what is wrong.
    Repair,
    /// Make the partition afresh as this filesystem.
    Remake(String),
}

/// A choice on the page of one partition, as a control carries it: the
/// filesystem to make it afresh as goes by its place among the others it
/// may hold, a control's action being copied freely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsChoice {
    Repair,
    Remake(usize),
}

/// Work on whole partitions one after another, as its page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartWork {
    pub kind: PartKind,
    /// The system asked for it before the recovery started: done, the phone
    /// leaves on its own.
    pub asked: bool,
    /// The partitions it goes through, once known, and where each stands.
    pub items: Vec<(String, ItemState)>,
    /// What the program that worked on the last of them said.
    pub output: Vec<String>,
    /// How it ended: done, or why it stopped before going through the
    /// partitions.
    pub outcome: Option<Result<(), PartStop>>,
}

/// What partition work does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartKind {
    /// Erase the partitions a wipe package lists.
    Disable,
    /// Undo the sharing of blocks in ext4 partitions.
    Unshare,
    Repair,
    /// Make the partition afresh as this filesystem.
    Remake(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemState {
    Waiting,
    Working,
    Ended(ItemEnd),
}

/// How a partition's work ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemEnd {
    Erased,
    Unshared,
    /// It holds this filesystem, none where blkid knows of none, and shares
    /// no blocks.
    NotShared(String),
    /// Nothing was wrong with the filesystem.
    Clean,
    Fixed,
    Made,
    /// The work went wrong: why, where what the program working on it said
    /// does not tell it.
    Failed(Option<String>),
}

/// Why partition work stopped before going through the partitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartStop {
    /// The wipe package is not one this phone may be erased by.
    Refused(Refusal),
    /// The program it takes is only on this partition of the installed
    /// system, which is not mounted.
    Unmounted(&'static str),
    Failed(String),
}

impl App {
    /// The partitions for the filesystem page, as the runtime read them,
    /// or why it could not.
    pub fn set_filesystems(&mut self, rows: Result<Vec<FsRow>, String>) {
        self.state.filesystems = Some(rows);
        self.refresh();
    }
}

impl App {
    /// The partition the page of one shows.
    pub(super) fn fs_row(&self) -> Option<&FsRow> {
        self.state.filesystems.as_ref()?.as_ref().ok()?.get(self.state.fs_chosen)
    }
}

impl App {
    /// Work on whole partitions has begun. Work the system asked for, which
    /// `asked` says, has its page as the only one; the filesystem page's
    /// opens over it, to go back to once the work has ended.
    pub fn part_work_started(&mut self, kind: PartKind, asked: bool) {
        self.state.part_work = Some(PartWork {
            kind,
            asked,
            items: Vec::new(),
            output: Vec::new(),
            outcome: None,
        });
        match asked {
            true => {
                self.stack = vec![(Screen::PartWork, 0.0, Focus::Unset)];
                self.settle_page();
            }
            false => self.open(Screen::PartWork),
        }
        self.refresh();
    }
}

impl App {
    /// The partitions the work goes through.
    pub fn part_listed(&mut self, names: Vec<String>) {
        let work = self.state.part_work.as_mut().expect("only work under way lists its partitions");
        work.items = names.into_iter().map(|name| (name, ItemState::Waiting)).collect();
        self.refresh();
    }
}

impl App {
    /// The partition at `index` is under way.
    pub fn part_at(&mut self, index: usize) {
        let work = self.state.part_work.as_mut().expect("only work under way moves on");
        work.items[index].1 = ItemState::Working;
        self.refresh();
    }
}

impl App {
    /// The partition at `index` ended as `end`, the program working on it
    /// having said `said`.
    pub fn part_ended(&mut self, index: usize, end: ItemEnd, said: &str) {
        let work = self.state.part_work.as_mut().expect("only work under way ends a partition");
        work.items[index].1 = ItemState::Ended(end);
        let lines: Vec<String> = said
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        if !lines.is_empty() {
            work.output = lines[lines.len().saturating_sub(SCRIPT_LINES)..].to_vec();
        }
        self.refresh();
    }
}

impl App {
    /// The work has ended. What comes next is the user's to choose, and
    /// the panel lights for it, unless the system asked for the work and it
    /// went through every partition: the phone then leaves on its own. The
    /// data partition, worked on, is closed now.
    pub fn part_work_ended(&mut self, outcome: Result<(), PartStop>, now: u64) {
        let work = self.state.part_work.as_mut().expect("only work under way ends");
        let failed = outcome.is_err() || work.items.iter().any(|(_, s)| matches!(s, ItemState::Ended(ItemEnd::Failed(_))));
        let lit = failed || !work.asked;
        let data = work.kind == PartKind::Repair && work.items.iter().any(|(name, _)| name == DATA_PARTITION);
        work.outcome = Some(outcome);
        if data && self.state.data != Data::Erased {
            self.state.data = Data::Closed;
        }
        if lit {
            self.wake(now);
        }
        self.refresh();
    }
}

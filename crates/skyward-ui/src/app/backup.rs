use super::*;

/// What a backup can take, in the order the backup page lists it.
pub const BACKUP_PARTS: [Text; 4] = [
    ["启动分区", "啟動分割區", "Boot partitions"],
    ["系统分区", "系統分割區", "System partitions"],
    ["data 分区", "data 分割區", "data partition"],
    ["基带与 EFS", "基頻與 EFS", "Modem and EFS"],
];

/// A backup found in a place's Skyward/backups folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupEntry {
    pub place: Place,
    /// Its folder's name: when it was begun.
    pub name: String,
    /// What it holds, as its manifest records it, or why that cannot be
    /// read: a backup that did not finish has none.
    pub held: Result<Held, String>,
    pub image: Option<ImageCopy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageCopy {
    pub partition: String,
    pub sha256: String,
}

/// What a backup holds, and where it was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    /// When it was made, in Unix seconds.
    pub made: i64,
    /// The bytes its files take.
    pub size: u64,
    /// Which of [`BACKUP_PARTS`] it holds.
    pub parts: [bool; 4],
    /// The phone it was made on, by its codename, the system it ran, as
    /// its version reads, and the slot it was made from.
    pub device: String,
    pub system: Option<String>,
    pub slot: Option<Slot>,
}

/// Backing up or restoring, as its page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupJob {
    /// Restoring a backup, rather than making one.
    pub restore: bool,
    /// Where the backup goes, or comes from.
    pub place: Place,
    /// Each part is read back to check it: all once they are written, for a
    /// backup; all before any is written, for a restore.
    pub checks: bool,
    /// The parts it goes through, by name with their bytes; none while it
    /// still measures them.
    pub plan: Vec<(String, u64)>,
    pub at: Option<BackupAt>,
    pub outcome: Option<BackupEnd>,
    /// Stopping was asked for and is under way.
    pub stopping: bool,
}

/// The part under way, by its place in the plan, and how far it has come:
/// bytes done of how many in this pass over it, and whether this is the
/// pass that checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackupAt {
    pub item: usize,
    pub done: u64,
    pub of: u64,
    pub checking: bool,
}

/// How backing up or restoring ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackupEnd {
    ImageSaved(String),
    /// A backup made, in the folder of this name.
    Saved(String),
    /// A backup restored, but for what it held in the folders of users gone
    /// since, which only the system makes: where those were.
    Restored {
        unmade: Vec<String>,
    },
    /// Stopped as asked: a backup stopped leaves nothing of itself, and a
    /// restore stops only while nothing is written.
    Stopped,
    Refused(BackupRefusal),
    /// What went wrong, as the system says it.
    Failed(String),
}

/// Why backing up or restoring did not start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackupRefusal {
    /// Too little room where the backup goes: the bytes it takes, and the
    /// bytes free.
    Room { needed: u64, free: u64 },
    /// An update is in flight, and the slot's partitions in super are its
    /// to change.
    Update(SnapshotState),
    /// The backup was made on another phone, by its codename.
    OtherPhone(String),
    /// This partition is not the size it was backed up at, as after an
    /// update.
    Resized(String),
    /// The data partition has been made afresh since the backup, as a
    /// factory reset makes it: the files backed up would not open in it.
    DataReset,
}

impl App {
    /// What was found in the places looked in for backups: the newest
    /// first, by when each was made, and those that cannot be read after
    /// them, by their folder's name, which is when each was begun.
    pub fn set_backups(&mut self, mut backups: Vec<BackupEntry>) {
        let key = |e: &BackupEntry| (e.held.is_ok(), e.held.as_ref().map_or(0, |h| h.made), e.name.clone());
        backups.sort_by_key(|e| std::cmp::Reverse(key(e)));
        self.state.backups = backups;
        self.refresh();
    }
}

impl App {
    /// Look for backups in the places that can be read now.
    pub(super) fn find_backups(&self) -> Effect {
        Effect::FindBackups {
            storage: self.state.place_open(Place::Storage),
            usb: self.state.place_open(Place::UsbDrive),
        }
    }
}

impl App {
    /// Backing up, or restoring, has begun: its page shows how far it comes.
    /// `checks` when each part is read back to check it.
    pub fn backup_started(&mut self, restore: bool, place: Place, checks: bool) {
        self.state.backup_job = Some(BackupJob {
            restore,
            place,
            checks,
            plan: Vec::new(),
            at: None,
            outcome: None,
            stopping: false,
        });
        self.open(Screen::BackupJob);
    }
}

impl App {
    /// The parts the work goes through, once they are measured.
    pub fn backup_plan(&mut self, plan: Vec<(String, u64)>) {
        self.state.backup_job.as_mut().expect("only work under way is measured").plan = plan;
        if self.screen() == Screen::BackupJob {
            self.refresh();
        }
    }
}

impl App {
    /// How far the work has come.
    pub fn backup_at(&mut self, at: BackupAt) {
        self.state.backup_job.as_mut().expect("only work under way moves on").at = Some(at);
        if self.screen() == Screen::BackupJob {
            self.refresh();
        }
    }
}

impl App {
    /// Backing up or restoring has ended: its page says how, lit for the
    /// user to read.
    pub fn backup_ended(&mut self, end: BackupEnd, now: u64) {
        let job = self.state.backup_job.as_mut().expect("only work under way ends");
        job.stopping = false;
        job.outcome = Some(end);
        self.wake(now);
        self.refresh();
    }
}

impl App {
    /// How deleting the backup the restore page shows went: deleted, the
    /// page goes, and the backups left are looked for; not, why.
    pub fn backup_deleted(&mut self, outcome: Result<(), String>, now: u64) -> Vec<Effect> {
        match outcome {
            Ok(()) => {
                self.state.restore = None;
                self.state.asking_delete = false;
                if self.screen() == Screen::RestoreBackup {
                    self.leave(true);
                    self.stack.pop();
                    self.settle_page();
                }
                self.refresh();
                self.say(self.state.lang.pick(["已删除", "已刪除", "Deleted"]), now);
                vec![self.find_backups()]
            }
            Err(why) => {
                self.report(
                    &tr!(self.state.lang, "删除失败：{why}", "刪除失敗：{why}", "Couldn't delete it: {why}"),
                    now,
                );
                Vec::new()
            }
        }
    }
}

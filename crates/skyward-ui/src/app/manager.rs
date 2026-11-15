use super::*;

/// Where the USB drive plugged in is shown.
pub const USB_DRIVE: &str = "/mnt/usb";

/// What a USB drive plugged in that cannot be read is said to be, and what
/// is asked for while none is plugged in.
pub const USB_DRIVE_UNREADABLE: Text = ["无法读取该 U 盘", "無法讀取這個 USB 隨身碟", "This USB drive can't be read"];

pub const USB_DRIVE_WANTED: Text = [
    "请通过 OTG 连接 U 盘",
    "請透過 OTG 連接 USB 隨身碟",
    "Connect a USB drive through OTG",
];

/// Where a file to install is chosen from, and where backups go and are
/// found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The owner's internal storage, which opens with the data partition.
    Storage,
    /// A USB drive plugged into the port, which the runtime shows read
    /// only.
    UsbDrive,
}

/// The USB drive plugged in, as the runtime found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsbDrive {
    /// Shown, for a file on it to be chosen.
    Shown,
    /// Nothing on it could be read: why.
    Unreadable(String),
}

/// What can be done with an entry picked in the file manager.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOp {
    Copy,
    Move,
}

/// An entry picked to copy or move, while the file manager looks for the
/// folder it goes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clip {
    pub op: FileOp,
    /// Where the entry is.
    pub path: String,
}

/// What to do with an entry of the same name already where one goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClashChoice {
    /// The new entry takes the old one's place.
    Replace,
    /// The old entry stays, and the new one is left out.
    Skip,
    /// The new entry gets a name of its own beside the old one.
    KeepBoth,
}

/// Work on files as its page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileTask {
    Copy { path: String, into: String },
    Move { path: String, into: String },
    Delete { path: String },
}

/// How work on files ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobEnd {
    Done,
    /// Stopped as asked, with what was done by then left as it is.
    Stopped,
    Failed(String),
}

/// Work on files under way: what it is, how far it has come, and how it
/// ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileJob {
    pub task: FileTask,
    /// Bytes and entries, each as done and out of how many; none while the
    /// work still measures what it covers.
    pub progress: Option<((u64, u64), (u64, u64))>,
    pub outcome: Option<JobEnd>,
    /// Stopping was asked for and is under way.
    pub stopping: bool,
}

/// An entry of a folder, as the file manager lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    pub name: String,
    pub kind: FileKind,
    /// Bytes; for a link, the link's own.
    pub size: u64,
    /// Last changed, in Unix seconds.
    pub modified: i64,
    /// Type and permission bits, as `st_mode` holds them.
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// A folder, and how many entries it holds when it can be read.
    Folder {
        items: Option<usize>,
    },
    File,
    /// A symbolic link: where it points, and whether that is a folder.
    Link {
        target: String,
        folder: bool,
    },
    /// A device, socket or pipe.
    Special,
}

/// A folder as a list of its entries shows it: where it is, what is in it
/// as the runtime read it or why it could not, and when it was read, in
/// Unix seconds: dates in the list are told against that day.
#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    pub path: String,
    pub entries: Result<Vec<FileRow>, String>,
    pub listed: i64,
    /// Which entries the rows show, in the order they are in: row `i` of
    /// the list is entry `shown[i]`. Every entry while nothing is typed.
    pub(super) shown: Vec<usize>,
    /// The folders left for one inside them, outermost first, with how far
    /// each was scrolled.
    pub(super) left: Vec<(String, f32)>,
}

impl Folder {
    /// The folder at `path`, not read yet.
    pub fn new(path: &str) -> Folder {
        Folder {
            path: path.into(),
            entries: Ok(Vec::new()),
            listed: 0,
            shown: Vec::new(),
            left: Vec::new(),
        }
    }
    /// The entry at this place in the list, or none when the list no longer
    /// has a row there — a listing that shrank between a tap and the tap
    /// landing shows nothing rather than ending the recovery.
    pub fn entry(&self, i: usize) -> Option<&FileRow> {
        let entries = self.entries.as_ref().ok()?;
        entries.get(*self.shown.get(i)?)
    }
    /// The entries the rows show, in the order they are in.
    pub fn shown_entries(&self) -> Vec<&FileRow> {
        match &self.entries {
            Ok(entries) => self.shown.iter().filter_map(|i| entries.get(*i)).collect(),
            Err(_) => Vec::new(),
        }
    }
    /// Keep the entries `query` picks out, in the order they were read in;
    /// a query with nothing in it keeps all of them. Matched without case
    /// and anywhere in the name, as the stock file manager matches it.
    pub fn keep(&mut self, query: &str) {
        self.shown.clear();
        let Ok(entries) = &self.entries else { return };
        let needle = query.trim().to_lowercase();
        for (i, e) in entries.iter().enumerate() {
            if needle.is_empty() || e.name.to_lowercase().contains(&needle) {
                self.shown.push(i);
            }
        }
    }
    /// Where the entry of this name in the folder is.
    pub(super) fn inside(&self, name: &str) -> String {
        std::path::Path::new(&self.path).join(name).to_string_lossy().into_owned()
    }
    /// The folder this one is in; none for the root.
    pub(super) fn up(&self) -> Option<String> {
        std::path::Path::new(&self.path).parent().map(|p| p.to_string_lossy().into_owned())
    }
    /// The folder this many levels below the root, by its place in the
    /// path shown over the list; none for this folder itself.
    pub(super) fn ancestor(&self, depth: usize) -> Option<String> {
        let path = std::path::Path::new(&self.path);
        let names: Vec<_> = path.components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect();
        if depth >= names.len() {
            return None;
        }
        let mut up = if path.has_root() {
            std::path::PathBuf::from("/")
        } else {
            std::path::PathBuf::new()
        };
        up.extend(&names[..depth]);
        Some(up.to_string_lossy().into_owned())
    }
    /// Going into a folder inside this one: this one keeps how far it was
    /// scrolled, for coming back.
    pub(super) fn leave(&mut self, scroll: f32) {
        self.left.push((self.path.clone(), scroll));
    }
    /// How far `path` is scrolled as it takes this folder's place: where it
    /// was left, for a folder gone back up to; its top for any other.
    pub(super) fn scroll_for(&mut self, path: &str) -> f32 {
        let kept = self.left.iter().position(|(left, _)| left == path);
        let scroll = kept.map_or(0.0, |k| self.left[k].1);
        self.left.truncate(kept.unwrap_or(self.left.len()));
        scroll
    }
}

/// Whether a list in a place shows an entry: a folder, or a file the
/// installer takes. Hidden entries are left out, as the stock file manager
/// leaves them out.
pub(super) fn installable(e: &FileRow) -> bool {
    if e.name.starts_with('.') {
        return false;
    }
    match e.kind {
        FileKind::Folder { .. } | FileKind::Link { folder: true, .. } => true,
        FileKind::File | FileKind::Link { folder: false, .. } => installs(e),
        FileKind::Special => false,
    }
}

/// Whether the installer takes a file by its name: an update package, a
/// zip that installs itself, or a partition image.
pub(crate) fn installs(e: &FileRow) -> bool {
    let named = std::path::Path::new(&e.name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("img"));
    named && matches!(e.kind, FileKind::File | FileKind::Link { folder: false, .. })
}

/// The last part of `path`, as the installing page names the file there.
pub(super) fn file_name(path: &str) -> Option<String> {
    std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned())
}

impl App {
    /// Open what can be done with entry `i` of the folder shown, held long
    /// enough to, with a tap of the motor.
    pub(super) fn held_entry(&mut self, i: usize) -> Vec<Effect> {
        let Some(entry) = self.state.files.entry(i).cloned() else {
            return Vec::new();
        };
        self.state.file = Some(entry);
        self.state.asking_delete = false;
        self.open(Screen::FileInfo);
        if self.state.vibrate { vec![Effect::Haptic] } else { Vec::new() }
    }
}

impl App {
    /// Names are taken in `into` where a copy or move of `path` goes: what
    /// to do with them is the user's to choose.
    pub fn file_clash(&mut self, op: FileOp, path: String, into: String, taken: Vec<String>) {
        self.state.clash = Some((Clip { op, path }, into, taken));
        self.open(Screen::Clash);
    }
}

impl App {
    /// Work on files has begun: its page shows how far it comes, in place
    /// of the page of an entry being deleted.
    pub fn file_job_started(&mut self, task: FileTask) {
        if matches!(task, FileTask::Copy { .. } | FileTask::Move { .. }) {
            self.state.clip = None;
        }
        self.state.job = Some(FileJob {
            task,
            progress: None,
            outcome: None,
            stopping: false,
        });
        if self.screen() == Screen::FileInfo {
            self.leave(false);
            self.state.file = None;
            self.state.file_path = None;
            self.state.asking_delete = false;
            self.stack.pop();
        }
        self.open(Screen::FileJob);
    }
}

impl App {
    /// How far work on files has come, in bytes and in entries.
    pub fn file_progress(&mut self, bytes: (u64, u64), entries: (u64, u64)) {
        let job = self.state.job.as_mut().expect("only work under way moves on");
        job.progress = Some((bytes, entries));
        if self.screen() == Screen::FileJob {
            self.refresh();
        }
    }
}

impl App {
    /// Work on files has ended. Done, its page goes and says so; stopped
    /// or failed, the page stays and says why, lit for the user to read.
    pub fn file_job_ended(&mut self, end: JobEnd, now: u64) {
        let job = self.state.job.as_mut().expect("only work under way ends");
        job.stopping = false;
        let done = match (&end, &job.task) {
            (JobEnd::Done, FileTask::Copy { .. }) => Some(["已复制", "已複製", "Copied"]),
            (JobEnd::Done, FileTask::Move { .. }) => Some(["已移动", "已移動", "Moved"]),
            (JobEnd::Done, FileTask::Delete { .. }) => Some(["已删除", "已刪除", "Deleted"]),
            _ => None,
        };
        job.outcome = Some(end);
        match done {
            Some(said) => {
                self.state.job = None;
                if self.screen() == Screen::FileJob {
                    self.leave(true);
                    self.stack.pop();
                    self.settle_page();
                }
                self.refresh();
                self.say(self.state.lang.pick(said), now);
            }
            None => {
                self.wake(now);
                self.refresh();
            }
        }
    }
}

impl App {
    /// The runtime listed a folder for the file manager at `listed`, in Unix
    /// seconds, or could not: it becomes the folder shown.
    pub fn set_files(&mut self, folder: String, files: Result<Vec<FileRow>, String>, listed: i64) {
        self.show_folder(Screen::Files, folder, files, listed);
    }
}

impl App {
    /// The runtime listed a folder of `place` at `listed`, its path from
    /// the place's top, or could not: it becomes the folder shown to choose
    /// a file to install in, with what it holds that may be installed.
    pub fn set_place(&mut self, place: Place, folder: String, files: Result<Vec<FileRow>, String>, listed: i64) {
        let images = self.state.image_only;
        let files = files.map(|entries| {
            entries
                .into_iter()
                .filter(installable)
                .filter(|e| {
                    !images
                        || matches!(e.kind, FileKind::Folder { .. } | FileKind::Link { folder: true, .. })
                        || e.name.to_ascii_lowercase().ends_with(".img")
                        || e.name.to_ascii_lowercase().ends_with(".img.zst")
                })
                .collect()
        });
        self.show_folder(Screen::Choose(place), folder, files, listed);
    }
}

impl App {
    pub fn set_usb_drive(&mut self, drive: Option<UsbDrive>) -> Vec<Effect> {
        if drive == self.state.usb_drive {
            return Vec::new();
        }
        self.state.usb_drive = drive;
        self.state.drive = Folder::new("");
        self.refresh();
        if self.screen() == Screen::Files && std::path::Path::new(&self.state.files.path).starts_with(USB_DRIVE) {
            self.state.file = None;
            self.state.clip = None;
            if self.state.usb_drive == Some(UsbDrive::Shown) {
                return vec![Effect::ReadDir(USB_DRIVE.into())];
            }
            self.state.files = Folder::new("/");
            return vec![Effect::ReadDir("/".into())];
        }
        let gone = !self.state.place_open(Place::UsbDrive) && self.state.restore.as_ref().is_some_and(|(e, _)| e.place == Place::UsbDrive);
        match self.screen() {
            Screen::RestoreBackup if gone => {
                self.state.restore = None;
                self.state.asking_delete = false;
                self.state.confirm = 0.0;
                self.leave(true);
                self.stack.pop();
                self.settle_page();
                return vec![self.find_backups()];
            }
            Screen::Restore => return vec![self.find_backups()],
            _ => {}
        }
        let page = Screen::Choose(Place::UsbDrive);
        if self.screen() != page {
            return Vec::new();
        }
        *self.top_mut() = (page, 0.0, Focus::Unset);
        self.fling = None;
        self.bounce = None;
        self.seek = None;
        self.pull = 0.0;
        self.snap = true;
        self.arrive_at(page)
    }
}

impl App {
    /// `folder` becomes the one the list on page `screen` shows. A folder
    /// gone back up to shows where it was left, as the stock file manager
    /// does; any other starts from its top.
    pub(super) fn show_folder(&mut self, screen: Screen, folder: String, files: Result<Vec<FileRow>, String>, listed: i64) {
        let left = &self.state.folder(screen).path;
        if folder != *left && self.screen() == screen {
            let came_from = std::path::Path::new(left)
                .strip_prefix(&folder)
                .ok()
                .and_then(|rest| rest.components().next());
            let name = came_from.map(|c| c.as_os_str().to_string_lossy().into_owned());
            let k = name.and_then(|name| files.as_ref().ok()?.iter().position(|e| e.name == name));
            let row = if screen == Screen::Files {
                Action::OpenEntry
            } else {
                Action::ChooseEntry
            };
            self.top_mut().2 = match k {
                Some(k) => Focus::On(Spot {
                    stop: Stop::Hit(row(k)),
                    group: Group::Page,
                    rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                }),
                None => Focus::Unset,
            };
            self.top_mut().1 = self.state.folder_mut(screen).scroll_for(&folder);
            self.fling = None;
            self.bounce = None;
            self.seek = None;
            self.pull = 0.0;
            self.snap = true;
        }
        if screen == Screen::Files
            && folder == self.state.files.path
            && let (Some(file), Ok(entries)) = (self.state.file.as_mut(), &files)
            && let Some(listed) = entries.iter().find(|e| e.name == file.name)
        {
            *file = listed.clone();
        }
        if screen == Screen::Files && folder != self.state.files.path {
            self.state.files_query.clear();
            self.state.files_search = false;
            self.clear_found();
        }
        let shown = self.state.folder_mut(screen);
        shown.path = folder;
        shown.entries = files;
        shown.listed = listed;
        shown.keep("");
        self.refresh();
    }
}

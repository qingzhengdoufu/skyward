use super::*;

/// Where a plugin's folder is looked for, from the top of internal storage:
/// beside the backups the recovery writes, so that what a person puts on the
/// phone and what the recovery keeps are found in one place.
pub const PLUGIN_FOLDER: &str = "Skyward/plugins";

/// An entry a search found, and the folder it is in: what the search box
/// shows is the whole tree under the folder on the page, so a row of that
/// list has to say which folder its entry came from — two folders can hold
/// the same name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The folder the entry is in.
    pub folder: String,
    /// The entry, named as it is in that folder.
    pub entry: FileRow,
}

/// The folder an entry a search found is in, and its name, as one path.
pub fn found_path(found: &Found) -> String {
    std::path::Path::new(&found.folder)
        .join(&found.entry.name)
        .to_string_lossy()
        .into_owned()
}

/// How the file manager orders a folder's entries, in the order the page
/// offers them. Folders come first in every one of them, as the stock file
/// manager puts them; ties are broken by name, ignoring case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortKey {
    /// By name, ignoring case.
    #[default]
    Name,
    /// Most recently changed first, so what was just written is at the top.
    Modified,
    /// By kind: folders, then files grouped by what they are — an image, a
    /// package, a text file, anything else — each group ordered by name.
    Kind,
}

/// The three orders, in the order the page lists them.
pub const SORT_KEYS: [SortKey; 3] = [SortKey::Name, SortKey::Modified, SortKey::Kind];

/// A copy of the slot's vbmeta image kept somewhere that outlives a start,
/// as the AVB page lists it. What the image itself says — its slot and its
/// rollback index — is `skyward_vab::vbmeta`'s to read; this is only the
/// pairing of that with the place the file is in, which is the page's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VbmetaCopy {
    pub place: Place,
    /// The file's own name, which says the slot and when it was taken.
    pub name: String,
    /// The slot the image was read from, empty on a phone with one slot.
    pub suffix: String,
    /// Bytes the image takes.
    pub size: u64,
    /// The index it carries, or why it cannot be read as a vbmeta image. The
    /// page shows either, so a file that cannot be read is never passed over
    /// as though there were nothing there — and the write does not trust
    /// this reading either: it reads the partition and the copy again where
    /// the write is made, because what a page was showing is not what the
    /// phone holds.
    pub rollback: Result<Rollback, String>,
    /// When it was taken, in Unix seconds.
    pub made: i64,
}

/// A plugin found in the folder the recovery reads them from: what its
/// manifest says, or why it cannot be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRow {
    /// The plugin's folder name, which is what it is called.
    pub name: String,
    /// What its manifest says, or why there is none to read: the page shows
    /// either, so a folder a person dropped in is never silently passed
    /// over.
    pub manifest: Result<Plugin, String>,
}

/// What a plugin's manifest says it is.
///
/// Nothing runs one yet: this release is the page and the reading of what a
/// plugin will be, so that a later one has somewhere to put what it does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plugin {
    /// What it calls itself, where its folder's name is not enough.
    pub title: Option<String>,
    /// The version it is at, as its author numbers it.
    pub version: Option<String>,
    /// Who wrote it.
    pub author: Option<String>,
    /// What it is for, in one line.
    pub about: Option<String>,
    /// What it says it needs of the recovery, by name: what a later release
    /// refuses to run it without.
    pub needs: Vec<String>,
}

/// A tap on the dark panel, as the touchscreen reports it while nothing else
/// is to be done with the finger: the phone's own system lights its screen
/// on two of them, so the recovery does too.
///
/// A tap is not a touch: what is counted here is only what a finger does to
/// a panel that is off, so the gesture that lights the panel works nothing
/// on the page under it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap {
    /// Where the finger came down, in the same units as [`Touch`].
    pub x: f32,
    pub y: f32,
}

impl Tap {
    /// How long after a tap another may come and still be the second of a
    /// double tap: Android's own window.
    pub const WINDOW_MS: u64 = 300;
    /// How far apart two taps of one double tap may land, as a share of the
    /// screen's width: the slop the phone's own system uses. A tap is in
    /// screen pixels, so this is measured against the width the pages are
    /// laid out in.
    pub const SLOP: f32 = 0.08;
    /// Whether `other` lands near enough to be the second tap of one double
    /// tap started by this, on a screen `w` pixels wide.
    pub fn doubles(&self, other: &Tap, w: f32) -> bool {
        let slop = Self::SLOP * w;
        (other.x - self.x).abs() <= slop && (other.y - self.y).abs() <= slop
    }
}

impl App {
    /// A double tap already recognized by the touch controller. Never toggle
    /// an already-lit screen or activate a page item with this event.
    pub fn hardware_wake(&mut self, now: u64) {
        if !self.display_on && self.state.double_tap && self.state.double_tap_supported {
            self.wake(now);
        }
    }
}

impl App {
    /// A finger came down on the dark panel: the second of two taps that land
    /// near each other, soon enough, lights it, as the phone's own double tap
    /// does. One that comes later than the window is a tap of its own and
    /// starts the count again.
    pub(super) fn tap(&mut self, now: u64, x: f32, y: f32) {
        if !self.state.double_tap || !self.state.double_tap_supported {
            self.tapped = None;
            return;
        }
        let tap = Tap { x, y };
        let doubled = self
            .tapped
            .filter(|(first, at)| now.saturating_sub(*at) <= Tap::WINDOW_MS && first.doubles(&tap, self.w as f32));
        match doubled {
            Some(_) => self.wake(now),
            None => self.tapped = Some((tap, now)),
        }
    }
}

impl App {
    /// The credential to try by itself at this start, where the owner's
    /// files are still shut and one was kept for them: what the decrypt
    /// page would have been asked for, taken without asking. `owner_shut`
    /// says the phone's files did not open with nothing known, which is
    /// the only case a credential is any use in.
    pub fn auto_attempt(&self, owner_shut: bool) -> Option<String> {
        if !self.state.remember_password || !owner_shut {
            return None;
        }
        self.state.remembered.clone()
    }
}

impl App {
    /// The kept credential was refused, so it is not the phone's any more:
    /// a credential left behind after a password change would be tried and
    /// refused at every start.
    pub fn forget_password(&mut self) -> Vec<Effect> {
        self.state.remembered = None;
        vec![Effect::RememberPassword(None)]
    }
}

impl App {
    /// A credential that opened the phone's files, to be written down
    /// where the owner asked for it to be kept.
    pub fn keep_password(&mut self, code: String) -> Vec<Effect> {
        self.state.remembered = Some(code.clone());
        vec![Effect::RememberPassword(Some(code))]
    }
}

impl App {
    /// What an earlier start kept, for the decrypt page to offer.
    pub fn remembered(&mut self, code: Option<String>) {
        self.state.remembered = code;
        self.refresh();
    }
}

impl App {
    /// What the AVB page's read found, and what the write before it said.
    pub fn avb_read(&mut self, state: Result<avbctl::State, String>, notice: Option<String>) {
        self.state.avb = Some(state);
        self.state.avb_pending = None;
        self.state.avb_confirming = false;
        self.state.avb_notice = notice;
        self.state.confirm = 0.0;
        self.refresh();
    }
}

impl App {
    /// The copies of the slot's vbmeta image kept where the phone's storage
    /// has them, or why they could not be read.
    pub fn set_vbmeta_copies(&mut self, copies: Result<Vec<VbmetaCopy>, String>) {
        self.state.vbmeta_copies = Some(copies);
        self.state.vbmeta_pending = None;
        self.state.confirm = 0.0;
        self.refresh();
    }
}

impl App {
    /// Saving a copy of vbmeta, or writing one back, ended: the page says
    /// what came of it and the pending choice is put down.
    pub fn vbmeta_notice(&mut self, said: String) {
        self.state.vbmeta_notice = Some(said);
        self.state.vbmeta_pending = None;
        self.state.avb = None;
        self.state.confirm = 0.0;
        self.refresh();
    }
}

impl App {
    /// The plugins found in the folder the recovery reads them from, or why
    /// it could not be read.
    pub fn set_plugins(&mut self, plugins: Result<Vec<PluginRow>, String>) {
        self.state.plugins = Some(plugins);
        self.refresh();
    }
}

impl App {
    /// A network joined has its password kept, or the one kept for it is
    /// taken away: what the WLAN page offers to forget changed.
    pub fn set_saved(&mut self, saved: Vec<SavedNetwork>) {
        self.state.saved = saved;
        self.refresh();
    }
}

impl App {
    /// Forgetting a network ended: the page says what came of it, and what
    /// is kept is read again.
    pub fn wlan_forgotten(&mut self, forgot: Result<String, String>, now: u64) {
        let l = self.state.lang;
        let said = match forgot {
            Ok(name) if !name.is_empty() => {
                tr!(l, "已忘记「{name}」", "已忘記「{name}」", "“{name}” is forgotten")
            }
            Ok(_) => l
                .pick(["已清除保存的网络", "已清除儲存的網路", "The networks kept are cleared"])
                .to_string(),
            Err(why) => {
                format!(
                    "{}: {why}",
                    l.pick(["未能忘记该网络", "未能忘記該網路", "The network was not forgotten"])
                )
            }
        };
        self.state.forget_pending = None;
        self.state.details = None;
        self.state.asking_delete = false;
        self.state.confirm = 0.0;
        self.refresh();
        self.say(&said, now);
    }
}

impl App {
    /// How deleting the full package downloaded went: bytes freed, and what
    /// internal storage holds of it now; or why it is still there.
    pub fn download_deleted(&mut self, outcome: Result<u64, String>, have: Have, now: u64) {
        self.state.have = have;
        self.state.asking_delete = false;
        self.state.confirm = 0.0;
        self.refresh();
        let l = self.state.lang;
        match outcome {
            Ok(0) => self.say(l.pick(["安装包已不存在", "安裝檔已不存在", "The package is already gone"]), now),
            Ok(freed) => {
                let size = crate::fmt::file_size(freed);
                self.say(
                    &tr!(
                        l,
                        "已删除安装包，释放 {size}",
                        "已刪除安裝檔，釋放 {size}",
                        "Package deleted, freeing {size}"
                    ),
                    now,
                );
            }
            Err(why) => self.report(
                &tr!(
                    l,
                    "无法删除安装包：{why}",
                    "無法刪除安裝檔：{why}",
                    "Couldn't delete the package: {why}"
                ),
                now,
            ),
        }
    }
}

impl App {
    /// The phone has a WLAN, and the recovery brings it up as it starts, as
    /// the phone's own system does: the page shows the radio coming up
    /// rather than a switch that is off, and how it goes comes back with
    /// [`App::set_wlan`]. A WLAN that does not come up says why on its page
    /// and in the kernel log, and the other pages are not held up by it.
    pub fn start_wlan(&mut self) {
        self.state.wlan = Some(Wlan {
            power: Power::Starting,
            ..Wlan::default()
        });
        self.refresh();
    }
}

impl App {
    pub fn image_partitions_read(&mut self, parts: Result<Vec<(String, u64)>, String>) {
        self.state.image_partitions = Some(parts);
        self.state.image_partition = None;
        self.state.confirm = 0.0;
        self.refresh();
    }
}

impl App {
    /// The runtime found the USB drive plugged in, shown or not, or found
    /// none. Another drive, or none, starts its folders over from the top,
    /// and one shown while its page is up is read at once. The backups
    /// listed are looked for again, and a backup on a drive gone leaves its
    /// page.
    pub fn usb_drive_changed(&mut self, drive: Option<UsbDrive>, now: u64) -> Vec<Effect> {
        if drive == self.state.usb_drive {
            return Vec::new();
        }
        let message = match &drive {
            Some(UsbDrive::Shown) => ["U 盘已连接", "USB 隨身碟已連接", "USB drive connected"],
            Some(UsbDrive::Unreadable(_)) => [
                "U 盘已插入，无法读取",
                "USB 隨身碟已插入，無法讀取",
                "USB drive inserted, but unreadable",
            ],
            None => ["U 盘已拔出", "USB 隨身碟已拔出", "USB drive removed"],
        };
        let effects = self.set_usb_drive(drive);
        self.wake(now);
        self.say(self.state.lang.pick(message), now);
        effects
    }
}

impl App {
    /// What a page whose entry has gone says, instead of acting on nothing.
    pub(super) fn no_entry(&mut self, now: u64) -> Vec<Effect> {
        let said = self
            .state
            .lang
            .pick(["没有可操作的条目", "沒有可操作的項目", "There is no entry left to act on"]);
        self.say(said, now);
        Vec::new()
    }
}

impl App {
    /// Look through the folders below the one shown for what the box says,
    /// which is what its list shows while anything is typed. The folder's own
    /// entries stay whole behind it, so emptying the box puts them back
    /// without reading the folder again.
    pub(super) fn find_files(&mut self) -> Vec<Effect> {
        self.state.files.keep("");
        let query = self.state.files_query.trim().to_owned();
        self.state.find_gen += 1;
        let generation = self.state.find_gen;
        if query.is_empty() {
            self.clear_found();
            self.refresh();
            return Vec::new();
        }
        self.state.found = None;
        self.state.finding = true;
        self.refresh();
        vec![Effect::FindFiles {
            root: self.state.files.path.clone(),
            query,
            generation,
            sort: self.state.sort,
        }]
    }
}

impl App {
    /// Forget what a search found, because its text is gone or the folder
    /// under it has changed.
    pub(super) fn clear_found(&mut self) {
        self.state.found = None;
        self.state.finding = false;
        self.state.find_gen += 1;
    }
}

impl App {
    /// What the search `gen` found, or why it could not look: an answer to a
    /// query typed over since is dropped, since the rows under the box are
    /// the rows of the text that is in it now.
    pub fn found_files(&mut self, generation: u64, found: Result<Vec<Found>, String>) {
        if generation != self.state.find_gen {
            return;
        }
        self.state.finding = false;
        self.state.found = Some(found);
        self.refresh();
    }
}

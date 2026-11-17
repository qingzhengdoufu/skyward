use super::*;

/// Choices the pages show and the user changes, and where installing
/// stands.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub control_clock: String,
    pub control_date: String,
    pub image_only: bool,
    pub browser: super::Browser,
    pub root_modules: Option<Result<(String, Vec<skyward_files::root_modules::Module>), String>>,
    pub root_notice: String,
    /// What the AVB page last read: none before its first read, and why
    /// where it could not be read.
    pub avb: Option<Result<avbctl::State, String>>,
    /// A check the page was asked to change, with the state it was asked
    /// to change it in: the write refuses unless that is still what the
    /// slot holds.
    pub avb_pending: Option<(avbctl::State, avbctl::Check, bool)>,
    /// What the last write said, once it ended.
    pub avb_notice: Option<String>,
    /// The language the pages speak.
    pub lang: Lang,
    /// The language was chosen on the language page, now or at an earlier
    /// start, where otherwise it is the one the system names.
    pub language_chosen: bool,
    pub dark: bool,
    pub brightness: f32,
    pub media_volume: f32,
    /// How long the screen stays on with nothing touched, by its place in
    /// [`SCREEN_OFF`].
    pub screen_off: usize,
    /// The time zone the clock and the times on the pages keep, by its
    /// place in [`TIME_ZONES`].
    pub time_zone: usize,
    pub vibrate: bool,
    /// User preference; independent from the device's wake capability.
    pub double_tap: bool,
    pub double_tap_supported: bool,
    /// How strongly the motor taps, from 0 at the slider's left end to 1
    /// at its right.
    pub vibration: f32,
    /// A computer connected over USB is offered internal storage.
    pub file_transfer: bool,
    /// A dark panel keeps the clock and the phone's readings on the glass in
    /// the panel's own doze, rather than being put out. A phone whose panel
    /// has no doze of its own shows nothing either way, and the switch is
    /// not offered on it. It starts off: a phone flashed and never visited
    /// here puts a dark panel out the way the recovery always did, since a
    /// face left glowing all night is a thing to ask for rather than to find.
    pub always_on: bool,
    /// This phone's panel has the two-tap wake at all, which the settings page
    /// then offers a switch for: a panel whose touch controller stops
    /// reporting fingers with the panel off has nothing for the gesture to
    /// work with, and shows nothing either way.
    pub panel_taps: bool,
    /// This phone's panel has a doze of its own, which the settings page then
    /// offers a switch for.
    pub panel_dozes: bool,
    pub reboot: usize,
    /// What the wipe page has chosen, by place in [`WIPES`].
    pub wipe: [bool; WIPES.len()],
    /// The phone has a cache partition, which the wipe page then offers.
    pub cache_partition: bool,
    /// How many folders of its own the phone keeps logs in on the data
    /// partition, which erasing the logs takes in.
    pub phone_logs: usize,
    pub pattern: Vec<usize>,
    /// How far the swipe-to-confirm knob is dragged, 0 to 1.
    pub confirm: f32,
    pub install: Install,
    /// Where the package being installed comes from.
    pub source: Source,
    /// The name of the file being installed, when it is one on the phone.
    pub package_name: Option<String>,
    /// What the install script last run showed, its latest lines.
    pub script_output: Vec<String>,
    /// A zip's install script runs only if the zip was signed with one of
    /// the phone's update keys.
    pub check_signature: bool,
    /// The phone keeps its recovery in a partition of each slot, which the
    /// install page then offers to keep Skyward in.
    pub recovery_partitions: bool,
    /// Skyward goes back into the recovery partitions an install writes
    /// over.
    pub keep_recovery: bool,
    /// Turn both AVB checks off after a ROM is installed, so that a system
    /// built without this phone's keys can start on a bootloader that
    /// would otherwise refuse it. Off unless asked for: it is a change to
    /// the signature the bootloader checks, not to what an install writes.
    pub disable_avb: bool,
    /// Remember the screen lock once it opens the data partition, and try
    /// it by itself at the next start. Off unless asked for, because a
    /// password kept on the phone is a password someone else may read.
    pub remember_password: bool,
    /// The last credential this phone accepted, where remembering is on.
    /// Held so that the decrypt page can offer it, and dropped when it is
    /// turned off or the phone forgets it.
    pub remembered: Option<String>,
    /// How Skyward went back into the recovery partitions the install
    /// wrote over, where it did.
    pub reflash: Option<Reflash>,
    /// The mount page's partitions, as the runtime last read them, or why
    /// it could not.
    pub mounts: Result<Vec<MountRow>, String>,
    pub mount_failure: Option<MountFailure>,
    /// Which of [`BACKUP_PARTS`] a backup takes.
    pub backup: [bool; 4],
    pub efs_backup_available: bool,
    /// Where a backup goes.
    pub backup_to: Place,
    pub image_partitions: Option<Result<Vec<(String, u64)>, String>>,
    pub image_partition: Option<usize>,
    pub compress: bool,
    pub verify: bool,
    /// The backups found, newest first, those that cannot be read last.
    pub backups: Vec<BackupEntry>,
    /// The backup the restore page shows, as it was found, and which of its
    /// parts to restore.
    pub restore: Option<(BackupEntry, [bool; 4])>,
    /// Backing up or restoring under way, or how it last ended.
    pub backup_job: Option<BackupJob>,
    /// Which logs to export: the recovery's, the kernel's.
    pub logs: [bool; 2],
    /// Where the logs go: 0 for the temporary folder, 1 for internal
    /// storage.
    pub logs_to: usize,
    /// What the phone's boot control holds of the slots, as the runtime
    /// last read it for the slot or reboot page, or why it could not.
    pub boot_slots: Option<Result<BootSlots, String>>,
    /// The slot picked on the slot page to start next; with none picked,
    /// the page shows the one the boot control starts next.
    pub next_slot: Option<Slot>,
    /// The phone has what a second system needs: boot and init_boot
    /// partitions in both slots, which a switch writes, and recovery
    /// partitions apart from them, which keep Skyward through it.
    pub dual_boot: bool,
    /// The systems, as the runtime last read them, or why it could not;
    /// none until it has.
    pub systems: Option<Result<Overview, String>>,
    /// The system picked on the systems page to start next; with none
    /// picked, the page shows the one that starts.
    pub system_pick: Option<Who>,
    /// Work on the systems under way.
    pub systems_work: Option<SystemsWork>,
    /// Why the second system's start stopped, as its first program told
    /// it, which the systems page shows.
    pub system_notice: Option<String>,
    /// Where the data partition stands, and what is typed at the screen
    /// lock its users' own files wait on.
    pub data: Data,
    pub code: String,
    /// The users whose own files are still closed beyond the one the page
    /// waits on: a second space's, a cloned app's. The owner's files open
    /// first, and these do not hold the recovery; the decrypt page offers
    /// each by name, and the file manager says so where a folder of one is
    /// opened. Each with the control its screen lock is answered with.
    pub others: Vec<(u32, Keys)>,
    /// What is typed in the search field.
    pub search: String,
    /// The on-screen keyboard's layer.
    pub layer: Layer,
    /// The key of the on-screen keyboard, or the choice of a list open over
    /// the page, a finger holds down.
    pub held: Option<Action>,
    /// The folder the file manager shows.
    pub files: Folder,
    /// How the file manager orders a folder's entries.
    pub sort: SortKey,
    /// What is typed in the file manager's search box, which keeps the rows
    /// of the folder shown down to the names it finds.
    pub files_query: String,
    /// Whether the file manager's keyboard stands at the bottom of it,
    /// typing into that box.
    pub files_search: bool,
    /// What that box found in the folders below the one shown: `None` while
    /// nothing is typed or while the search runs, and the reason it could
    /// not run when it could not.
    pub found: Option<Result<Vec<Found>, String>>,
    /// A search is looking through the folders below the one shown.
    pub finding: bool,
    /// Which search an answer belongs to: typing over a query asks for
    /// another search, and an answer to a query already typed over is
    /// dropped rather than shown under the new one.
    pub find_gen: u64,
    /// Where the entry the file page describes is, when a search found it
    /// and it is therefore not in the folder the list shows.
    pub file_path: Option<String>,
    /// Whether the recovery's own update reaches GitHub through a mirror in
    /// the mainland. On by default: the check goes direct first either way,
    /// so trying the mirror costs nothing when GitHub answers.
    pub mirror: bool,
    /// A network joined keeps its password, so that the recovery joins it by
    /// itself at the next start. It starts on, because joining a network and
    /// typing its password again at every start is the thing this spares
    /// someone; a network that is never joined keeps nothing either way.
    pub remember_wlan: bool,
    /// The folder of internal storage a file to install is chosen in, its
    /// path from the top of the storage.
    pub storage: Folder,
    /// The USB drive plugged in, as the runtime last found it, and the
    /// folder of it a file to install is chosen in, its path from the top
    /// of the drive.
    pub usb_drive: Option<UsbDrive>,
    pub drive: Folder,
    /// The entry the file page describes.
    pub file: Option<FileRow>,
    pub fastboot: Fastboot,
    pub erase: Erase,
    pub asked: Asked,
    /// The entry picked to copy or move, while a folder for it is looked
    /// for.
    pub clip: Option<Clip>,
    /// What is typed as an entry's new name.
    pub new_name: String,
    /// The text file the editor page shows, as typed so far.
    pub editor: Option<Editor>,
    /// Deleting the entry the file page shows, or the backup the restore
    /// page shows, was asked for; a swipe confirms it.
    pub asking_delete: bool,
    /// A copy or move waiting on what to do with the names taken where it
    /// goes, and those names.
    pub clash: Option<(Clip, String, Vec<String>)>,
    /// Work on files under way, or how it last ended.
    pub job: Option<FileJob>,
    /// Giving the data partition's files their SELinux labels under way, or
    /// how it last ended.
    pub relabel: Option<RelabelJob>,
    /// The filesystem page's partitions, as the runtime last read them, or
    /// why it could not; none until it has.
    pub filesystems: Option<Result<Vec<FsRow>, String>>,
    /// Which of them the page for one shows, and what it is to do to it:
    /// a repair, which leaves what it holds as it is, until another is
    /// chosen.
    pub fs_chosen: usize,
    pub fs_action: FsAction,
    /// Work on whole partitions under way, or how it last ended.
    pub part_work: Option<PartWork>,
    /// Where an update the system installed stands, as the runtime last
    /// read it.
    pub update: Standing,
    /// The WLAN, as the runtime last told it; none on a phone whose WLAN the
    /// recovery does not bring up.
    pub wlan: Option<Wlan>,
    /// The network the password page is for, and what is typed there.
    pub joining: Option<Network>,
    /// The network the details page is of, as it was found.
    pub details: Option<Network>,
    /// The network the page that adds one is adding.
    pub adding: Option<Adding>,
    /// The password typed on the password page or the page that adds a
    /// network, and whether it shows.
    pub password: String,
    pub show_password: bool,
    /// The phone maker's update server is known, and the update page asks
    /// it.
    pub update_server: bool,
    pub check: Check,
    pub have: Have,
    /// Downloading the newest system's full package, or how that last
    /// ended.
    pub download: Option<DownloadJob>,
    pub skyward_check: SkywardCheck,
    /// Updating Skyward, or how that last ended.
    pub skyward_job: Option<SkywardJob>,
    /// How fast data went over the network joined in the last second.
    pub traffic: Traffic,
    /// The copies of the slot's vbmeta image found in the places that can
    /// be read, as the runtime last looked for them; none until it has.
    pub vbmeta_copies: Option<Result<Vec<VbmetaCopy>, String>>,
    /// The copy the page was asked to put back, and whether the page has
    /// said what that means: the write waits on a swipe.
    pub vbmeta_pending: Option<VbmetaCopy>,
    /// What the last save or write back of vbmeta said, once it ended.
    pub vbmeta_notice: Option<String>,
    /// A check was asked to be turned off and the AVB page has said what
    /// that means: the second confirmation stands over the page, and the
    /// swipe it holds is the only place the write comes from.
    pub avb_confirming: bool,
    /// The network the details page shows and the swipe asks to forget.
    pub forget_pending: Option<String>,
    /// The networks whose passwords the phone keeps, as the runtime last
    /// read them: what the WLAN page offers to forget one of.
    pub saved: Vec<SavedNetwork>,
    /// The plugins kept on the phone, as the runtime last read them; none
    /// until it has.
    pub plugins: Option<Result<Vec<PluginRow>, String>>,
    /// The plugin the page was opened on, by its place in the list.
    pub plugin_chosen: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            control_clock: "00:00".into(),
            control_date: String::new(),
            image_only: false,
            browser: super::Browser::default(),
            root_modules: None,
            root_notice: String::new(),
            avb: None,
            avb_pending: None,
            avb_notice: None,
            lang: Lang::Hans,
            language_chosen: false,
            dark: false,
            brightness: SLIDER_START,
            media_volume: 0.15,
            screen_off: 3,
            time_zone: 0,
            vibrate: true,
            double_tap: true,
            double_tap_supported: true,
            vibration: SLIDER_START,
            file_transfer: true,
            always_on: false,
            panel_taps: false,
            panel_dozes: false,
            reboot: 0,
            wipe: [false; WIPES.len()],
            cache_partition: false,
            phone_logs: 0,
            pattern: Vec::new(),
            confirm: 0.0,
            install: Install::Idle,
            source: Source::Computer,
            package_name: None,
            script_output: Vec::new(),
            check_signature: false,
            recovery_partitions: false,
            keep_recovery: true,
            disable_avb: false,
            remember_password: false,
            remembered: None,
            reflash: None,
            mounts: Ok(Vec::new()),
            mount_failure: None,
            backup: [true, true, false, true],
            efs_backup_available: true,
            backup_to: Place::Storage,
            image_partitions: None,
            image_partition: None,
            compress: false,
            verify: true,
            backups: Vec::new(),
            restore: None,
            backup_job: None,
            logs: [true, true],
            logs_to: 0,
            boot_slots: None,
            next_slot: None,
            dual_boot: false,
            systems: None,
            system_pick: None,
            systems_work: None,
            system_notice: None,
            data: Data::Closed,
            code: String::new(),
            others: Vec::new(),
            search: String::new(),
            layer: Layer::Letters,
            held: None,
            files: Folder::new("/"),
            sort: SortKey::default(),
            files_query: String::new(),
            files_search: false,
            found: None,
            finding: false,
            find_gen: 0,
            file_path: None,
            mirror: true,
            remember_wlan: true,
            storage: Folder::new(""),
            usb_drive: None,
            drive: Folder::new(""),
            file: None,
            fastboot: Fastboot {
                running: false,
                link: UsbLink::Unplugged,
            },
            erase: Erase::default(),
            asked: Asked::default(),
            clip: None,
            new_name: String::new(),
            editor: None,
            asking_delete: false,
            clash: None,
            job: None,
            relabel: None,
            filesystems: None,
            fs_chosen: 0,
            fs_action: FsAction::Repair,
            part_work: None,
            update: Standing::None,
            wlan: None,
            joining: None,
            details: None,
            adding: None,
            password: String::new(),
            show_password: false,
            update_server: false,
            check: Check::Idle,
            have: Have::Nothing,
            download: None,
            skyward_check: SkywardCheck::Idle,
            skyward_job: None,
            traffic: Traffic::default(),
            vbmeta_copies: None,
            vbmeta_pending: None,
            vbmeta_notice: None,
            avb_confirming: false,
            forget_pending: None,
            saved: Vec::new(),
            plugins: None,
            plugin_chosen: 0,
        }
    }
}

impl State {
    /// The key of the on-screen keyboard a finger holds down.
    pub fn key_held(&self) -> Option<Cap> {
        match self.held {
            Some(Action::Type(cap)) => Some(cap),
            _ => None,
        }
    }
    /// The folder the list on page `screen` shows: the file manager's, or
    /// a place's to choose a file to install in.
    pub fn folder(&self, screen: Screen) -> &Folder {
        match screen {
            Screen::Files => &self.files,
            Screen::Choose(Place::Storage) => &self.storage,
            Screen::Choose(Place::UsbDrive) => &self.drive,
            other => unreachable!("{other:?} lists no folder"),
        }
    }
    pub(super) fn folder_mut(&mut self, screen: Screen) -> &mut Folder {
        match screen {
            Screen::Files => &mut self.files,
            Screen::Choose(Place::Storage) => &mut self.storage,
            Screen::Choose(Place::UsbDrive) => &mut self.drive,
            other => unreachable!("{other:?} lists no folder"),
        }
    }
    /// Whether the files of `place` can be read: internal storage once it
    /// is open, a USB drive once the runtime shows it.
    pub fn place_open(&self, place: Place) -> bool {
        match place {
            Place::Storage => self.data.storage_open(),
            Place::UsbDrive => self.usb_drive == Some(UsbDrive::Shown),
        }
    }
    /// Whether a network is joined and has given an address.
    pub fn online(&self) -> bool {
        matches!(
            & self.wlan, Some(Wlan { joined : Some(j), .. }) if matches!(j.stage,
            Joining::Connected(_))
        )
    }
    /// The newest system the update server told of.
    pub fn newest(&self) -> Option<&Release> {
        match &self.check {
            Check::Answered(Some(r)) => Some(r),
            _ => None,
        }
    }
    /// Whether the full package is being downloaded.
    pub fn downloading(&self) -> bool {
        self.download.as_ref().is_some_and(|d| d.outcome.is_none())
    }
    /// Skyward's newest release, as GitHub told of it.
    pub fn skyward_newest(&self) -> Option<&Newest> {
        match &self.skyward_check {
            SkywardCheck::Answered(n) => Some(n),
            _ => None,
        }
    }
    /// Whether Skyward is being updated.
    pub fn updating_skyward(&self) -> bool {
        self.skyward_job.as_ref().is_some_and(|j| j.outcome.is_none())
    }
    /// Whether the phone has the page `screen`: the WLAN page where the
    /// recovery brings the phone's WLAN up, and updating Skyward where the
    /// phone also keeps its recovery in a partition of each slot.
    pub fn offers(&self, screen: Screen) -> bool {
        match screen {
            Screen::Wlan => self.wlan.is_some(),
            Screen::SkywardUpdate => self.wlan.is_some() && self.recovery_partitions,
            Screen::Systems => self.dual_boot,
            _ => true,
        }
    }
}

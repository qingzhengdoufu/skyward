use super::*;

/// What going back is told while work that can be stopped goes on.
pub(super) const WAIT_OR_STOP: Text = crate::texts::toast::back::WAIT_OR_STOP;

impl App {
    /// Open `screen` over the page shown, sliding in over it.
    pub(super) fn open(&mut self, screen: Screen) {
        if self.shade.is_some() {
            self.dismiss_shade();
        }
        if screen == Screen::Browser {
            if let Some(index) = self.stack.iter().rposition(|(page, ..)| *page == Screen::Browser) {
                self.end_turn();
                self.stack.truncate(index + 1);
                self.settle_page();
                self.refresh();
                return;
            }
        } else if self.screen() == screen {
            self.refresh();
            return;
        }
        self.leave(false);
        self.push(screen);
    }
}

impl App {
    pub(super) fn push(&mut self, screen: Screen) {
        self.state.ime.clear();
        self.state.ime.enabled = false;
        self.stack.push((screen, 0.0, Focus::Unset));
        self.settle_page();
        self.refresh();
    }
}

impl App {
    /// The page shown gives way to another, opened over it or, `back`, gone
    /// back to under it: draw it as it last showed, to slide away. Changes
    /// made before the slide's first frame keep the page that showed; a
    /// page rendered anew since it last showed gives way at once.
    pub(super) fn leave(&mut self, back: bool) {
        if let Some(turn) = self.turn.as_mut().filter(|t| t.spring.is_none()) {
            turn.back = back;
            return;
        }
        self.end_turn();
        let Some(view) = self.shown.take() else { return };
        let mut left = self.backdrop.take().unwrap_or_else(|| Canvas::new(self.w, self.h));
        let view = View { pressing: None, ..view };
        let t = self.theme();
        self.paint_page(&mut left, &t, None, &view);
        if let Some(fr) = self.frame.as_ref().filter(|_| self.cols < self.w) {
            left.fill_rect(self.cols as f32, 0.0, (self.w - self.cols) as f32, self.h as f32, fr.bg);
        }
        self.turn = Some(Turn { back, left, spring: None });
        self.dirty = true;
    }
}

impl App {
    /// Set the home page's search bar moving up to the search page's field,
    /// or back down, before the page changes: from where a move not yet
    /// done has it, at the speed it has.
    pub(super) fn start_morph(&mut self, into: bool, now: u64) {
        let (q, speed, title, cancel) = match self.morph.take() {
            Some(m) => {
                let (q, _) = m.at(now);
                let speed = (q - m.at(now.saturating_sub(8)).0) / 0.008;
                (q, speed, m.title, m.cancel)
            }
            None => (if into { 0.0 } else { 1.0 }, 0.0, None, None),
        };
        let target = if into { 1.0 } else { 0.0 };
        let home = self.stack.len() - if into { 1 } else { 2 };
        let keyboard = if into {
            None
        } else {
            self.frame.as_ref().map(|fr| keyboard_strip(&self.theme(), fr))
        };
        self.morph = Some(Morph {
            into,
            spring: Spring::new(now, (q - target) * 1000.0, speed * 1000.0, MORPH_PERIOD),
            start: now,
            scroll: self.stack[home].1,
            keyboard,
            body: None,
            title,
            cancel,
        });
        self.dirty = true;
    }
}

impl App {
    /// Go back a page. Leaving the installing page stops what can be
    /// stopped; while a package is being read or written, the page stays.
    /// In the file manager, back climbs a folder until it reaches the root,
    /// and in a place, until it reaches the place's top.
    pub(super) fn back(&mut self, now: u64) -> Vec<Effect> {
        if self.screen() == Screen::NewFolder {
            self.state.new_name.clear();
        }
        if self.browser_immersive() {
            return self.set_browser_immersive(false);
        }
        if self.shade.is_some() && self.screen() == Screen::ControlCenter {
            self.settle_shade(false, now, 0.0);
            return Vec::new();
        }
        let (l, mut fx) = (self.state.lang, Vec::new());
        if self.stack.len() <= 1 {
            return fx;
        }
        let morph = self.screen() == Screen::Search && self.stack[self.stack.len() - 2].0 == Screen::Home;
        if morph {
            self.start_morph(false, now);
        }
        let left = self.screen();
        self.state.ime.clear();
        self.state.ime.enabled = false;
        match left {
            Screen::BrowserAddress => {
                self.state.browser.full_text = false;
                self.state.browser.ime.clear();
            }
            Screen::Mount => self.state.mount_failure = None,
            Screen::Slot => self.state.next_slot = None,
            Screen::Systems => {
                if self.state.systems_work.is_some() {
                    self.say(l.pick(crate::texts::toast::back::WRITING), now);
                    return fx;
                }
                self.state.system_pick = None;
                self.state.asking_delete = false;
            }
            Screen::Files => {
                if let Some(up) = self.state.files.up() {
                    return vec![Effect::ReadDir(up)];
                }
                self.state.clip = None;
            }
            Screen::Choose(place) => {
                if let Some(up) = self.state.folder(Screen::Choose(place)).up() {
                    return vec![Effect::ReadPlace(place, up)];
                }
                self.state.module_picker = false;
            }
            Screen::FileInfo => {
                self.state.file = None;
                self.state.asking_delete = false;
            }
            Screen::Editor => {
                let changed = self.state.editor.as_ref().expect("the editor page shows a file").changed;
                if changed && self.unsaved_warned.is_none_or(|at| now >= at + TOAST_MS) {
                    self.unsaved_warned = Some(now);
                    self.say(l.pick(crate::texts::toast::back::UNSAVED), now);
                    return fx;
                }
                self.state.editor = None;
                self.unsaved_warned = None;
            }
            Screen::Clash => self.state.clash = None,
            Screen::WlanPassword => {
                self.state.joining = None;
                self.state.password.clear();
            }
            Screen::WlanNetwork => {
                self.state.details = None;
                self.state.asking_delete = false;
                self.state.confirm = 0.0;
            }
            Screen::WlanAdd => {
                if let Some(a) = self.state.adding.as_mut().filter(|a| a.choosing) {
                    a.choosing = false;
                    self.refresh();
                    return fx;
                }
                self.state.adding = None;
                self.state.password.clear();
            }
            Screen::FileJob => {
                if self.state.job.as_ref().is_some_and(|j| j.outcome.is_none()) {
                    self.say(l.pick(WAIT_OR_STOP), now);
                    return fx;
                }
                self.state.job = None;
            }
            Screen::Relabel => {
                if self.state.relabel.as_ref().is_some_and(|j| j.outcome.is_none()) {
                    self.say(l.pick(WAIT_OR_STOP), now);
                    return fx;
                }
                self.state.relabel = None;
            }
            Screen::PartWork => {
                if self.state.part_work.as_ref().is_some_and(|w| w.outcome.is_none()) {
                    self.say(l.pick(crate::texts::toast::back::WAIT), now);
                    return fx;
                }
                self.state.part_work = None;
                self.state.fs_action = FsAction::Repair;
            }
            Screen::RestoreBackup => {
                self.state.restore = None;
                self.state.asking_delete = false;
            }
            Screen::SkywardUpdate => match self.state.skyward_job.as_ref().filter(|j| j.outcome.is_none()).map(|j| j.stage) {
                Some(Some(SkywardStage::Write { .. })) => {
                    self.say(l.pick(crate::texts::toast::back::WRITING), now);
                    return fx;
                }
                Some(_) => {
                    self.say(l.pick(WAIT_OR_STOP), now);
                    return fx;
                }
                None => {}
            },
            Screen::BackupJob => {
                let job = self.state.backup_job.as_ref().expect("the backup page shows work");
                if job.outcome.is_none() {
                    let writing = job.restore && job.at.is_some_and(|a| !a.checking);
                    let said = if writing {
                        crate::texts::toast::back::RESTORING
                    } else {
                        WAIT_OR_STOP
                    };
                    self.say(l.pick(said), now);
                    return fx;
                }
                self.state.backup_job = None;
            }
            _ => {}
        }
        if self.screen() == Screen::Installing {
            match self.state.install {
                Install::Checking {
                    stage: Some(Stage::Signature { .. }),
                    ..
                } => {
                    self.state.install = Install::Ending;
                    fx.push(Effect::Cancel);
                }
                Install::Checking { .. } => {
                    self.say(l.pick(crate::texts::toast::back::READING), now);
                    return fx;
                }
                Install::Updating { .. } | Install::Scripting { .. } => {
                    self.say(l.pick(crate::texts::toast::back::INSTALLING), now);
                    return fx;
                }
                Install::Flashing { .. } => {
                    self.say(l.pick(crate::texts::toast::back::FLASHING), now);
                    return fx;
                }
                Install::Systeming { .. } => {
                    self.say(l.pick(crate::texts::toast::back::INSTALLING), now);
                    return fx;
                }
                Install::Opening => {
                    self.say(l.pick(DATA_OPENING), now);
                    return fx;
                }
                Install::Wiping => {
                    self.say(l.pick(crate::texts::toast::back::WIPING), now);
                    return fx;
                }
                Install::Starting
                | Install::Locked(_)
                | Install::Waiting
                | Install::Choosing(_)
                | Install::SystemChoosing(_)
                | Install::Confirming { .. }
                | Install::ConfirmingScript => {
                    self.state.install = Install::Ending;
                    fx.push(Effect::Cancel);
                }
                Install::Finished { .. } | Install::Wiped(_) if self.state.source == Source::Rescue => {
                    self.state.install = Install::Ending;
                    fx.push(Effect::Cancel);
                }
                Install::Finished { .. } | Install::Wiped(_) => {
                    self.state.install = Install::Idle;
                }
                Install::Idle | Install::Ending => {}
            }
        }
        if !morph {
            self.leave(true);
        }
        self.stack.pop();
        self.state.confirm = 0.0;
        self.settle_page();
        self.refresh();
        match self.screen() {
            Screen::Restore => fx.push(self.find_backups()),
            Screen::Filesystem | Screen::Filesystems if left == Screen::PartWork => fx.push(Effect::ReadFilesystems),
            _ => {}
        }
        fx
    }
}

impl App {
    /// What the page shown first asks of the runtime as it shows.
    pub fn arrive(&mut self) -> Vec<Effect> {
        self.arrive_at(self.screen())
    }
}

impl App {
    /// What a page asks of the runtime as it opens: what it shows read, or
    /// started. The decrypt page opens the data partition, which is also
    /// how it learns which control a user's screen lock is answered with.
    pub(super) fn arrive_at(&mut self, s: Screen) -> Vec<Effect> {
        match s {
            Screen::Browser => {
                self.state.layer = Layer::Letters;
                Vec::new()
            }
            Screen::RootModules => vec![Effect::ReadRootModules],
            Screen::Partitions => vec![Effect::ReadImagePartitions],
            Screen::Avb => {
                self.state.avb = None;
                self.state.avb_pending = None;
                self.state.avb_notice = None;
                vec![Effect::ReadAvb]
            }
            Screen::Mount => vec![Effect::ReadMounts],
            Screen::Vbmeta => {
                self.state.vbmeta_copies = None;
                self.state.vbmeta_pending = None;
                vec![
                    Effect::FindVbmeta {
                        storage: true,
                        usb: self.state.usb_drive.is_some(),
                    },
                    Effect::ReadAvb,
                ]
            }
            Screen::Filesystems => vec![Effect::ReadFilesystems],
            Screen::Slot | Screen::Reboot => vec![Effect::ReadSlots],
            Screen::Systems => vec![Effect::ReadSystems],
            Screen::Restore => vec![self.find_backups()],
            Screen::ImageBackup => {
                self.state.image_partitions = None;
                self.state.image_partition = None;
                self.state.confirm = 0.0;
                vec![Effect::ReadImagePartitions]
            }
            Screen::Files => vec![Effect::ReadDir(self.state.files.path.clone())],
            Screen::Choose(place) if self.state.place_open(place) => {
                vec![Effect::ReadPlace(place, self.state.folder(s).path.clone())]
            }
            Screen::Search => {
                self.state.search.clear();
                self.state.layer = Layer::Letters;
                Vec::new()
            }
            Screen::Terminal => {
                self.state.layer = Layer::Letters;
                if self.term.is_none() {
                    self.shell_folder = "/".into();
                    vec![Effect::StartShell(self.shell_folder.clone())]
                } else {
                    Vec::new()
                }
            }
            Screen::Decrypt if matches!(self.state.data, Data::Closed | Data::Failed(_)) => self.open_data(),
            Screen::Update => {
                let mut fx = vec![Effect::ReadUpdate];
                self.state.asking_delete = false;
                self.state.confirm = 0.0;
                if self.state.update_server && matches!(self.state.check, Check::Idle | Check::Failed(_)) && self.state.online() {
                    self.state.check = Check::Asking;
                    fx.push(Effect::CheckUpdate);
                }
                fx
            }
            Screen::SkywardUpdate
                if matches!(self.state.skyward_check, SkywardCheck::Idle | SkywardCheck::Failed(_)) && self.state.online() =>
            {
                self.state.skyward_check = SkywardCheck::Asking;
                vec![Effect::CheckSkyward]
            }
            Screen::Wlan if self.state.wlan.as_ref().is_some_and(|w| w.power == Power::On) => vec![Effect::ScanWlan],
            _ => Vec::new(),
        }
    }
}

use super::*;

/// What an install is told while the last one has not ended.
pub const INSTALL_UNFINISHED: Text = crate::texts::toast::INSTALL_UNFINISHED;

pub const RESTART_AFTER_INSTALL: Text = crate::lang::legacy([
    "系统分区已卸载，请重启 recovery 后再使用解密等系统服务。等待不会自动恢复。",
    "系統分割區已卸載，請重新啟動 recovery 後再使用解密等系統服務。等待不會自動恢復。",
    "System partitions were detached. Reboot recovery before decrypting or using system services. Waiting does not restore them.",
]);

/// Where the package being installed comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Source {
    /// The computer, with `adb sideload`.
    #[default]
    Computer,
    /// A file chosen in a place.
    File(Place),
    /// A file opened in the file manager; `drive` for one on the USB drive.
    Opened { drive: bool },
    /// A package on the phone the system asked to be installed before the
    /// recovery started; `security` for a security update.
    Asked { security: bool },
    /// The computer, with `adb rescue`.
    Rescue,
}

/// What the system asked of the recovery before it started, as the pages
/// that answer it show it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// Why, as the system put it.
    pub reason: Option<String>,
    /// Power off once the work is done, instead of starting the system.
    pub shutdown_after: bool,
    /// Work the recovery does not do, as it was asked.
    pub unhandled: Option<String>,
}

/// Which slots an image goes to, by the slot the phone runs from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Current,
    Other,
    Both,
}

impl Pick {
    pub const ALL: [Pick; 3] = [Pick::Current, Pick::Other, Pick::Both];
    /// The slots this means on a phone running from `current`.
    pub fn slots(self, current: Slot) -> Vec<Slot> {
        match self {
            Pick::Current => vec![current],
            Pick::Other => vec![current.other()],
            Pick::Both => vec![Slot::A, Slot::B],
        }
    }
}

/// Where installing stands, as the installer last told.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Install {
    /// No installer runs.
    #[default]
    Idle,
    /// USB is being set up for sideload or rescue, or the package the
    /// system asked for is looked for.
    Starting,
    /// The package the system asked for is in the data partition, which is
    /// being opened.
    Opening,
    /// It is in the own files of this user, which wait on their screen
    /// lock.
    Locked(u32),
    Waiting,
    /// The package of `size` bytes is looked at; a zip's signature is
    /// checked, once the installer tells how far.
    Checking {
        size: u64,
        stage: Option<Stage>,
    },
    /// An image waits to be told where it goes.
    Choosing(Choice),
    /// An update waits for the user to agree: the version it brings, and
    /// whether all user data is erased after it.
    Confirming {
        version: Option<String>,
        wipe: bool,
    },
    /// `overwriting` says whether the update writes over the running
    /// system.
    Updating {
        version: Option<String>,
        stage: Option<Stage>,
        overwriting: bool,
    },
    Flashing {
        partition: &'static str,
        stage: Option<Stage>,
    },
    /// A zip with an install script waits for the user to agree to run it.
    ConfirmingScript,
    /// A system to install beside the installed one waits for the room its
    /// root takes to be chosen.
    SystemChoosing(SystemChoice),
    /// The system is being installed: its name, and how far.
    Systeming {
        name: String,
        stage: Option<Stage>,
    },
    /// Its install script is made ready and then, once `running`, runs; how
    /// far it says it is, once it says.
    Scripting {
        stage: Option<Stage>,
        running: bool,
    },
    /// `damage` is what a failure may have left broken.
    Finished {
        outcome: Outcome,
        damage: Option<Damage>,
    },
    /// In rescue, all user data is being erased, as the computer asked.
    Wiping,
    /// In rescue, erasing all user data ended: done, or why it stopped.
    Wiped(Result<(), EraseStop>),
    /// The page was left and the installer is stopping, handing USB back
    /// if it had it.
    Ending,
}

/// What a failed install may have left broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Damage {
    /// The update had begun writing over the running system.
    System,
    /// The image had begun going into this partition.
    Partition(&'static str),
    /// The install script had begun to run, and may have written to any
    /// partition.
    Script,
}

/// An image, and where it goes so far.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub kind: ImageKind,
    pub size: u64,
    /// The partitions it fits, the likeliest first.
    pub partitions: Vec<&'static str>,
    /// Which of them is chosen.
    pub partition: usize,
    pub pick: Pick,
}

impl App {
    /// The system asked for the package at `name` to be installed before
    /// the recovery started: the installing page opens over the home page,
    /// looking for it. `security` for a security update.
    pub fn start_asked(&mut self, security: bool, name: String) {
        self.begin_install(Install::Starting, Source::Asked { security }, Some(name));
        self.open(Screen::Installing);
    }
}

impl App {
    /// The package the system asked for is in the data partition, which is
    /// being opened for it.
    pub fn asked_opening(&mut self) {
        self.state.install = Install::Opening;
        self.state.data = Data::Opening;
        self.refresh();
    }
}

impl App {
    /// The package the system asked for is in the own files of `user`,
    /// which wait on their screen lock: the page says to decrypt them, and
    /// lights for it.
    pub fn asked_locked(&mut self, user: u32, now: u64) {
        self.state.install = Install::Locked(user);
        self.wake(now);
        self.refresh();
    }
}

impl App {
    /// The system asked for rescue before the recovery started: the
    /// installing page opens over the home page, already waiting for the
    /// computer.
    pub fn start_rescue(&mut self) {
        self.begin_install(Install::Starting, Source::Rescue, None);
        self.open(Screen::Installing);
    }
}

impl App {
    /// In rescue, the computer asked for all user data to be erased, and it
    /// is being erased.
    pub fn rescue_wiping(&mut self) {
        self.state.install = Install::Wiping;
        self.refresh();
    }
}

impl App {
    /// In rescue, erasing all user data has ended; a data partition erased
    /// whole opens no more until the system makes it afresh.
    pub fn rescue_wiped(&mut self, outcome: Result<(), EraseStop>, now: u64) {
        if outcome.is_ok() {
            self.state.data = Data::Erased;
        }
        self.state.install = Install::Wiped(outcome);
        self.wake(now);
        if matches!(self.screen(), Screen::Installing | Screen::ControlCenter) {
            self.refresh();
        }
    }
}

impl App {
    /// The system asked for sideload before the recovery started: the page
    /// opens over the home page, already receiving.
    pub fn start_sideload(&mut self) {
        self.begin_install(Install::Starting, Source::Computer, None);
        self.open(Screen::Installing);
    }
}

impl App {
    /// Start an install from `source` afresh, of the file named `name` when
    /// it is one on the phone: nothing the last install showed stays.
    pub(super) fn begin_install(&mut self, install: Install, source: Source, name: Option<String>) {
        let s = &mut self.state;
        (s.install, s.source, s.package_name) = (install, source, name);
        s.script_output.clear();
        s.script_warnings.clear();
        s.reflash = None;
    }
}

impl App {
    /// What the system asked of the recovery before it started.
    pub fn set_asked(&mut self, asked: Asked) {
        self.state.asked = asked;
        self.refresh();
    }
}

impl App {
    /// Hear from the installer. Once the installing page is left, only its
    /// end matters. An image waiting for its partition, an update or a
    /// script waiting to be agreed to, and the end light a dark panel: all
    /// want the user.
    pub fn install(&mut self, ev: Event, now: u64) {
        if ev == Event::Detached {
            self.state.system_detached = true;
            self.refresh();
            return;
        }
        if self.state.install == Install::Ending {
            if matches!(ev, Event::Finished(_)) {
                self.state.install = Install::Idle;
            }
            return;
        }
        if matches!(ev, Event::Wipe | Event::Reboot(_)) {
            return;
        }
        if matches!(
            ev,
            Event::Image { .. } | Event::Ready { .. } | Event::Script | Event::System { .. } | Event::Done(_) | Event::Finished(_)
        ) {
            self.wake(now);
        }
        let install = &mut self.state.install;
        match ev {
            Event::Waiting => *install = Install::Waiting,
            Event::Checking { size } => {
                *install = Install::Checking { size, stage: None };
            }
            Event::Ready { version, wipe } => {
                *install = Install::Confirming { version, wipe };
            }
            Event::Installing { version } => {
                *install = Install::Updating {
                    version,
                    stage: None,
                    overwriting: false,
                };
            }
            Event::Overwriting => {
                if let Install::Updating { overwriting, .. } = install {
                    *overwriting = true;
                }
            }
            Event::Image { kind, size, partitions } => {
                *install = Install::Choosing(Choice {
                    kind,
                    size,
                    partitions,
                    partition: 0,
                    pick: Pick::Current,
                });
            }
            Event::Script => *install = Install::ConfirmingScript,
            Event::System { name, version, sizes } => {
                *install = Install::SystemChoosing(SystemChoice {
                    name,
                    version,
                    sizes,
                    size: 0,
                });
            }
            Event::Running => {
                *install = Install::Scripting {
                    stage: None,
                    running: true,
                };
                self.state.script_output.clear();
                self.state.script_warnings.clear();
            }
            Event::Warning(warning) => {
                if !self.state.script_warnings.contains(&warning) && self.state.script_warnings.len() < 6 {
                    self.state.script_warnings.push(warning);
                }
            }
            Event::Print(line) => {
                let shown = &mut self.state.script_output;
                if shown.len() == SCRIPT_LINES {
                    shown.remove(0);
                }
                shown.push(line);
            }
            Event::Progress(stage) => match install {
                Install::Checking { stage: shown, .. }
                | Install::Updating { stage: shown, .. }
                | Install::Flashing { stage: shown, .. }
                | Install::Scripting { stage: shown, .. }
                | Install::Systeming { stage: shown, .. } => *shown = Some(stage),
                _ => {}
            },
            Event::Reflashed(reflash) => self.state.reflash = Some(reflash),
            Event::Avb(avb) => {
                let slot = avb.slot.map(|s| s.letter()).unwrap_or("?");
                let said = match avb.done {
                    Ok(None) => {
                        tr!(
                            self.state.lang,
                            "{slot} 槽的 AVB 校验本来就是关闭的",
                            "{slot} 槽的 AVB 驗證本來就是關閉的",
                            "The AVB checks of slot {slot} were already off"
                        )
                    }
                    Ok(Some(backup)) => {
                        tr!(
                            self.state.lang,
                            "{slot} 槽的 AVB 校验已关闭，重启后生效。原镜像备份：{backup}",
                            "{slot} 槽的 AVB 驗證已停用，重新啟動後生效。原映像檔備份：{backup}",
                            "The AVB checks of slot {slot} are off, from the next start. The image as it was is kept at {backup}"
                        )
                    }
                    Err(why) => {
                        tr!(
                            self.state.lang,
                            "{slot} 槽的 AVB 校验没有改动：{why}",
                            "{slot} 槽的 AVB 驗證沒有變更：{why}",
                            "The AVB checks of slot {slot} were left as they were: {why}"
                        )
                    }
                };
                self.state.avb_notice = Some(said);
                self.refresh();
            }
            Event::Detached | Event::Wipe | Event::Reboot(_) => {
                unreachable!("the runtime carries out what rescue asks of it")
            }
            Event::Done(outcome) | Event::Finished(outcome) => {
                let damage = match (&outcome, &*install) {
                    (Outcome::Failed(_), Install::Updating { overwriting: true, .. }) => Some(Damage::System),
                    (
                        Outcome::Failed(_),
                        Install::Flashing {
                            partition,
                            stage: Some(Stage::Flashing { done, .. }),
                        },
                    ) if *done > 0 => Some(Damage::Partition(partition)),
                    (Outcome::Failed(_), Install::Scripting { running: true, .. }) => Some(Damage::Script),
                    _ => None,
                };
                *install = Install::Finished { outcome, damage };
            }
        }
        if matches!(self.screen(), Screen::Installing | Screen::ControlCenter) {
            self.refresh();
        }
    }
}

impl App {
    /// Install the file of `size` bytes at `path` in `place`, its page
    /// following how it goes; one install at a time.
    pub(super) fn install_file(&mut self, place: Place, path: String, size: u64, now: u64) -> Vec<Effect> {
        let name = file_name(&path);
        self.install_named(Source::File(place), name, size, Effect::InstallFile(place, path), now)
    }
}

impl App {
    /// Install the file of `size` bytes at the full `path`, opened in the
    /// file manager, as a file chosen in a place installs.
    pub(super) fn install_opened(&mut self, path: String, size: u64, now: u64) -> Vec<Effect> {
        let source = Source::Opened {
            drive: std::path::Path::new(&path).starts_with(USB_DRIVE),
        };
        let name = file_name(&path);
        self.install_named(source, name, size, Effect::InstallPath(path), now)
    }
}

impl App {
    /// Install the file `name` of `size` bytes from `source`, `start` asking
    /// the runtime to, its page following how it goes; one install at a
    /// time.
    pub(super) fn install_named(&mut self, source: Source, name: Option<String>, size: u64, start: Effect, now: u64) -> Vec<Effect> {
        match self.state.install {
            Install::Idle | Install::Finished { .. } => {
                self.begin_install(Install::Checking { size, stage: None }, source, name);
                self.open(Screen::Installing);
                vec![start]
            }
            _ => {
                self.say(self.state.lang.pick(INSTALL_UNFINISHED), now);
                Vec::new()
            }
        }
    }
}

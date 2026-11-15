use super::*;

/// Where the reboot page sends the device, in the order it lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    System,
    Recovery,
    Fastboot,
    Bootloader,
    Edl,
    PowerOff,
}

pub const TARGETS: [Target; 6] = [
    Target::System,
    Target::Recovery,
    Target::Fastboot,
    Target::Bootloader,
    Target::Edl,
    Target::PowerOff,
];

/// Where the fastboot page goes, in the order it lists them: the volume
/// keys move between them and the power key takes one.
pub const FASTBOOT_CHOICES: [(Text, Action); 4] = [
    (
        ["重启到系统", "重新啟動到系統", "Reboot to system"],
        Action::RebootTo(Target::System),
    ),
    (["进入恢复模式", "進入復原模式", "Enter recovery"], Action::EnterRecovery),
    (
        ["重启到 Bootloader", "重新啟動到 Bootloader", "Reboot to bootloader"],
        Action::RebootTo(Target::Bootloader),
    ),
    (["关机", "關機", "Power off"], Action::RebootTo(Target::PowerOff)),
];

/// How far a computer has the USB port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbLink {
    /// No cable, or nothing answering at its other end.
    Unplugged,
    /// Plugged in, with the computer still setting the port up.
    Connecting,
    /// Set up by the computer, which can send commands.
    Connected,
}

/// Fastboot as its page shows it: whether the phone's fastbootd runs and
/// how far a computer has the port, as the runtime last read them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fastboot {
    pub running: bool,
    pub link: UsbLink,
}

impl App {
    /// The runtime read whether fastbootd runs and how far a computer has
    /// the port: keep it and show it if it changed.
    pub fn set_fastboot(&mut self, running: bool, link: UsbLink) {
        let now = Fastboot { running, link };
        if self.state.fastboot != now {
            self.state.fastboot = now;
            self.refresh();
        }
    }
}

impl App {
    /// Fastboot has stopped and the recovery is up: its pages start again
    /// from the home page, which nothing can go back from to fastboot.
    pub fn enter_recovery(&mut self) {
        self.stack = vec![(Screen::Home, 0.0, Focus::Unset)];
        self.settle_page();
        self.refresh();
    }
}

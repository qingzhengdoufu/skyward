use super::*;

/// The settings' sliders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slider {
    Brightness,
    /// How strongly the motor taps.
    Vibration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    Vibrate,
    /// Offer internal storage to a computer over USB.
    FileTransfer,
    /// Compress a backup.
    Compress,
    /// Read a backup back once it is written.
    Verify,
    /// Run a zip's install script only if the zip was signed with one of
    /// the phone's update keys.
    Signature,
    /// Write Skyward back into the recovery partitions an install writes
    /// over.
    KeepRecovery,
    /// Turn both AVB checks off after a ROM is installed.
    DisableAvb,
    /// Remember the screen lock that opens the data partition.
    RememberPassword,
    Wlan,
    /// Show the clock and the phone's readings on a dark panel, in the
    /// panel's own doze, rather than putting the panel out.
    AlwaysOn,
    /// Light the dark panel when it is tapped twice, which keeps the panel
    /// dozing rather than taken down so that the taps can be felt at all.
    DoubleTap,
    /// Show the password typed rather than a dot for each character.
    ShowPassword,
    /// Keep the network joined, with its password, so that the recovery
    /// joins it by itself at the next start.
    RememberWlan,
    /// Reach GitHub through a mirror in the mainland for the recovery's own
    /// update: the check and the download both, since both hosts that serve
    /// them are commonly out of reach there. Direct first, then the mirror.
    Mirror,
}

/// How long the screen stays on with nothing touched, in milliseconds, in
/// the order the page lists them; `None` keeps it on.
pub const SCREEN_OFF: [Option<u64>; 7] = [
    Some(15_000),
    Some(30_000),
    Some(60_000),
    Some(120_000),
    Some(300_000),
    Some(600_000),
    None,
];

/// The time zones the clock and the times on the pages can keep, in the
/// order their page lists them: the name, the cities it covers, and the
/// zone's name in the tz database.
pub const TIME_ZONES: [(Text, Text, &str); 9] = [
    (
        ["中国标准时间", "中國標準時間", "China Standard Time"],
        ["北京、上海", "北京、上海", "Beijing, Shanghai"],
        "Asia/Shanghai",
    ),
    (
        ["香港时间", "香港時間", "Hong Kong Time"],
        ["香港", "香港", "Hong Kong"],
        "Asia/Hong_Kong",
    ),
    (["台北时间", "台北時間", "Taipei Time"], ["台北", "台北", "Taipei"], "Asia/Taipei"),
    (
        ["日本标准时间", "日本標準時間", "Japan Standard Time"],
        ["东京", "東京", "Tokyo"],
        "Asia/Tokyo",
    ),
    (
        ["协调世界时", "世界標準時間", "Coordinated Universal Time"],
        ["UTC", "UTC", "UTC"],
        "UTC",
    ),
    (
        ["英国时间", "英國時間", "British Time"],
        ["伦敦", "倫敦", "London"],
        "Europe/London",
    ),
    (
        ["中欧时间", "中歐時間", "Central European Time"],
        ["柏林、巴黎", "柏林、巴黎", "Berlin, Paris"],
        "Europe/Berlin",
    ),
    (
        ["美国东部时间", "美國東部時間", "Eastern Time"],
        ["纽约", "紐約", "New York"],
        "America/New_York",
    ),
    (
        ["美国太平洋时间", "美國太平洋時間", "Pacific Time"],
        ["洛杉矶", "洛杉磯", "Los Angeles"],
        "America/Los_Angeles",
    ),
];

impl App {
    pub(super) fn set_slider(&mut self, t: &Theme, which: Slider, track: Rect, x: f32) {
        self.set_level(t, which, track, (x - track.x) / track.w);
    }
}

impl App {
    /// Where a slider stands, as a share of its span.
    pub(super) fn level(&self, which: Slider) -> f32 {
        match which {
            Slider::Brightness => self.state.brightness,
            Slider::Vibration => self.state.vibration,
        }
    }
}

impl App {
    /// Set a slider to `level` of its span, the brightness no lower than
    /// its floor, and draw the slider at `track` on the page again.
    pub(super) fn set_level(&mut self, t: &Theme, which: Slider, track: Rect, level: f32) {
        let value = match which {
            Slider::Brightness => &mut self.state.brightness,
            Slider::Vibration => &mut self.state.vibration,
        };
        *value = level.clamp(if which == Slider::Brightness { MIN_BRIGHTNESS } else { 0.0 }, 1.0);
        let value = *value;
        if self.screen() == Screen::ControlCenter {
            self.refresh();
            return;
        }
        if let Some(fr) = self.frame.as_mut() {
            fr.canvas.fill_rect(track.x, track.y, track.w, track.h, t.pal.card);
            widgets::slider(&mut fr.canvas, t, track.x, track.y, track.w, value);
        }
        self.layer = None;
        self.dirty = true;
    }
}

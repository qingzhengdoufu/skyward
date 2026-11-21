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
    (texts::time_zone::name::CHINA, texts::time_zone::cities::CHINA, "Asia/Shanghai"),
    (texts::time_zone::name::HONG_KONG, texts::time_zone::cities::HONG_KONG, "Asia/Hong_Kong"),
    (texts::time_zone::name::TAIPEI, texts::time_zone::cities::TAIPEI, "Asia/Taipei"),
    (texts::time_zone::name::JAPAN, texts::time_zone::cities::JAPAN, "Asia/Tokyo"),
    (texts::time_zone::name::UTC, same("UTC"), "UTC"),
    (texts::time_zone::name::BRITAIN, texts::time_zone::cities::BRITAIN, "Europe/London"),
    (texts::time_zone::name::CENTRAL_EUROPE, texts::time_zone::cities::CENTRAL_EUROPE, "Europe/Berlin"),
    (texts::time_zone::name::US_EASTERN, texts::time_zone::cities::US_EASTERN, "America/New_York"),
    (texts::time_zone::name::US_PACIFIC, texts::time_zone::cities::US_PACIFIC, "America/Los_Angeles"),
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

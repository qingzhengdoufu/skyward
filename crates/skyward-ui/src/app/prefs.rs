use super::*;
use anyhow::{Context, Result};
use jiff::tz::TimeZone;

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
    /// Keep the time zone the installed system is set to.
    SystemTimeZone,
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

/// A part of the world the time zone page lists its zones under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Americas,
    EuropeAfrica,
    /// Eleven zones of one country, which its people look for under it.
    Russia,
    Asia,
    Oceania,
}

impl Region {
    /// The regions, in the order the time zone page lists them.
    pub const ALL: [Region; 5] = [
        Region::Americas,
        Region::EuropeAfrica,
        Region::Russia,
        Region::Asia,
        Region::Oceania,
    ];

    pub fn name(self) -> Text {
        match self {
            Region::Americas => texts::time_zone::region::AMERICAS,
            Region::EuropeAfrica => texts::time_zone::region::EUROPE_AFRICA,
            Region::Russia => texts::time_zone::region::RUSSIA,
            Region::Asia => texts::time_zone::region::ASIA,
            Region::Oceania => texts::time_zone::region::OCEANIA,
        }
    }
}

/// A time zone the clock and the times on the pages can keep.
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub name: Text,
    /// The cities it is known by.
    pub cities: Text,
    /// Its name in the tz database, whose rules it keeps.
    pub tz: &'static str,
    /// The other names in the tz database whose places keep the same time
    /// under the zone's name: a system set to one of them shows as the
    /// zone, and keeps the rules of the name it is set to.
    pub also: &'static [&'static str],
    /// The part of the world its page lists it under; none for the one
    /// zone that is no place's own.
    pub region: Option<Region>,
}

const fn zone(region: Region, name: Text, cities: Text, tz: &'static str, also: &'static [&'static str]) -> Zone {
    Zone {
        name,
        cities,
        tz,
        also,
        region: Some(region),
    }
}

/// The time zones the clock and the times on the pages can keep, each
/// region's from west to east.
pub static TIME_ZONES: [Zone; 62] = {
    use Region::{Americas, Asia, EuropeAfrica, Oceania, Russia};
    use texts::time_zone::{cities as c, name as n};
    [
        zone(Americas, n::HAWAII, c::HAWAII, "Pacific/Honolulu", &[]),
        zone(
            Americas,
            n::ALASKA,
            c::ALASKA,
            "America/Anchorage",
            &["America/Juneau", "America/Sitka", "America/Nome"],
        ),
        zone(Americas, n::US_PACIFIC, c::US_PACIFIC, "America/Los_Angeles", &["America/Tijuana"]),
        zone(Americas, n::US_MOUNTAIN, c::US_MOUNTAIN, "America/Denver", &["America/Boise"]),
        zone(Americas, n::US_CENTRAL, c::US_CENTRAL, "America/Chicago", &["America/Menominee"]),
        zone(
            Americas,
            n::MEXICO,
            c::MEXICO,
            "America/Mexico_City",
            &["America/Monterrey", "America/Merida"],
        ),
        zone(
            Americas,
            n::US_EASTERN,
            c::US_EASTERN,
            "America/New_York",
            &["America/Toronto", "America/Detroit"],
        ),
        zone(Americas, n::COLOMBIA, c::COLOMBIA, "America/Bogota", &[]),
        zone(Americas, n::PERU, c::PERU, "America/Lima", &[]),
        zone(Americas, n::VENEZUELA, c::VENEZUELA, "America/Caracas", &[]),
        zone(
            Americas,
            n::AMAZON,
            c::AMAZON,
            "America/Manaus",
            &["America/Porto_Velho", "America/Boa_Vista", "America/Cuiaba", "America/Campo_Grande"],
        ),
        zone(Americas, n::CHILE, c::CHILE, "America/Santiago", &[]),
        zone(
            Americas,
            n::BRASILIA,
            c::BRASILIA,
            "America/Sao_Paulo",
            &[
                "America/Bahia",
                "America/Fortaleza",
                "America/Recife",
                "America/Belem",
                "America/Maceio",
                "America/Araguaina",
                "America/Santarem",
            ],
        ),
        zone(
            Americas,
            n::ARGENTINA,
            c::ARGENTINA,
            "America/Argentina/Buenos_Aires",
            &[
                "America/Buenos_Aires",
                "America/Argentina/Cordoba",
                "America/Cordoba",
                "America/Argentina/Mendoza",
                "America/Mendoza",
            ],
        ),
        zone(
            EuropeAfrica,
            n::BRITAIN,
            c::BRITAIN,
            "Europe/London",
            &["Europe/Belfast", "Europe/Jersey", "Europe/Guernsey", "Europe/Isle_of_Man"],
        ),
        zone(
            EuropeAfrica,
            n::CENTRAL_EUROPE,
            c::CENTRAL_EUROPE,
            "Europe/Berlin",
            &[
                "Europe/Paris",
                "Europe/Rome",
                "Europe/Madrid",
                "Europe/Warsaw",
                "Europe/Amsterdam",
                "Europe/Brussels",
                "Europe/Vienna",
                "Europe/Zurich",
                "Europe/Prague",
                "Europe/Budapest",
                "Europe/Stockholm",
                "Europe/Oslo",
                "Europe/Copenhagen",
                "Europe/Belgrade",
                "Europe/Zagreb",
                "Europe/Ljubljana",
                "Europe/Bratislava",
                "Europe/Luxembourg",
                "Europe/Malta",
                "Europe/Monaco",
                "Europe/Tirane",
                "Europe/Sarajevo",
                "Europe/Skopje",
                "Europe/Podgorica",
                "Europe/Vaduz",
                "Europe/Andorra",
                "Europe/San_Marino",
                "Europe/Vatican",
                "Europe/Busingen",
                "Europe/Gibraltar",
            ],
        ),
        zone(
            EuropeAfrica,
            n::WEST_AFRICA,
            c::WEST_AFRICA,
            "Africa/Lagos",
            &[
                "Africa/Kinshasa",
                "Africa/Luanda",
                "Africa/Douala",
                "Africa/Libreville",
                "Africa/Niamey",
                "Africa/Ndjamena",
                "Africa/Brazzaville",
                "Africa/Bangui",
                "Africa/Malabo",
                "Africa/Porto-Novo",
            ],
        ),
        zone(
            EuropeAfrica,
            n::EASTERN_EUROPE,
            c::EASTERN_EUROPE,
            "Europe/Kyiv",
            &[
                "Europe/Kiev",
                "Europe/Athens",
                "Europe/Helsinki",
                "Europe/Bucharest",
                "Europe/Sofia",
                "Europe/Riga",
                "Europe/Vilnius",
                "Europe/Tallinn",
                "Europe/Mariehamn",
                "Asia/Nicosia",
                "Europe/Nicosia",
                "Europe/Uzhgorod",
                "Europe/Zaporozhye",
            ],
        ),
        zone(EuropeAfrica, n::EGYPT, c::EGYPT, "Africa/Cairo", &[]),
        zone(
            EuropeAfrica,
            n::SOUTH_AFRICA,
            c::SOUTH_AFRICA,
            "Africa/Johannesburg",
            &["Africa/Maseru", "Africa/Mbabane"],
        ),
        zone(EuropeAfrica, n::TURKEY, c::TURKEY, "Europe/Istanbul", &["Asia/Istanbul", "Turkey"]),
        zone(
            EuropeAfrica,
            n::EAST_AFRICA,
            c::EAST_AFRICA,
            "Africa/Nairobi",
            &[
                "Africa/Addis_Ababa",
                "Africa/Dar_es_Salaam",
                "Africa/Kampala",
                "Africa/Mogadishu",
                "Africa/Djibouti",
                "Africa/Asmara",
            ],
        ),
        zone(Russia, n::KALININGRAD, c::KALININGRAD, "Europe/Kaliningrad", &[]),
        zone(
            Russia,
            n::MOSCOW,
            c::MOSCOW,
            "Europe/Moscow",
            &["Europe/Kirov", "Europe/Volgograd", "W-SU"],
        ),
        zone(
            Russia,
            n::SAMARA,
            c::SAMARA,
            "Europe/Samara",
            &["Europe/Astrakhan", "Europe/Saratov", "Europe/Ulyanovsk"],
        ),
        zone(Russia, n::YEKATERINBURG, c::YEKATERINBURG, "Asia/Yekaterinburg", &[]),
        zone(Russia, n::OMSK, c::OMSK, "Asia/Omsk", &[]),
        zone(
            Russia,
            n::KRASNOYARSK,
            c::KRASNOYARSK,
            "Asia/Krasnoyarsk",
            &["Asia/Novosibirsk", "Asia/Novokuznetsk", "Asia/Barnaul", "Asia/Tomsk"],
        ),
        zone(Russia, n::IRKUTSK, c::IRKUTSK, "Asia/Irkutsk", &[]),
        zone(Russia, n::YAKUTSK, c::YAKUTSK, "Asia/Yakutsk", &["Asia/Chita", "Asia/Khandyga"]),
        zone(Russia, n::VLADIVOSTOK, c::VLADIVOSTOK, "Asia/Vladivostok", &["Asia/Ust-Nera"]),
        zone(
            Russia,
            n::MAGADAN,
            c::MAGADAN,
            "Asia/Magadan",
            &["Asia/Sakhalin", "Asia/Srednekolymsk"],
        ),
        zone(Russia, n::KAMCHATKA, c::KAMCHATKA, "Asia/Kamchatka", &["Asia/Anadyr"]),
        zone(Asia, n::ISRAEL, c::ISRAEL, "Asia/Jerusalem", &["Asia/Tel_Aviv", "Israel"]),
        zone(
            Asia,
            n::ARABIA,
            c::ARABIA,
            "Asia/Riyadh",
            &["Asia/Kuwait", "Asia/Qatar", "Asia/Bahrain", "Asia/Baghdad", "Asia/Aden"],
        ),
        zone(Asia, n::IRAN, c::IRAN, "Asia/Tehran", &["Iran"]),
        zone(Asia, n::GULF, c::GULF, "Asia/Dubai", &["Asia/Muscat"]),
        zone(Asia, n::PAKISTAN, c::PAKISTAN, "Asia/Karachi", &[]),
        zone(Asia, n::UZBEKISTAN, c::UZBEKISTAN, "Asia/Tashkent", &["Asia/Samarkand"]),
        zone(
            Asia,
            n::KAZAKHSTAN,
            c::KAZAKHSTAN,
            "Asia/Almaty",
            &[
                "Asia/Qyzylorda",
                "Asia/Aqtobe",
                "Asia/Aqtau",
                "Asia/Atyrau",
                "Asia/Oral",
                "Asia/Qostanay",
            ],
        ),
        zone(Asia, n::INDIA, c::INDIA, "Asia/Kolkata", &["Asia/Calcutta"]),
        zone(Asia, n::NEPAL, c::NEPAL, "Asia/Kathmandu", &["Asia/Katmandu"]),
        zone(Asia, n::BANGLADESH, c::BANGLADESH, "Asia/Dhaka", &["Asia/Dacca"]),
        zone(Asia, n::MYANMAR, c::MYANMAR, "Asia/Yangon", &["Asia/Rangoon"]),
        zone(
            Asia,
            n::INDOCHINA,
            c::INDOCHINA,
            "Asia/Bangkok",
            &["Asia/Ho_Chi_Minh", "Asia/Saigon", "Asia/Phnom_Penh", "Asia/Vientiane"],
        ),
        zone(
            Asia,
            n::INDONESIA_WESTERN,
            c::INDONESIA_WESTERN,
            "Asia/Jakarta",
            &["Asia/Pontianak"],
        ),
        zone(
            Asia,
            n::CHINA,
            c::CHINA,
            "Asia/Shanghai",
            &["Asia/Chongqing", "Asia/Chungking", "Asia/Harbin", "PRC"],
        ),
        zone(Asia, n::HONG_KONG, c::HONG_KONG, "Asia/Hong_Kong", &["Hongkong"]),
        zone(Asia, n::TAIPEI, c::TAIPEI, "Asia/Taipei", &["ROC"]),
        zone(
            Asia,
            n::SINGAPORE,
            c::SINGAPORE,
            "Asia/Singapore",
            &["Asia/Kuala_Lumpur", "Asia/Kuching", "Singapore"],
        ),
        zone(Asia, n::PHILIPPINES, c::PHILIPPINES, "Asia/Manila", &[]),
        zone(
            Asia,
            n::INDONESIA_CENTRAL,
            c::INDONESIA_CENTRAL,
            "Asia/Makassar",
            &["Asia/Ujung_Pandang"],
        ),
        zone(Asia, n::JAPAN, c::JAPAN, "Asia/Tokyo", &["Japan"]),
        zone(Asia, n::KOREA, c::KOREA, "Asia/Seoul", &["ROK"]),
        zone(Asia, n::INDONESIA_EASTERN, c::INDONESIA_EASTERN, "Asia/Jayapura", &[]),
        zone(
            Oceania,
            n::AUSTRALIA_WESTERN,
            c::AUSTRALIA_WESTERN,
            "Australia/Perth",
            &["Australia/West"],
        ),
        zone(
            Oceania,
            n::AUSTRALIA_CENTRAL_STANDARD,
            c::AUSTRALIA_CENTRAL_STANDARD,
            "Australia/Darwin",
            &["Australia/North"],
        ),
        zone(
            Oceania,
            n::AUSTRALIA_CENTRAL,
            c::AUSTRALIA_CENTRAL,
            "Australia/Adelaide",
            &["Australia/South", "Australia/Broken_Hill"],
        ),
        zone(
            Oceania,
            n::AUSTRALIA_EASTERN_STANDARD,
            c::AUSTRALIA_EASTERN_STANDARD,
            "Australia/Brisbane",
            &["Australia/Queensland", "Australia/Lindeman"],
        ),
        zone(
            Oceania,
            n::AUSTRALIA_EASTERN,
            c::AUSTRALIA_EASTERN,
            "Australia/Sydney",
            &[
                "Australia/Melbourne",
                "Australia/Canberra",
                "Australia/Hobart",
                "Australia/ACT",
                "Australia/NSW",
                "Australia/Victoria",
                "Australia/Tasmania",
            ],
        ),
        zone(Oceania, n::NEW_ZEALAND, c::NEW_ZEALAND, "Pacific/Auckland", &["NZ"]),
        Zone {
            name: n::UTC,
            cities: same("UTC"),
            tz: UTC,
            also: &[
                "Etc/UTC",
                "Etc/UCT",
                "UCT",
                "Etc/Universal",
                "Universal",
                "Etc/Zulu",
                "Zulu",
                "Etc/GMT",
                "GMT",
                "Etc/Greenwich",
                "Greenwich",
            ],
            region: None,
        },
    ]
};

/// The name of Coordinated Universal Time in the tz database.
pub const UTC: &str = "UTC";

/// The zone the clock keeps until the installed system's is known or
/// another is chosen, by its name in the tz database.
pub const FIRST_ZONE: &str = "Asia/Shanghai";

/// Where the zone the tz database names `tz` is in [`TIME_ZONES`], by its
/// own name or one of those that keep its time under its name.
pub fn listed(tz: &str) -> Option<usize> {
    TIME_ZONES.iter().position(|zone| zone.tz == tz || zone.also.contains(&tz))
}

/// The time zone the installed system is set to.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemZone {
    /// Its name in the tz database.
    pub name: String,
    pub rules: TimeZone,
}

impl SystemZone {
    /// The zone the tz database names `name`, with the rules the recovery
    /// carries for it.
    pub fn named(name: &str) -> Result<SystemZone> {
        let rules = TimeZone::get(name).with_context(|| format!("the recovery carries no rules for the time zone {name:?}"))?;
        Ok(SystemZone {
            name: name.to_string(),
            rules,
        })
    }
}

impl State {
    /// The installed system's time zone, while the clock keeps it.
    pub fn followed(&self) -> Option<&SystemZone> {
        self.system_zone.as_ref().filter(|_| self.follow_system_zone)
    }

    /// The rules of the zone the clock and the times on the pages keep:
    /// the installed system's while it is followed and known, and the one
    /// chosen otherwise.
    pub fn zone(&self) -> &TimeZone {
        match self.followed() {
            Some(system) => &system.rules,
            None => fmt::rules(self.time_zone),
        }
    }

    /// Where the zone the clock keeps is in [`TIME_ZONES`]; none for a
    /// zone of the installed system's that is not listed.
    pub fn zone_listed(&self) -> Option<usize> {
        match self.followed() {
            Some(system) => listed(&system.name),
            None => Some(self.time_zone),
        }
    }

    /// What the zone the clock keeps is called: its name where it is
    /// listed, and otherwise the name the tz database gives it.
    pub fn zone_name(&self) -> &str {
        match (self.zone_listed(), self.followed()) {
            (Some(at), _) => self.lang.pick(TIME_ZONES[at].name),
            (None, Some(system)) => &system.name,
            (None, None) => unreachable!("a zone chosen is one listed"),
        }
    }
}

impl App {
    /// The installed system is set to the time zone the tz database names
    /// `name`: the clock keeps it while the system's zone is followed.
    pub fn set_system_zone(&mut self, name: &str) -> Result<()> {
        let zone = SystemZone::named(name)?;
        if self.state.system_zone.as_ref() != Some(&zone) {
            self.state.system_zone = Some(zone);
            self.refresh();
        }
        Ok(())
    }

    /// The time is `unix`, in Unix seconds. The time zone pages tell how
    /// far each zone is from UTC now, to the minute, and are drawn again
    /// when a zone's clocks go forward or back.
    pub fn set_now(&mut self, unix: i64) {
        let minute = unix - unix.rem_euclid(60);
        if minute == self.state.now {
            return;
        }
        let before = self.offsets();
        self.state.now = minute;
        if matches!(self.screen(), Screen::TimeZone | Screen::TimeZones(_)) && self.offsets() != before {
            self.refresh();
        }
    }

    /// How far from UTC each zone the time zone pages tell of is now.
    fn offsets(&self) -> Vec<i32> {
        let now = self.state.now;
        let listed = (0..TIME_ZONES.len()).map(|at| fmt::utc_offset_seconds(now, fmt::rules(at)));
        listed
            .chain(
                self.state
                    .system_zone
                    .iter()
                    .map(|system| fmt::utc_offset_seconds(now, &system.rules)),
            )
            .collect()
    }

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

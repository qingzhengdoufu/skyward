use crate::app::RESTART_AFTER_INSTALL;
use crate::app::{Region, UTC, listed};
use crate::texts;
use crate::app::{
    ADDABLE, Action, BACKUP_PARTS, BackupAt, BackupEnd, BackupJob, Check, ClashChoice, Confirm, DATA_ERASED, DATA_PARTITION, DOWNLOADS,
    Damage, Data, Dock, DownloadEnd, EraseStop, FASTBOOT_CHOICES, Field, FileKind, FileOp, FileRow, FileTask, Fold, Folder, Found, Frame,
    FsAction, FsChoice, FsRow, Have, HeroArt, Hit, Install, JobEnd, Keys, Pick, Place, Rows, SCREEN_OFF, Screen, SkywardCheck, SkywardEnd,
    SkywardJob, SkywardStage, Slider, SortKey, Source, State, Switch, SystemsWork, TARGETS, TIME_ZONES, Target, USB_DRIVE_UNREADABLE,
    USB_DRIVE_WANTED, UsbDrive, UsbLink, WIPES, Wipe, installs, starting,
};
use crate::canvas::Canvas;
use crate::color::Rgba;
use crate::editor;
use crate::files;
use crate::fmt::{self, Verb};
use crate::font::Fonts;
use crate::geom::Rect;
use crate::icons::{self, Icon};
use crate::lang::{Lang, Text};
use crate::search;
use crate::term::{self, Term};
use crate::theme::Theme;
use crate::tr;
use crate::widgets::{self, Caption, Detail, Fact, Outline, Row, Spec, SystemCard, SystemMark, Tone, Trail};
use skyward_info::{DeviceInfo, VERSION};
use skyward_install::{Outcome, Reflash, Refusal, Stage};
use skyward_net::download::Progress;
use skyward_net::supplicant::Security;
use skyward_net::wlan::{Bands, Network, Power, Stage as Joining};
use skyward_systems::{Held, Who};
use skyward_vab::{Slot, Standing};
use std::path::Path;

// The colours the stock Settings fills its icons' tiles with, the same in
// light and dark mode.
const BLUE: Rgba = Rgba::rgb(0x34, 0x82, 0xff);
const GREEN: Rgba = Rgba::rgb(0x1d, 0xcd, 0x3a);
const PURPLE: Rgba = Rgba::rgb(0x77, 0x67, 0xf9);
const SLATE: Rgba = Rgba::rgb(0x8c, 0x9d, 0xb0);
const ORANGE: Rgba = Rgba::rgb(0xff, 0x69, 0x00);
const RED: Rgba = Rgba::rgb(0xfa, 0x38, 0x2e);

/// What every page draws from: style, typefaces, the device readings, and
/// what the pages hold, from the choices made on them to where the data
/// partition stands.
pub struct Ctx<'a> {
    pub t: &'a Theme,
    pub f: &'a Fonts,
    pub info: &'a DeviceInfo,
    pub state: &'a State,
    /// The shell the terminal page shows, once one has started.
    pub term: Option<&'a Term>,
    /// The part of the page to draw; all of it if none is given. The page
    /// is laid out whole either way.
    pub band: Option<Band>,
}

/// Rows `rows` of a page, and of them the columns `cols`, each from the
/// first to one past the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Band {
    pub rows: (u32, u32),
    pub cols: (u32, u32),
}

pub fn render(screen: Screen, cx: &Ctx) -> Frame {
    match screen {
        Screen::RootModules => crate::tools::modules(cx),
        Screen::Partitions => crate::tools::partitions(cx),
        Screen::ControlCenter => crate::tools::control(cx),
        Screen::Tasks => crate::tools::tasks(cx),
        Screen::Browser => crate::browser::screen(cx),
        Screen::BrowserAddress => crate::browser::address(cx),
        Screen::BrowserDownloads => crate::browser::downloads(cx),
        Screen::BrowserForm => crate::browser::form(cx),
        Screen::BrowserFull => crate::browser::full(cx),
        Screen::BrowserTabs => crate::browser::tabs(cx),
        Screen::BrowserQuick => crate::browser::quick(cx),
        Screen::BrowserMenu => crate::browser::menu(cx),
        Screen::BrowserHistory => crate::browser::marks(cx, true),
        Screen::BrowserBookmarks => crate::browser::marks(cx, false),
        Screen::Decrypt => decrypt(cx),
        Screen::Home => home(cx),
        Screen::Settings => settings(cx),
        Screen::Reboot => reboot(cx),
        Screen::Wipe => wipe(cx),
        Screen::Mount => mount(cx),
        Screen::Install => install(cx),
        Screen::Choose(place) => choose(cx, place),
        Screen::Installing => installing(cx),
        Screen::About => about(cx),
        Screen::Specs => specs(cx),
        Screen::ScreenOff => screen_off(cx),
        Screen::Search => search(cx),
        Screen::Backup => backup(cx),
        Screen::ImageBackup => image_backup(cx),
        Screen::Restore => restore(cx),
        Screen::RestoreBackup => restore_backup(cx),
        Screen::BackupJob => backup_job(cx),
        Screen::Advanced => advanced(cx),
        Screen::Avb => avb(cx),
        Screen::Files => files(cx),
        Screen::FileInfo => file_info(cx),
        Screen::Permissions => permissions(cx),
        Screen::Editor => editor(cx),
        Screen::Terminal => terminal(cx),
        Screen::Relabel => relabel(cx),
        Screen::Filesystems => filesystems(cx),
        Screen::Filesystem => filesystem(cx),
        Screen::PartWork => part_work(cx),
        Screen::Language => language(cx),
        Screen::TimeZone => time_zone(cx),
        Screen::TimeZones(region) => time_zones(cx, region),
        Screen::Logs => logs(cx),
        Screen::Update => update(cx),
        Screen::SkywardUpdate => skyward_update(cx),
        Screen::License => license(cx),
        Screen::FontLicense => font_license(cx),
        Screen::Slot => slot(cx),
        Screen::Systems => systems(cx),
        Screen::Fastboot => fastboot(cx),
        Screen::Erase => erase(cx),
        Screen::CantStart => cant_start(cx),
        Screen::Unhandled => unhandled(cx),
        Screen::Rename => rename(cx),
        Screen::Clash => clash(cx),
        Screen::FileJob => file_job(cx),
        Screen::Wlan => wlan(cx),
        Screen::WlanPassword => wlan_password(cx),
        Screen::WlanNetwork => wlan_network(cx),
        Screen::WlanAdd => wlan_add(cx),
        Screen::PowerOff => power_off(cx),
        Screen::Vbmeta => vbmeta(cx),
    }
}

/// Label and colour of each swipe-to-confirm control: the filesystem
/// page's as `s` says what it is to do.
pub fn confirm_label(which: Confirm, s: &State) -> (&'static str, bool) {
    const INSTALL: Text = crate::texts::confirm::INSTALL;
    let (label, danger) = match which {
        Confirm::Avb => (crate::lang::legacy(["滑动以修改 vbmeta", "滑動以變更 vbmeta", "Slide to change vbmeta"]), true),
        Confirm::Filesystem => match s.fs_action {
            FsAction::Repair => (crate::texts::confirm::REPAIR, true),
            FsAction::Remake(_) => (crate::texts::confirm::CHANGE, true),
        },
        Confirm::Reboot => (crate::texts::confirm::REBOOT, false),
        Confirm::Wipe => (crate::texts::confirm::WIPE, true),
        Confirm::Flash => (crate::texts::confirm::FLASH, true),
        Confirm::Backup => (crate::texts::confirm::BACK_UP, false),
        Confirm::SkywardUpdate => (crate::lang::legacy(["滑动以更新 Recovery", "滑動以更新 Recovery", "Slide to update recovery"]), true),
        Confirm::ImageBackup => (crate::lang::legacy(["滑动以导出镜像", "滑動以匯出映像檔", "Slide to export image"]), false),
        Confirm::Restore => (crate::texts::confirm::RESTORE, true),
        Confirm::Slot => (crate::texts::confirm::SWITCH, true),
        Confirm::FactoryReset => (crate::texts::confirm::FACTORY_RESET, true),
        Confirm::DeleteEntry | Confirm::DeleteBackup => (crate::texts::confirm::DELETE, true),
        Confirm::Install => (INSTALL, false),
        Confirm::InstallWipe => (crate::texts::confirm::INSTALL_AND_WIPE, true),
        Confirm::RunScript => (INSTALL, true),
        Confirm::Merge => (crate::texts::confirm::MERGE, true),
        Confirm::Relabel => (crate::texts::confirm::FIX, true),
        Confirm::SwitchSystem => (crate::texts::confirm::SWITCH_AND_REBOOT, false),
        Confirm::RemoveSystem => (crate::texts::confirm::REMOVE, true),
        Confirm::InstallSystem => (INSTALL, false),
        Confirm::AvbDisable => (crate::lang::legacy(["滑动以关闭校验", "滑動以關閉驗證", "Slide to turn the check off"]), true),
        Confirm::VbmetaRestore => (crate::lang::legacy(["滑动以写回备份", "滑動以寫回備份", "Slide to write the copy back"]), true),
        Confirm::ForgetNetwork => (crate::lang::legacy(["滑动以忘记网络", "滑動以忘記網路", "Slide to forget the network"]), true),
        Confirm::DeleteDownload => (crate::lang::legacy(["滑动以删除安装包", "滑動以刪除安裝檔", "Slide to delete the package"]), true),
        Confirm::PowerOff => (crate::lang::legacy(["滑动以关机", "滑動以關機", "Slide to power off"]), false),
    };
    (s.lang.pick(label), danger)
}

/// The home page's large title, and what its search bar and the search
/// page's field say while nothing is typed.
pub const HOME_TITLE: &str = "Skyward";
pub const SEARCH_HINT: Text = crate::texts::search::HINT;

/// Captions under the readings on the home card's strip.
const DATA_STATE: Text = crate::lang::legacy(["数据分区", "資料分割區", "Data partition"]);
const FREE_SPACE: Text = crate::lang::legacy(["可用空间", "可用空間", "Free space"]);

const REBOOT_TARGETS: [(Text, Option<Text>); 6] = [
    (crate::texts::reboot::target::SYSTEM, None),
    (crate::texts::reboot::target::RECOVERY, None),
    (
        crate::lang::same("Fastboot"),
        Some(crate::texts::reboot::target::FASTBOOT_NOTE),
    ),
    (crate::lang::same("Bootloader"), None),
    (
        crate::lang::same("EDL"),
        Some(crate::texts::reboot::target::EDL_NOTE),
    ),
    (crate::texts::reboot::target::POWER_OFF, None),
];

/// What pages are called both on the rows that open them and in their own
/// titles.
const WLAN: Text = crate::texts::page::WLAN;
const DECRYPT: Text = crate::texts::page::DECRYPT;
const ADVANCED: Text = crate::texts::page::ADVANCED;
const SETTINGS: Text = crate::texts::page::SETTINGS;
const REBOOT: Text = crate::texts::page::REBOOT;
const ABOUT: Text = crate::texts::page::ABOUT;
const SPECS: Text = crate::texts::page::SPECS;
const LOGS: Text = crate::texts::page::LOGS;
const SCREEN_TIMEOUT: Text = crate::texts::page::SCREEN_TIMEOUT;
const ALWAYS_ON: Text = crate::lang::legacy(["息屏显示", "螢幕關閉顯示", "Always-on display"]);
const DOUBLE_TAP: Text = crate::lang::legacy(["双击亮屏", "連點兩下亮屏", "Double-tap to wake"]);
const REMEMBER_PASSWORD: Text = crate::lang::legacy(["记住解密密码", "記住解密密碼", "Remember the decrypt password"]);
const AVB: Text = crate::lang::legacy(["AVB 校验", "AVB 驗證", "AVB verification"]);
const UPDATE: Text = crate::texts::page::UPDATE;
const SKYWARD_UPDATE: Text = crate::texts::page::SKYWARD_UPDATE;
const FILES: Text = crate::texts::page::FILES;
const TERMINAL: Text = crate::texts::page::TERMINAL;
const LANGUAGE: Text = crate::texts::page::LANGUAGE;
const TIME_ZONE: Text = crate::texts::page::TIME_ZONE;
const LICENSE: Text = crate::texts::page::LICENSE;
const FONTS: Text = crate::texts::page::FONTS;
const DUAL_BOOT: Text = crate::texts::page::DUAL_BOOT;

/// A canvas for a page `h` tall in the colour `bg`, holding the rows `cx`
/// asks to be drawn.
fn sheet(cx: &Ctx, h: u32, bg: Rgba) -> Canvas {
    let w = cx.t.m.w;
    let Band { rows, cols } = cx.band.unwrap_or(Band {
        rows: (0, h),
        cols: (0, w),
    });
    Canvas::band(w, h, rows.0..rows.1, cols.0..cols.1, bg)
}

fn page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h, cx.t.pal.bg)
}

/// A canvas tall enough for a scrolling page; [`fit`] trims it to the content.
fn long_page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h * 3, cx.t.pal.bg)
}

/// How tall a long page whose content ends at `bottom` is: to the bottom
/// margin after it, and never shorter than one screen.
fn fitted(t: &Theme, bottom: f32) -> u32 {
    ((bottom + t.m.margin * 2.0).ceil() as u32).max(t.m.h)
}

/// Trim a long page to its content plus the bottom margin, and never shorter
/// than one screen.
fn fit(c: Canvas, t: &Theme, bottom: f32) -> Canvas {
    c.crop_height(fitted(t, bottom))
}

fn hits(rects: &[Rect], actions: &[Action]) -> Vec<Hit> {
    // One action for each row, and no row without one: a list given one
    // action fewer than it draws leaves its last row dead, which is a control
    // that does nothing when it is touched and says nothing about why.
    assert_eq!(
        rects.len(),
        actions.len(),
        "a list draws {} rows and was given {} actions",
        rects.len(),
        actions.len()
    );
    rects.iter().zip(actions).map(|(&rect, &action)| Hit { rect, action }).collect()
}

/// Recovery home, laid out like the Settings root: title, search, a device
/// card, then grouped entries. The card's strip carries the readings: the
/// slot, the data partition's lock, the room left on it and the battery; the
/// status bar adds the battery's temperature.
pub fn home(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    // The search bar is left room on the page and drawn over it, folding
    // away as the page scrolls up.
    let bar = Rect::new(t.m.margin, t.m.content_top, t.m.w as f32 - t.m.margin * 2.0, t.m.search_h);
    let mut y = bar.y + bar.h + t.m.after_search;
    let cards_top = y;
    let fold = Fold {
        bar,
        hint: l.pick(SEARCH_HINT),
        span: cards_top - bar.y,
    };
    out.push(Hit {
        rect: bar,
        action: Action::Open(Screen::Search),
    });

    let sub = match &i.os_version {
        Some(v) => format!("{} · {v}", i.codename),
        None => i.codename.clone(),
    };
    // Under the phone's name the card holds only what asks for something to
    // be done: a battery too low to install on, an update waiting to merge,
    // and a restart into the slot the system does not run from now.
    let low = match i.battery.as_ref().and_then(skyward_install::battery_refusal) {
        Some(Refusal::Battery { level, charging }) => Some(fmt::battery_low(l, level, charging)),
        _ => None,
    };
    let next = match (i.slot, &cx.state.boot_slots) {
        (Some(now), Some(Ok(slots))) if slots.active != now => Some(fmt::next_boot(l, slots.active, now)),
        _ => None,
    };
    let mut rows = Vec::new();
    let mut opens = Vec::new();
    if cx.state.system_detached && matches!(cx.state.install, Install::Idle | Install::Finished { .. }) {
        rows.push(
            Row::new(l.pick(crate::lang::legacy(["请重启 recovery", "請重新啟動 recovery", "Reboot recovery"])))
                .icon(ORANGE, Icon::Power)
                .subtitle(l.pick(RESTART_AFTER_INSTALL)),
        );
        opens.push(Some(Action::RebootTo(Target::Recovery)));
    }
    if let Some((title, note)) = &low {
        rows.push(Row::new(title).icon(RED, Icon::Battery).subtitle(note).trail(Trail::None));
        opens.push(None);
    }
    if let Some(note) = fmt::snapshot_note(l, &i.snapshot) {
        rows.push(
            Row::new(fmt::snapshot(l, &i.snapshot))
                .icon(ORANGE, Icon::Layers)
                .subtitle(note)
                .trail(Trail::Arrow),
        );
        opens.push(Some(Action::Open(Screen::Update)));
    }
    if let Some((title, note)) = &next {
        rows.push(Row::new(title).icon(ORANGE, Icon::Power).subtitle(note).trail(Trail::Arrow));
        opens.push(Some(Action::Open(Screen::Slot)));
    }
    if cx.state.system_notice.is_some() {
        let why = l.pick(crate::texts::home::TAP_TO_SEE_WHY);
        rows.push(
            Row::new(l.pick(SECOND_FAILED))
                .icon(RED, Icon::Systems)
                .subtitle(why)
                .trail(Trail::Arrow),
        );
        opens.push(Some(Action::Open(Screen::Systems)));
    }
    // The strip of readings under the phone's name: the slot the recovery
    // runs from, the data partition's lock, the room left on it and the
    // battery's level, each as the stock settings home shows them.
    let stats = [
        widgets::Stat {
            value: i
                .slot
                .map(|s| s.letter().to_string())
                .unwrap_or_else(|| l.pick(crate::texts::battery::health::UNKNOWN).to_string()),
            caption: l.pick(CURRENT_SLOT),
        },
        widgets::Stat {
            value: fmt::data(l, &cx.state.data).to_string(),
            caption: l.pick(DATA_STATE),
        },
        widgets::Stat {
            value: i
                .storage
                .free
                .map(fmt::bytes)
                .unwrap_or_else(|| l.pick(crate::texts::battery::health::UNKNOWN).to_string()),
            caption: l.pick(FREE_SPACE),
        },
        widgets::Stat {
            value: i
                .battery
                .as_ref()
                .map(|b| format!("{}%", b.level))
                .unwrap_or_else(|| l.pick(crate::texts::battery::health::UNKNOWN).to_string()),
            caption: i
                .battery
                .as_ref()
                .map(|b| fmt::charge(l, b.charge))
                .unwrap_or_else(|| l.pick(crate::lang::legacy(["无电池", "無電池", "No battery"]))),
        },
    ];
    let card = widgets::device_card(&mut c, t, f, y, &i.model, &sub, &stats, &rows);
    out.push(Hit {
        rect: card.rows[0],
        action: Action::Open(Screen::About),
    });
    out.extend(
        card.rows[1..]
            .iter()
            .zip(&opens)
            .filter_map(|(&rect, open)| open.map(|action| Hit { rect, action })),
    );
    y = card.bottom + t.m.card_gap;
    // On a phone whose WLAN the recovery brings up, the WLAN comes first
    // under the device, as on the stock settings home; with it, where the
    // phone can hold a second system, the two systems.
    let mut rows = Vec::new();
    let mut opens = Vec::new();
    if let Some(w) = &cx.state.wlan {
        rows.push(
            Row::new(l.pick(WLAN))
                .icon(BLUE, Icon::Wlan)
                .trail(Trail::Value(fmt::wlan_value(l, w))),
        );
        opens.push(Action::Open(Screen::Wlan));
    }
    let starts = systems_value(cx);
    if cx.state.offers(Screen::Systems) {
        let row = Row::new(l.pick(DUAL_BOOT)).icon(PURPLE, Icon::Systems);
        rows.push(match &starts {
            Some(value) => row.trail(Trail::Value(value)),
            None => row,
        });
        opens.push(Action::Open(Screen::Systems));
    }
    if !rows.is_empty() {
        let first = widgets::list(&mut c, t, f, y, &rows);
        out.extend(hits(&first.rows, &opens));
        y = first.bottom + t.m.card_gap;
    }
    let tools = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(crate::texts::page::INSTALL)).icon(GREEN, Icon::Install),
            Row::new(l.pick(crate::texts::page::BACKUP)).icon(BLUE, Icon::Backup),
            Row::new(l.pick(crate::texts::page::RESTORE)).icon(PURPLE, Icon::Restore),
            Row::new(l.pick(crate::texts::page::MOUNT)).icon(SLATE, Icon::Mount),
            Row::new(l.pick(DECRYPT))
                .icon(BLUE, Icon::Lock)
                .trail(Trail::Value(fmt::data(l, &cx.state.data))),
            Row::new(l.pick(USB_MANAGER)).icon(PURPLE, Icon::UsbDrive),
        ],
    );
    out.extend(hits(
        &tools.rows,
        &[
            Action::Open(Screen::Install),
            Action::Open(Screen::Backup),
            Action::Open(Screen::Restore),
            Action::Open(Screen::Mount),
            Action::Open(Screen::Decrypt),
            Action::OpenUsbManager,
        ],
    ));
    y = tools.bottom + t.m.card_gap;
    let target = l.pick(REBOOT_TARGETS[cx.state.reboot].0);
    // Turning the phone off sits beside restarting it: the two ways of
    // leaving the recovery, one of which brings it back by itself. A phone
    // is put off from its own page, which says what that means, rather than
    // by a swipe on the restart page that only names it as one destination
    // among six.
    let system = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(crate::texts::page::WIPE)).icon(RED, Icon::Wipe),
            Row::new(l.pick(ADVANCED)).icon(SLATE, Icon::More),
            Row::new(l.pick(SETTINGS)).icon(SLATE, Icon::Gear),
            Row::new(l.pick(REBOOT)).icon(ORANGE, Icon::Power).trail(Trail::Value(target)),
        ],
    );
    out.extend(hits(
        &system.rows,
        &[
            Action::Open(Screen::Wipe),
            Action::Open(Screen::Advanced),
            Action::Open(Screen::Settings),
            Action::Open(Screen::Reboot),
        ],
    ));
    // However little it holds, the page scrolls far enough to fold its bar
    // away.
    let height = fitted(t, system.bottom).max(t.m.h + fold.span.ceil() as u32);
    Frame {
        back: false,
        fold: Some(fold),
        ..Frame::new(t, c.crop_height(height), out, HOME_TITLE, cards_top)
    }
}

/// What the home page says beside its entry to the systems: the one that
/// starts, where a second system is installed, once they are read.
fn systems_value(cx: &Ctx) -> Option<String> {
    let l = cx.state.lang;
    match &cx.state.systems {
        Some(Ok(systems)) => Some(match (&systems.other, starting(systems)) {
            (None, _) => l.pick(crate::texts::home::NO_SECOND_SYSTEM).into(),
            (Some(_), Who::Installed) => systems.installed.name.clone(),
            (Some(other), Who::Other) => other.name.clone(),
        }),
        _ => None,
    }
}

/// What the advanced page says beside its system update entry: a download
/// under way, a newer system found, or where an update the system installed
/// stands.
fn update_value(cx: &Ctx) -> &'static str {
    let (s, l) = (cx.state, cx.state.lang);
    if s.downloading() {
        return l.pick(DOWNLOADING);
    }
    if s.newest()
        .is_some_and(|r| cx.info.os_version.as_deref() != Some(r.version.as_str()))
    {
        return l.pick(crate::texts::update::NEW_VERSION);
    }
    fmt::update_value(l, &s.update)
}

/// Preferences, opening with the stock「显示与亮度」theme previews.
pub fn settings(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let themes = widgets::theme_picker(&mut c, t, f, l, top + t.m.dp(8.9));
    out.extend(hits(&themes.rows, &[Action::Theme { dark: false }, Action::Theme { dark: true }]));
    let mut y = widgets::section(&mut c, t, f, themes.bottom, l.pick(crate::texts::settings::BRIGHTNESS));
    let off = fmt::screen_off(l, SCREEN_OFF[s.screen_off]);
    let mut rows = vec![Row::new(l.pick(SCREEN_TIMEOUT)).trail(Trail::Value(&off))];
    let mut actions = vec![Action::Open(Screen::ScreenOff)];
    if s.double_tap_supported {
        rows.push(Row::new(l.pick(crate::lang::legacy(["双击亮屏", "點兩下喚醒螢幕", "Double-tap to wake"]))).trail(Trail::Switch(s.double_tap)));
        actions.push(Action::Toggle(Switch::DoubleTap));
    }
    let slider = widgets::slider_rows(
        &mut c,
        t,
        f,
        y,
        Caption::Sun,
        l.pick(crate::texts::settings::SCREEN_BRIGHTNESS),
        s.brightness,
        &rows,
    );
    out.push(Hit {
        rect: slider.track,
        action: Action::Slider(Slider::Brightness),
    });
    out.extend(hits(&slider.rows, &actions));
    y = widgets::section(&mut c, t, f, slider.bottom, l.pick(crate::texts::settings::VIBRATION));
    let below = Row::new(l.pick(crate::texts::settings::TOUCH_VIBRATION)).trail(Trail::Switch(s.vibrate));
    let vibration = widgets::slider_card(
        &mut c,
        t,
        f,
        y,
        Caption::Vibrate,
        l.pick(crate::texts::settings::VIBRATION_INTENSITY),
        s.vibration,
        Some(below),
    );
    out.push(Hit {
        rect: vibration.track,
        action: Action::Slider(Slider::Vibration),
    });
    if let Some(r) = vibration.below {
        out.push(Hit {
            rect: r,
            action: Action::Toggle(Switch::Vibrate),
        });
    }
    // A phone whose panel has a doze of its own can keep a little on the glass
    // with the screen off; one whose panel has none shows nothing to offer.
    // The switch starts off, so a phone that has never been to this page puts
    // a dark panel out the way the recovery always did. It sits under the
    // vibration, so the settings above it stay where the pages have always put
    // them.
    let mut y = vibration.bottom;
    if s.panel_dozes {
        y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["息屏显示", "螢幕關閉顯示", "Always-on display"])));
        let on = Row::new(l.pick(ALWAYS_ON)).switch(s.always_on);
        let aod = widgets::list(&mut c, t, f, y, &[on]);
        out.extend(hits(&aod.rows, &[Action::Toggle(Switch::AlwaysOn)]));
        let note = l.pick(crate::lang::legacy([
            "息屏时显示时间与设备状态。",
            "關閉螢幕時顯示時間與裝置狀態。",
            "Show the time and device status while the screen is off.",
        ]));
        y = widgets::footnote(&mut c, t, f, aod.bottom, note);
    }
    y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::settings::GENERAL));
    // Two taps on the dark panel light it, on a panel whose touch controller
    // goes on reporting fingers with the panel off. It costs the phone its
    // suspend — the panel has to keep dozing for the taps to be felt at all,
    // since Xiaomi's driver suspends the touchscreen with the panel's blanking
    // — so it is asked for here rather than shipped on.
    let mut rows = vec![
        Row::new(l.pick(LANGUAGE)).icon(BLUE, Icon::Globe).trail(Trail::Value(l.name())),
        Row::new(l.pick(TIME_ZONE))
            .icon(ORANGE, Icon::Clock)
            .trail(Trail::Value(s.zone_name())),
    ];
    if s.panel_taps && !s.double_tap_supported {
        rows.push(Row::new(l.pick(DOUBLE_TAP)).switch(s.double_tap));
    }
    let general = widgets::list(&mut c, t, f, y, &rows);
    let mut actions = vec![Action::Open(Screen::Language), Action::Open(Screen::TimeZone)];
    if s.panel_taps && !s.double_tap_supported {
        actions.push(Action::Toggle(Switch::DoubleTap));
    }
    out.extend(hits(&general.rows, &actions));
    y = widgets::section(&mut c, t, f, general.bottom, "USB");
    let usb = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(l.pick(crate::texts::settings::FILE_TRANSFER))
            .icon(BLUE, Icon::Usb)
            .switch(s.file_transfer)],
    );
    out.extend(hits(&usb.rows, &[Action::Toggle(Switch::FileTransfer)]));
    let note = l.pick(crate::lang::legacy([
        "解密后可通过 USB 访问内部存储。",
        "解密後可透過 USB 存取內部儲存空間。",
        "Internal storage is available over USB after decryption.",
    ]));
    y = widgets::footnote(&mut c, t, f, usb.bottom, note);
    // A password kept on the phone is a password someone else may read,
    // so it is asked for rather than given, and the row says what it costs.
    if s.data != Data::Erased {
        let remember = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["解密", "解密", "Decryption"])));
        let row = Row::new(l.pick(REMEMBER_PASSWORD)).switch(s.remember_password);
        let rows = widgets::list(&mut c, t, f, remember, &[row]);
        out.extend(hits(&rows.rows, &[Action::Toggle(Switch::RememberPassword)]));
        let note = l.pick(crate::lang::legacy([
            "下次自动解密。密码仅用软件加密，持有文件和设备标识者可能还原；关闭后删除保存的副本。",
            "下次自動解密。密碼僅用軟體加密，持有檔案與裝置識別資訊者可能還原；關閉後刪除儲存的副本。",
            "Decrypt automatically next time. Software-only encryption: the file and device identifiers may reveal the password. Turning this off deletes the saved copy.",
        ]));
        y = widgets::footnote(&mut c, t, f, rows.bottom, note);
    }
    y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::settings::ABOUT));
    let mut rows = vec![Row::new(l.pick(ABOUT)).trail(Trail::Value(VERSION))];
    let mut opens = vec![Action::Open(Screen::About)];
    if s.offers(Screen::SkywardUpdate) {
        let row = Row::new(l.pick(SKYWARD_UPDATE));
        rows.push(match skyward_value(cx) {
            Some(value) => row.trail(Trail::Value(value)),
            None => row,
        });
        opens.push(Action::Open(Screen::SkywardUpdate));
    }
    let about = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&about.rows, &opens));
    Frame::new(t, fit(c, t, about.bottom), out, l.pick(SETTINGS), top)
}

const MIRROR_SECTION: Text = crate::lang::legacy(["更新来源", "更新來源", "Updates"]);
const MIRROR: Text = crate::lang::legacy(["国内镜像", "中國大陸鏡像", "Mirror in the mainland"]);
const MIRROR_NOTE: Text = crate::lang::legacy([
    "GitHub 无法连接时使用镜像，仍校验下载文件。",
    "GitHub 無法連線時使用鏡像，仍驗證下載檔案。",
    "Use a mirror if GitHub is unavailable. Downloads are still verified.",
]);

/// What the settings page says beside its Skyward update entry: an update
/// under way, or a newer release found.
fn skyward_value(cx: &Ctx) -> Option<&'static str> {
    let (s, l) = (cx.state, cx.state.lang);
    if s.updating_skyward() {
        return Some(l.pick(crate::texts::skyward_update::UPDATING));
    }
    match (s.skyward_newest(), &s.skyward_job) {
        (
            _,
            Some(SkywardJob {
                outcome: Some(SkywardEnd::Updated(_)),
                ..
            }),
        ) => Some(l.pick(crate::texts::skyward_update::UPDATED)),
        (Some(n), _) if n.newer && n.build.is_some() => Some(l.pick(crate::texts::update::NEW_VERSION)),
        _ => None,
    }
}

/// How long the screen stays on with nothing touched, opened from the
/// settings.
pub fn screen_off(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let labels = SCREEN_OFF.map(|ms| fmt::screen_off(l, ms));
    let rows: Vec<Row> = labels
        .iter()
        .enumerate()
        .map(|(k, label)| Row::new(label).trail(Trail::Check(s.screen_off == k)))
        .collect();
    let times = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = (0..SCREEN_OFF.len()).map(Action::ScreenOff).collect();
    let note = l.pick(crate::texts::screen_off::NOTE);
    let y = widgets::footnote(&mut c, t, f, times.bottom, note);
    if SCREEN_OFF[s.screen_off].is_none() {
        let note = l.pick(crate::texts::screen_off::STAYS_ON);
        widgets::footnote(&mut c, t, f, y, note);
    }
    Frame::new(t, c, hits(&times.rows, &picks), l.pick(SCREEN_TIMEOUT), top)
}

/// Mount manager: the slot's partitions, each with a switch to mount it
/// read-only or unmount it; one that programs are using says which, and
/// stays.
pub fn mount(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;

    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::mount::PARTITIONS));
    let bottom = match &s.mounts {
        Err(why) => {
            let y = widgets::list(
                &mut c,
                t,
                f,
                y,
                &[Row::new(l.pick(crate::texts::mount::UNREADABLE)).trail(Trail::None)],
            )
            .bottom;
            widgets::footnote(&mut c, t, f, y, why)
        }
        Ok(mounts) if mounts.is_empty() => {
            widgets::list(
                &mut c,
                t,
                f,
                y,
                &[Row::new(l.pick(crate::texts::mount::NONE)).trail(Trail::None)],
            )
            .bottom
        }
        Ok(mounts) => {
            let subs: Vec<String> = mounts
                .iter()
                .map(|m| {
                    let (point, holders) = (&m.point, m.holders.join(l.pick(crate::texts::list::SEPARATOR)));
                    match m.holders.is_empty() {
                        true => point.clone(),
                        false => tr!(
                            l,
                            "{point} · {holders} 正在使用",
                            "{point} · {holders} 正在使用",
                            "{point} · in use by {holders}"
                        ),
                    }
                })
                .collect();
            let rows: Vec<Row> = mounts
                .iter()
                .zip(&subs)
                .map(|(m, sub)| {
                    let trail = if m.holders.is_empty() {
                        Trail::Switch(m.mounted)
                    } else {
                        Trail::Text(l.pick(crate::texts::mount::IN_USE))
                    };
                    Row::new(&m.base).subtitle(sub).trail(trail)
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(list.rows.iter().zip(mounts).enumerate().map(|(i, (&rect, m))| {
                let action = match m.holders.is_empty() {
                    true => Action::MountToggle(i),
                    false => Action::Unavailable(l.pick(crate::texts::mount::CANT_UNMOUNT_IN_USE)),
                };
                Hit { rect, action }
            }));
            let y = match &s.mount_failure {
                Some(failure) => widgets::footnote(&mut c, t, f, list.bottom, &fmt::mount_failure(l, failure)),
                None => list.bottom,
            };
            let about = match cx.info.slot.map(|s| s.letter()) {
                Some(slot) => tr!(
                    l,
                    "仅显示当前槽位 {slot} 在 super 中的分区，挂载后均为只读。",
                    "僅顯示目前槽位 {slot} 在 super 中的分割區，掛載後均為唯讀。",
                    "Only the current slot {slot}'s partitions in super show here, all mounted read-only."
                ),
                None => l
                    .pick(crate::texts::mount::SHOWS)
                    .into(),
            };
            widgets::footnote(&mut c, t, f, y, &about)
        }
    };

    Frame::new(t, fit(c, t, bottom), out, l.pick(crate::texts::page::MOUNT), top)
}

/// Who made the recovery, as the About page names them under its version.
const AUTHOR: &str = "© 2026 Kirk Lin & 红烧豆腐";

/// About this device, modelled on the stock「我的设备」: the wordmark, the
/// version and its author over a moving colour field, and the summary,
/// hardware and link cards scrolling over it. Only the cards are drawn
/// here; the canvas is clear wherever the field shows.
pub fn about(cx: &Ctx) -> Frame {
    let (t, f) = (cx.t, cx.f);
    let m = &t.m;
    // The page is drawn whole, under the field: laid out first to learn
    // how tall it comes to, then drawn that tall.
    let (_, _, bottom) = about_cards(cx, &mut Canvas::band(m.w, m.h * 3, 0..0, 0..0, Rgba { r: 0, g: 0, b: 0, a: 0 }));
    let mut c = Canvas::new(m.w, fitted(t, bottom));
    let (out, summary_top, bottom) = about_cards(cx, &mut c);
    let canvas = fit(c, t, bottom);
    let art = HeroArt {
        wordmark: widgets::wordmark(t, f, "Skyward", m.dp(233.7), m.dp(37.8)),
        version: widgets::version_line(t, f, VERSION),
        author: widgets::author_line(t, f, AUTHOR),
        runs: canvas.runs(),
    };
    Frame {
        header: false,
        hero: Some(art),
        ..Frame::new(t, canvas, out, cx.state.lang.pick(ABOUT), summary_top)
    }
}

const MODEL: Text = crate::texts::specs::MODEL;
const PROCESSOR: Text = crate::texts::specs::PROCESSOR;
const RAM: Text = crate::texts::specs::RAM;
const BATTERY_CAPACITY: Text = crate::texts::specs::BATTERY_CAPACITY;
const SCREEN_SIZE: Text = crate::texts::specs::SCREEN_SIZE;
const RESOLUTION: Text = crate::texts::specs::RESOLUTION;
const OS_VERSION: Text = crate::texts::specs::OS_VERSION;
const ANDROID_VERSION: Text = crate::texts::specs::ANDROID_VERSION;
const CURRENT_SLOT: Text = crate::texts::slot::CURRENT;
const DATA_PART: Text = crate::texts::wipe::part::DATA;

/// The phone's processor by its marketing name in `l`, from the phone's
/// settings, or else by its cores.
fn soc(l: Lang, i: &DeviceInfo) -> Option<&str> {
    i.soc.as_ref().map(|n| l.pick([n.hans.as_str(), n.hant.as_str(), n.en.as_str()]))
}

/// The About page's cards on `c`: what touching them does, where the
/// first starts, and where the last ends.
fn about_cards(cx: &Ctx, c: &mut Canvas) -> (Vec<Hit>, f32, f32) {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let m = &t.m;
    let tone = Tone::hero(t);
    let mut out = Vec::new();

    let storage = fmt::storage_used(&i.storage).unwrap_or_else(|| fmt::capacity(i.storage.total));
    let mut facts = vec![
        Fact {
            title: l.pick(MODEL),
            value: i.model.clone(),
            link: false,
        },
        Fact {
            title: l.pick(["存储空间", "儲存空間", "Storage"]),
            value: storage,
            link: true,
        },
    ];
    if let Some(v) = &i.os_version {
        facts.push(Fact {
            title: l.pick(OS_VERSION),
            value: v.clone(),
            link: false,
        });
    }
    if let Some(r) = &i.android {
        facts.push(Fact {
            title: l.pick(ANDROID_VERSION),
            value: r.clone(),
            link: false,
        });
    }
    let summary = widgets::facts(c, t, f, m.dp(490.7), &facts, &tone);
    out.push(Hit {
        rect: summary.rows[1],
        action: Action::Open(Screen::Specs),
    });

    let processor = soc(l, i).map_or_else(|| fmt::cpu(l, &i.cpu), str::to_string);
    let mut specs = vec![(l.pick(PROCESSOR), processor), (l.pick(RAM), fmt::ram(i.ram_kib))];
    if let Some(design) = i.battery.as_ref().and_then(|b| b.design) {
        specs.push((l.pick(BATTERY_CAPACITY), fmt::mah(design)));
    }
    if let Some(d) = i.screen {
        specs.push((l.pick(SCREEN_SIZE), fmt::inches(d)));
    }
    if let Some(p) = i.panel {
        specs.push((l.pick(RESOLUTION), fmt::panel(p)));
    }
    let y = widgets::spec_card(c, t, f, summary.bottom + m.card_gap, &fmt::model_lines(&i.model), &specs, &tone);
    let links = widgets::list_toned(c, t, f, y + m.card_gap, &[Row::new(l.pick(SPECS)), Row::new(l.pick(LOGS))], &tone);
    out.extend(hits(&links.rows, &[Action::Open(Screen::Specs), Action::Open(Screen::Logs)]));
    (out, summary.rows[0].y, links.bottom)
}

/// Every reading the recovery has, modelled on the stock「全部参数与信息」.
pub fn specs(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let gap = t.m.card_gap;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    // This page leaves more room under the title than the settings pages.
    let top = t.m.dp(188.0);

    let mut grid = vec![Spec {
        icon: Outline::Phone,
        label: l.pick(MODEL),
        value: fmt::model_lines(&i.model),
    }];
    if let Some(design) = i.battery.as_ref().and_then(|b| b.design) {
        grid.push(Spec {
            icon: Outline::Battery,
            label: l.pick(BATTERY_CAPACITY),
            value: fmt::mah(design),
        });
    }
    let cpu = match soc(l, i) {
        Some(soc) => format!("{soc}\n{}", fmt::cpu(l, &i.cpu)),
        None => fmt::cpu(l, &i.cpu),
    };
    grid.push(Spec {
        icon: Outline::Chip,
        label: l.pick(PROCESSOR),
        value: cpu,
    });
    grid.push(Spec {
        icon: Outline::Memory,
        label: l.pick(RAM),
        value: fmt::ram(i.ram_kib),
    });
    if let Some(p) = i.panel {
        grid.push(Spec {
            icon: Outline::Resolution,
            label: l.pick(RESOLUTION),
            value: fmt::panel(p),
        });
    }
    if let Some(d) = i.screen {
        grid.push(Spec {
            icon: Outline::Screen,
            label: l.pick(SCREEN_SIZE),
            value: fmt::inches(d),
        });
    }
    grid.push(Spec {
        icon: Outline::Storage,
        label: l.pick(crate::texts::specs::STORAGE_CAPACITY),
        value: fmt::capacity(i.storage.total),
    });
    grid.push(Spec {
        icon: Outline::Tag,
        label: l.pick(crate::texts::specs::CODENAME),
        value: i.codename.clone(),
    });
    let mut y = widgets::spec_grid(&mut c, t, f, top, &grid);

    if let Some(v) = &i.os_version {
        y = widgets::info_card(&mut c, t, f, y + gap, &[(l.pick(OS_VERSION), v.clone())]);
    }
    if let Some(r) = &i.android {
        y = widgets::info_card(
            &mut c,
            t,
            f,
            y + gap,
            &[(l.pick(ANDROID_VERSION), fmt::android(l, r, i.security_patch.as_ref()))],
        );
    }
    y = widgets::info_card(
        &mut c,
        t,
        f,
        y + gap,
        &[
            (
                l.pick(crate::texts::specs::RECOVERY_VERSION),
                format!("Skyward {VERSION}"),
            ),
            (l.pick(crate::texts::specs::KERNEL_VERSION), i.kernel.clone()),
        ],
    );

    let mut boot = Vec::new();
    if let Some(s) = i.slot {
        boot.push((l.pick(CURRENT_SLOT), s.letter().to_string()));
        boot.push((l.pick(UPDATE), fmt::snapshot(l, &i.snapshot).to_string()));
    }
    if let Some(unlocked) = i.boot.unlocked {
        let state = if unlocked {
            crate::texts::specs::UNLOCKED
        } else {
            crate::texts::specs::LOCKED
        };
        boot.push(("Bootloader", l.pick(state).to_string()));
    }
    if let Some(v) = &i.boot.verified {
        boot.push((l.pick(crate::texts::specs::VERIFIED_BOOT), fmt::verified(l, v)));
    }
    boot.push((l.pick(DATA_PART), fmt::data(l, &cx.state.data).to_string()));
    y = widgets::info_card(&mut c, t, f, y + gap, &boot);

    if let Some(b) = &i.battery {
        let mut rows = vec![(l.pick(["电量", "電量", "Level"]), fmt::level(l, b))];
        if let Some(v) = b.temp {
            rows.push((l.pick(crate::texts::specs::TEMPERATURE), fmt::celsius(v)));
        }
        if let Some(v) = fmt::wear(l, b) {
            rows.push((l.pick(crate::texts::specs::HEALTH), v));
        }
        if let Some(v) = b.cycles {
            rows.push((l.pick(crate::texts::specs::CHARGE_CYCLES), tr!(l, "{v} 次", "{v} 次", "{v}")));
        }
        if let Some(v) = b.voltage {
            rows.push((l.pick(crate::texts::specs::VOLTAGE), fmt::volts(v)));
        }
        y = widgets::info_card(&mut c, t, f, y + gap, &rows);
    }

    let total = fmt::capacity(i.storage.total);
    let storage = match i.storage.free.map(fmt::gb) {
        Some(free) => tr!(
            l,
            "可用存储 {free} / 总存储 {total}",
            "可用空間 {free} / 總容量 {total}",
            "{free} free of {total}"
        ),
        None => tr!(
            l,
            "总存储 {total} · data 分区未挂载",
            "總容量 {total} · data 分割區未掛載",
            "{total} in all · data partition not mounted"
        ),
    };
    y = widgets::info_card(
        &mut c,
        t,
        f,
        y + gap,
        &[(l.pick(crate::texts::specs::INTERNAL_STORAGE), storage)],
    );
    let links = widgets::list(
        &mut c,
        t,
        f,
        y + gap,
        &[
            Row::new(l.pick(LICENSE)).trail(Trail::Value("GPL-3.0")),
            Row::new(l.pick(FONTS)).trail(Trail::Value("MiSans")),
        ],
    );
    out.extend(hits(
        &links.rows,
        &[Action::Open(Screen::License), Action::Open(Screen::FontLicense)],
    ));
    Frame::new(t, fit(c, t, links.bottom), out, l.pick(SPECS), top)
}

/// The data partition and its users' own files. The page opens the
/// partition as it shows, which is how it learns each user's screen lock;
/// a user whose files wait on one is asked with the control that lock is
/// answered with, from the start.
pub fn decrypt(cx: &Ctx) -> Frame {
    let (c, l) = (page(cx), cx.state.lang);
    match &cx.state.data {
        Data::Locked { user, keys, checking } => code(cx, c, *user, *keys, *checking),
        Data::Closed | Data::Opening => data_status(
            cx,
            c,
            Icon::Lock,
            l.pick(crate::texts::decrypt::DATA_PARTITION),
            Some(l.pick(crate::texts::decrypt::READING_LOCK)),
            None,
        ),
        Data::Open => open_with_others(cx, c),
        Data::Failed(why) => data_status(
            cx,
            c,
            Icon::Lock,
            l.pick(crate::texts::decrypt::FAILED),
            None,
            Some(why),
        ),
        Data::Erased => data_status(
            cx,
            c,
            Icon::Wipe,
            l.pick(DATA_ERASED),
            Some(l.pick(crate::texts::decrypt::MADE_ANEW)),
            None,
        ),
    }
}

/// The partitions open: the owner's files first, then, where a phone keeps
/// further users whose own files are still closed — a second space's, a
/// cloned app's — each offered by name, since the owner came to open the
/// owner's files and those do not hold the recovery. Tapping one asks for
/// that user's screen lock.
fn open_with_others(cx: &Ctx, mut c: Canvas) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let others = &s.others;
    if others.is_empty() {
        return data_status(
            cx,
            c,
            Icon::Folder,
            l.pick(crate::texts::decrypt::DECRYPTED),
            Some(l.pick(crate::texts::decrypt::SEE_FILES)),
            None,
        );
    }
    let x = m.w as f32 / 2.0;
    let sign_cy = m.dp(150.0);
    c.fill_circle(x, sign_cy, m.dp(30.0), t.pal.card);
    icons::glyph(&mut c, Icon::Folder, x, sign_cy, m.dp(40.0), t.pal.accent);
    c.text_center(
        &f.heading,
        l.pick(crate::texts::decrypt::DECRYPTED),
        x,
        sign_cy + m.dp(64.0),
        m.dp(24.0),
        t.pal.text,
    );
    c.text_center(
        &f.body,
        l.pick(crate::texts::decrypt::SEE_FILES),
        x,
        sign_cy + m.dp(98.0),
        m.sub_px,
        t.pal.subtitle,
    );
    // The users still closed, each a row that asks for that user's lock.
    let note = l.pick(crate::lang::legacy([
        "以下用户的文件尚未解锁，其他用户的数据不受影响：",
        "以下使用者的檔案尚未解鎖，其他使用者的資料不受影響：",
        "These users' files are still closed; the other users' are not affected:",
    ]));
    // The rows borrow their names, so the names are made first and outlive
    // them.
    let names: Vec<String> = others
        .iter()
        .map(|(user, _)| match *user {
            0 => l.pick(crate::lang::legacy(["本机用户", "本機使用者", "This phone's user"])).to_string(),
            user => tr!(l, "用户 {user}", "使用者 {user}", "User {user}"),
        })
        .collect();
    let controls: Vec<&'static str> = others
        .iter()
        .map(|(_, keys)| {
            l.pick(match *keys {
                Keys::Pattern => crate::lang::legacy(["图案", "圖案", "Pattern"]),
                Keys::Digits => crate::lang::legacy(["密码", "密碼", "PIN"]),
                Keys::Letters => crate::texts::wlan::PASSWORD,
            })
        })
        .collect();
    let rows: Vec<Row> = names
        .iter()
        .zip(&controls)
        .map(|(name, control)| Row::new(name).icon(SLATE, Icon::Lock).trail(Trail::Value(control)))
        .collect();
    let top = sign_cy + m.dp(140.0);
    let y = widgets::footnote(&mut c, t, f, top, note);
    let list = widgets::list(&mut c, t, f, y, &rows);
    let mut out: Vec<Hit> = list
        .rows
        .iter()
        .zip(others)
        .map(|(&rect, (user, _))| Hit {
            rect,
            action: Action::UnlockUser(*user),
        })
        .collect();
    let leave_cy = m.h as f32 - m.dp(64.0);
    c.text_center(
        &f.heading,
        l.pick(crate::texts::common::DONE),
        x,
        leave_cy,
        m.row_title_px,
        t.pal.accent,
    );
    out.push(Hit {
        rect: Rect::new(x - m.dp(80.0), leave_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)),
        action: Action::Skip,
    });
    Frame {
        header: false,
        ..Frame::new(t, c, out, l.pick(DECRYPT), 0.0)
    }
}

/// Where the partition stands, under its sign: a line on it, or why it
/// did not open and a way to try again; then the way off the page.
fn data_status(cx: &Ctx, mut c: Canvas, icon: Icon, title: &str, note: Option<&str>, why: Option<&str>) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let x = m.w as f32 / 2.0;
    let sign_cy = m.dp(150.0);
    c.fill_circle(x, sign_cy, m.dp(30.0), t.pal.card);
    icons::glyph(&mut c, icon, x, sign_cy, m.dp(40.0), t.pal.accent);
    c.text_center(&f.heading, title, x, sign_cy + m.dp(64.0), m.dp(24.0), t.pal.text);
    if let Some(note) = note {
        c.text_center(&f.body, note, x, sign_cy + m.dp(98.0), m.sub_px, t.pal.subtitle);
    }
    let mut out = Vec::new();
    if let Some(why) = why {
        widgets::paragraph(&mut c, t, f, sign_cy + m.dp(88.0), why);
        let again = widgets::button(&mut c, t, f, m.h as f32 * 0.62, l.pick(crate::texts::common::TRY_AGAIN));
        out.push(Hit {
            rect: again,
            action: Action::OpenData,
        });
    }
    let leave_cy = m.h as f32 - m.dp(64.0);
    let leave = l.pick(if matches!(cx.state.data, Data::Open | Data::Erased) {
        crate::texts::common::DONE
    } else {
        NOT_NOW
    });
    c.text_center(&f.heading, leave, x, leave_cy, m.row_title_px, t.pal.accent);
    out.push(Hit {
        rect: Rect::new(x - m.dp(80.0), leave_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)),
        action: Action::Skip,
    });
    Frame {
        header: false,
        ..Frame::new(t, c, out, l.pick(DECRYPT), 0.0)
    }
}

/// What leaves the decrypt page with the data partition left as it is.
const NOT_NOW: Text = crate::texts::decrypt::NOT_NOW;

/// What a row whose work waits on the data partition says under it until
/// that is decrypted.
const DECRYPT_NEEDED: Text = crate::texts::decrypt::NEEDED;

/// A user's own files wait on their screen lock: the control it is
/// answered with, which takes nothing while what was entered is checked.
fn code(cx: &Ctx, mut c: Canvas, user: u32, keys: Keys, checking: bool) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let x = m.w as f32 / 2.0;
    let mut out = Vec::new();
    let title = match (keys == Keys::Pattern, user) {
        (true, 0) => l
            .pick(crate::texts::decrypt::DRAW_PATTERN)
            .to_string(),
        (true, user) => tr!(
            l,
            "绘制用户 {user} 的锁屏图案",
            "繪製使用者 {user} 的螢幕鎖定圖案",
            "Draw user {user}'s screen lock pattern"
        ),
        (false, 0) => l.pick(crate::texts::decrypt::ENTER_LOCK).to_string(),
        (false, user) => tr!(
            l,
            "输入用户 {user} 的锁屏密码",
            "輸入使用者 {user} 的螢幕鎖定密碼",
            "Enter user {user}'s screen lock"
        ),
    };
    let note = l.pick(if checking {
        crate::texts::decrypt::CHECKING
    } else {
        crate::texts::decrypt::TO_SEE_FILES
    });
    c.text_center(&f.heading, &title, x, m.dp(130.0), m.dp(24.0), t.pal.text);
    c.text_center(&f.body, note, x, m.dp(166.0), m.sub_px, t.pal.subtitle);
    let mut pattern = None;
    let bottom = match keys {
        Keys::Letters => {
            let top = m.h as f32 - widgets::keyboard_height(t);
            if !checking && cfg!(feature = "pinyin-input") {
                out.extend(crate::ime::bar(&mut c, t, f, &s.ime, top - crate::ime::height(t), false));
            }
            let caps = widgets::keyboard(&mut c, t, f, l, top, s.layer, s.key_held(), l.pick(DECRYPT));
            if !checking {
                out.extend(caps.into_iter().map(|(rect, cap)| Hit {
                    rect,
                    action: Action::Type(cap),
                }));
            }
            top
        }
        Keys::Pattern => {
            let span = m.dp(220.0).min(m.w as f32 - m.dp(96.0));
            let cy = m.dp(405.0).min(m.h as f32 - span / 2.0 - m.dp(120.0));
            widgets::pattern_lock(&mut c, t, x, cy, span, &s.pattern);
            if !checking {
                out.push(Hit {
                    rect: Rect::new(
                        x - span / 2.0 - m.dp(32.0),
                        cy - span / 2.0 - m.dp(32.0),
                        span + m.dp(64.0),
                        span + m.dp(64.0),
                    ),
                    action: Action::Pattern,
                });
                pattern = Some((x, cy, span));
            }
            m.h as f32 - m.dp(26.0)
        }
        Keys::Digits => {
            let top = m.h as f32 - widgets::keypad_height(t);
            let caps = widgets::keypad(&mut c, t, f, l, top, s.key_held());
            if !checking {
                out.extend(caps.into_iter().map(|(rect, cap)| Hit {
                    rect,
                    action: Action::Type(cap),
                }));
            }
            top
        }
    };
    // The dots sit just above whatever types into them; a pattern is its
    // own showing of what has been drawn.
    if keys != Keys::Pattern {
        widgets::code_dots(&mut c, t, x, bottom - m.dp(92.0), s.code.chars().count());
    }
    let skip_cy = bottom - m.dp(34.0);
    c.text_center(&f.heading, l.pick(NOT_NOW), x, skip_cy, m.row_title_px, t.pal.accent);
    out.push(Hit {
        rect: Rect::new(x - m.dp(80.0), skip_cy - m.dp(24.0), m.dp(160.0), m.dp(48.0)),
        action: Action::Skip,
    });
    Frame {
        header: false,
        pattern,
        ..Frame::new(t, c, out, l.pick(DECRYPT), 0.0)
    }
}

/// What a slot is shown as where the boot control could not be read.
const UNREADABLE: Text = crate::texts::common::UNREADABLE;

/// Reboot target picker with a swipe-to-confirm control.
pub fn reboot(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::reboot::TO));
    let rows: Vec<Row> = REBOOT_TARGETS
        .iter()
        .enumerate()
        .map(|(k, &(title, sub))| {
            let r = Row::new(l.pick(title)).trail(Trail::Check(s.reboot == k));
            match sub {
                Some(text) => r.subtitle(l.pick(text)),
                None => r,
            }
        })
        .collect();
    let targets = widgets::list(&mut c, t, f, y, &rows);
    let picks: Vec<Action> = (0..TARGETS.len()).map(Action::RebootTarget).collect();
    out.extend(hits(&targets.rows, &picks));
    y = targets.bottom;
    if let Some(slot) = cx.info.slot {
        let next = match &s.boot_slots {
            Some(Ok(slots)) if slots.active == slot => {
                let slot = slot.letter();
                tr!(l, "{slot}（当前）", "{slot}（目前）", "{slot} (current)")
            }
            Some(Ok(slots)) => slots.active.letter().to_string(),
            Some(Err(_)) => l.pick(UNREADABLE).to_string(),
            None => String::new(),
        };
        y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::slot::TITLE));
        let slots = widgets::list(
            &mut c,
            t,
            f,
            y,
            &[Row::new(l.pick(crate::texts::reboot::NEXT_BOOT)).trail(Trail::Value(&next))],
        );
        out.extend(hits(&slots.rows, &[Action::Open(Screen::Slot)]));
        y = slots.bottom;
    }
    let (label, danger) = confirm_label(Confirm::Reboot, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::Reboot),
    });
    Frame::new(t, c, out, l.pick(REBOOT), top)
}

/// Installing, opened from the home page: the system on the phone now, and
/// where a package can come from.
pub fn install(cx: &Ctx) -> Frame {
    let (t, f, i, l) = (cx.t, cx.f, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut facts = Vec::new();
    if let Some(v) = &i.os_version {
        facts.push((l.pick(crate::texts::install::CURRENT_SYSTEM), v.clone()));
    }
    if let Some(p) = &i.security_patch {
        facts.push((l.pick(SECURITY_PATCH), fmt::patch(l, p)));
    }
    if let Some(s) = i.slot {
        let (slot, other) = (s.letter(), s.other().letter());
        let detail = tr!(
            l,
            "{slot}，系统更新将安装到槽位 {other}",
            "{slot}，系統更新將安裝到槽位 {other}",
            "{slot}, and a system update installs to slot {other}"
        );
        facts.push((l.pick(CURRENT_SLOT), detail));
    }
    let ways = l.pick(crate::texts::install::WAYS);
    let y = if facts.is_empty() {
        widgets::section_first(&mut c, t, f, top, ways)
    } else {
        let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::common::THIS_PHONE));
        let y = widgets::info_card(&mut c, t, f, y, &facts);
        widgets::section(&mut c, t, f, y, ways)
    };
    let mut rows = Vec::new();
    if let Some(note) = fmt::snapshot_note(l, &i.snapshot) {
        rows.push(
            Row::new(fmt::snapshot(l, &i.snapshot))
                .icon(ORANGE, Icon::Layers)
                .subtitle(note)
                .trail(Trail::None),
        );
    }
    // Internal storage opens with the data partition, which the decrypt
    // page opens or tells why it does not.
    let data = &cx.state.data;
    let (storage, from_storage) = if data.storage_open() {
        (
            crate::texts::install::CHOOSE_IN_STORAGE,
            Action::Open(Screen::Choose(Place::Storage)),
        )
    } else if *data == Data::Erased {
        (DATA_ERASED, Action::Open(Screen::Decrypt))
    } else {
        (DECRYPT_NEEDED, Action::Open(Screen::Decrypt))
    };
    // The drive's page waits for one to be plugged in, and says why one
    // that is cannot be read.
    let usb = match &cx.state.usb_drive {
        Some(UsbDrive::Shown) => crate::texts::install::CHOOSE_ON_USB_DRIVE,
        Some(UsbDrive::Unreadable(_)) => USB_DRIVE_UNREADABLE,
        None => USB_DRIVE_WANTED,
    };
    rows.push(
        Row::new(install_title(l, Source::File(Place::Storage)))
            .icon(BLUE, Icon::Phone)
            .subtitle(l.pick(storage)),
    );
    rows.push(
        Row::new(install_title(l, Source::File(Place::UsbDrive)))
            .icon(PURPLE, Icon::UsbDrive)
            .subtitle(l.pick(usb)),
    );
    let sideload = l.pick(crate::texts::install::SIDELOAD);
    rows.push(
        Row::new(install_title(l, Source::Computer))
            .icon(GREEN, Icon::Usb)
            .subtitle(sideload),
    );
    let methods = widgets::list(&mut c, t, f, y, &rows);
    let from_usb = Action::Open(Screen::Choose(Place::UsbDrive));
    let mut out = hits(&methods.rows[methods.rows.len() - 3..], &[from_storage, from_usb, Action::Sideload]);
    let y = widgets::section(&mut c, t, f, methods.bottom, l.pick(crate::texts::common::OPTIONS));
    let check = Row::new(l.pick(crate::texts::install::VERIFY_SIGNATURES)).switch(cx.state.check_signature);
    let signature = widgets::list(&mut c, t, f, y, &[check]);
    out.extend(hits(&signature.rows, &[Action::Toggle(Switch::Signature)]));
    let note = l.pick(crate::lang::legacy([
        "脚本 ZIP 仅接受本机官方签名，不是 SHA-256 校验。",
        "指令碼 ZIP 僅接受本機官方簽章，並非 SHA-256 驗證。",
        "Require this phone's official signature for script ZIPs, not a SHA-256 checksum.",
    ]));
    let mut y = widgets::footnote(&mut c, t, f, signature.bottom, note);
    if cx.state.recovery_partitions {
        let row = Row::new(l.pick(KEEP_RECOVERY)).switch(cx.state.keep_recovery);
        let options = widgets::list(&mut c, t, f, y + t.m.card_gap, &[row]);
        out.extend(hits(&options.rows, &[Action::Toggle(Switch::KeepRecovery)]));
        let note = l.pick(crate::lang::legacy([
            "恢复被覆盖的 recovery，并保留到下一启动槽位。",
            "還原被覆寫的 recovery，並保留至下次啟動的槽位。",
            "Restore overwritten recovery partitions and keep Skyward in the next boot slot.",
        ]));
        y = widgets::footnote(&mut c, t, f, options.bottom, note);
    }
    // The recovery's own act after an install, not the package's: a ROM
    // built without this phone's keys will not start on a bootloader that
    // still checks signatures, and an install script is an arbitrary
    // program that may or may not disable them for itself.
    let avb = Row::new(l.pick(crate::lang::legacy(["刷入后禁用 AVB", "刷入後停用 AVB", "Disable AVB after installation"]))).switch(cx.state.disable_avb);
    let rows = widgets::list(&mut c, t, f, y + t.m.card_gap, &[avb]);
    out.extend(hits(&rows.rows, &[Action::Toggle(Switch::DisableAvb)]));
    let note = l.pick(crate::lang::legacy([
        "关闭目标槽位的启动验证与 dm-verity，重启生效。原 vbmeta 暂存 /tmp，重启即清空。",
        "停用目標槽位的啟動驗證與 dm-verity，重新啟動生效。原 vbmeta 暫存 /tmp，重新啟動即清空。",
        "Disable boot verification and dm-verity in the target slot after reboot. The vbmeta backup in /tmp is lost on reboot.",
    ]));
    let y = widgets::footnote(&mut c, t, f, rows.bottom, note);
    Frame::new(t, fit(c, t, y), out, l.pick(crate::texts::page::INSTALL), top)
}

const SECURITY_PATCH: Text = crate::texts::install::SECURITY_PATCH;
const KEEP_RECOVERY: Text = crate::lang::legacy(["刷入后保留 Skyward", "刷入後保留 Skyward", "Keep Skyward after installation"]);

/// How installing from `source` is named: on the install page, and as the
/// title of the page that installs.
fn install_title(l: Lang, source: Source) -> &'static str {
    l.pick(match source {
        Source::Computer => crate::texts::install::title::FROM_COMPUTER,
        Source::File(Place::Storage) => crate::texts::install::title::FROM_STORAGE,
        Source::File(Place::UsbDrive) => crate::texts::install::title::FROM_USB_DRIVE,
        Source::Opened { .. } => crate::texts::install::title::FROM_FILES,
        Source::Asked { security: false } => crate::texts::install::title::SYSTEM_UPDATE,
        Source::Asked { security: true } => crate::texts::install::title::SECURITY_UPDATE,
        Source::Rescue => crate::texts::install::title::RESCUE,
    })
}

/// What a place is called at the top of its folders.
fn place_name(l: Lang, place: Place) -> &'static str {
    l.pick(match place {
        Place::Storage => crate::texts::wipe::part::MEDIA,
        Place::UsbDrive => crate::texts::place::USB_DRIVE,
    })
}

/// A one-line card on how things stand.
fn standing<'a>(title: &'a str, color: Rgba, icon: Icon, detail: &'a str) -> Row<'a> {
    Row::new(title).icon(color, icon).subtitle(detail).trail(Trail::None)
}

fn share(done: u64, total: u64) -> f32 {
    if total == 0 { 0.0 } else { (done as f64 / total as f64) as f32 }
}

fn percent(share: f32) -> u32 {
    (share * 100.0).floor() as u32
}

/// Where an update stands, and the share of its step done when known.
pub(crate) fn update_stage(l: Lang, stage: Option<&Stage>) -> (String, Option<f32>) {
    match stage {
        Some(Stage::Writing { done, total }) => {
            let s = share(*done, *total);
            let p = percent(s);
            (
                tr!(
                    l,
                    "第 1 步，共 3 步：写入新系统 {p}%",
                    "第 1 步，共 3 步：寫入新系統 {p}%",
                    "Step 1 of 3: writing the new system {p}%"
                ),
                Some(s),
            )
        }
        Some(Stage::Verifying { index, count, partition }) => {
            let n = index + 1;
            let said = tr!(
                l,
                "第 2 步，共 3 步：校验 {partition}（{n}/{count}）",
                "第 2 步，共 3 步：驗證 {partition}（{n}/{count}）",
                "Step 2 of 3: verifying {partition} ({n}/{count})"
            );
            (said, Some(*index as f32 / *count as f32))
        }
        Some(Stage::Finishing) => (
            l.pick(crate::texts::installing::UPDATE_FINISHING)
                .into(),
            None,
        ),
        Some(Stage::Reflashing { done, total }) => (l.pick(REFLASHING).into(), Some(*done as f32 / *total as f32)),
        Some(
            Stage::Flashing { .. }
            | Stage::Script { .. }
            | Stage::Signature { .. }
            | Stage::Allocating { .. }
            | Stage::Root { .. }
            | Stage::Storing,
        )
        | None => (l.pick(GETTING_READY).into(), None),
    }
}

/// Where installing a system beside the installed one stands, and the share
/// of its step done when known.
pub(crate) fn system_stage(l: Lang, stage: Option<&Stage>) -> (String, Option<f32>) {
    match stage {
        Some(Stage::Allocating { done, total }) => {
            let s = share(*done, *total);
            let p = percent(s);
            (
                tr!(
                    l,
                    "第 1 步，共 3 步：在 data 分区中分配空间 {p}%",
                    "第 1 步，共 3 步：在 data 分割區中分配空間 {p}%",
                    "Step 1 of 3: setting room aside in the data partition {p}%"
                ),
                Some(s),
            )
        }
        Some(Stage::Root { done, total }) => {
            let s = share(*done, *total);
            let p = percent(s);
            let said = match done * 2 <= *total {
                true => tr!(
                    l,
                    "第 2 步，共 3 步：写入系统 {p}%",
                    "第 2 步，共 3 步：寫入系統 {p}%",
                    "Step 2 of 3: writing the system {p}%"
                ),
                false => tr!(
                    l,
                    "第 2 步，共 3 步：校验系统 {p}%",
                    "第 2 步，共 3 步：驗證系統 {p}%",
                    "Step 2 of 3: verifying the system {p}%"
                ),
            };
            (said, Some(s))
        }
        Some(Stage::Storing) => (
            l.pick(crate::texts::installing::SYSTEM_STORING)
            .into(),
            None,
        ),
        _ => (l.pick(GETTING_READY).into(), None),
    }
}

const REFLASHING: Text = crate::texts::installing::REFLASHING;

fn script_warnings(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    use skyward_install::warnings::Warning;
    if cx.state.script_warnings.is_empty() {
        return y;
    }
    let l = cx.state.lang;
    let kinds: Vec<_> = cx
        .state
        .script_warnings
        .iter()
        .map(|warning| {
            l.pick(match warning {
                Warning::MissingCommand => crate::lang::legacy(["命令缺失", "命令缺少", "Missing command"]),
                Warning::MissingFile => crate::lang::legacy(["文件缺失", "檔案缺少", "Missing file"]),
                Warning::NoSpace => crate::lang::legacy(["写入达到容量边界", "寫入達容量邊界", "Write reached capacity"]),
                Warning::Permission => crate::lang::legacy(["权限或只读限制", "權限或唯讀限制", "Permission or read-only restriction"]),
                Warning::Io => crate::lang::legacy(["读写错误", "讀寫錯誤", "I/O error"]),
                Warning::Unpack => crate::lang::legacy(["解压错误", "解壓縮錯誤", "Unpack error"]),
            })
        })
        .collect();
    let details = kinds.join(" / ");
    let note = tr!(
        l,
        "脚本输出含警告：{details}。未跳过命令或更改返回值，请检查安装日志。",
        "指令碼輸出含警告：{details}。未略過命令或變更回傳值，請檢查安裝記錄。",
        "Script output contains warnings: {details}. No command was skipped or return value changed. Review the install log."
    );
    widgets::footnote(c, cx.t, cx.f, y, &note)
}

/// What the card says while an install script runs, and how far it is: the
/// last line the script showed and how far it says it is, then Skyward
/// going back into the recovery partitions it wrote over.
pub(crate) fn script_stage<'a>(l: Lang, stage: Option<&Stage>, said: &'a [String]) -> (&'a str, Option<f32>) {
    let running = l.pick(crate::texts::installing::RUNNING_SCRIPT);
    let last = said
        .iter()
        .rev()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .unwrap_or(running);
    match stage {
        Some(Stage::Reflashing { done, total }) => (l.pick(REFLASHING), Some(*done as f32 / *total as f32)),
        Some(Stage::Script { thousandths }) => (last, Some(f32::from(*thousandths) / 1000.0)),
        _ => (last, None),
    }
}

/// How Skyward went back into the recovery partitions the install wrote
/// over, where it did.
fn reflashed(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    match &cx.state.reflash {
        None => y,
        Some(Reflash::Written(written)) => {
            let slots = fmt::slots(l, written);
            let said = match (l, written.len()) {
                (Lang::En, 1) => format!("Skyward went back into {slots}'s recovery partition."),
                _ => tr!(
                    l,
                    "已将 Skyward 重新刷入{slots} 的 recovery 分区。",
                    "已將 Skyward 重新刷入{slots} 的 recovery 分割區。",
                    "Skyward went back into the recovery partitions of {slots}."
                ),
            };
            widgets::footnote(c, t, f, y, &said)
        }
        Some(Reflash::Failed(why)) => {
            let note = l.pick(crate::texts::installing::REFLASH_FAILED);
            let y = widgets::footnote(c, t, f, y, note);
            widgets::footnote(c, t, f, y, why)
        }
    }
}

/// What the install script showed, where there is any: its heading, and
/// under it the lines as the page's long list, the last thing on the page.
fn script_output(c: &mut Canvas, cx: &Ctx, y: f32, rows: &mut Option<Rows>) -> f32 {
    let lines = &cx.state.script_output;
    if lines.is_empty() {
        return y;
    }
    let y = widgets::section(
        c,
        cx.t,
        cx.f,
        y,
        cx.state.lang.pick(crate::texts::installing::SCRIPT_OUTPUT),
    );
    *rows = Some(Rows {
        edges: widgets::output_edges(c, cx.t, cx.f, y, lines),
        action: None,
    });
    y
}

fn pick_title(l: Lang, pick: Pick, current: Option<Slot>) -> String {
    let letter = |s: Option<Slot>| s.map(|s| format!(" {}", s.letter())).unwrap_or_default();
    match pick {
        Pick::Current => format!("{}{}", l.pick(CURRENT_SLOT), letter(current)),
        Pick::Other => format!(
            "{}{}",
            l.pick(["另一个槽位", "另一個槽位", "Other slot"]),
            letter(current.map(Slot::other))
        ),
        Pick::Both => l.pick(crate::texts::flash::BOTH_SLOTS).into(),
    }
}

/// A raw super image: the one partition `super`, and no slots of its own. The
/// installer writes the whole partition and ignores the slot picked, so the
/// page does not offer one.
fn super_image(partitions: &[&'static str]) -> bool {
    matches!(partitions, ["super"])
}

/// Words the installing page says of more than one of its steps.
const SWITCHING_USB: Text = crate::texts::installing::SWITCHING_USB;
const READING_FILE: Text = crate::texts::installing::READING_FILE;
const WAITING: Text = crate::texts::installing::WAITING;
const CONNECT: Text = crate::texts::installing::CONNECT;
const RUN_ON_COMPUTER: Text = crate::texts::installing::RUN_ON_COMPUTER;
const KEEP_CABLE: Text = crate::texts::installing::KEEP_CABLE;
const KEEP_DRIVE: Text = crate::texts::installing::KEEP_USB_DRIVE;
const GETTING_READY: Text = crate::texts::installing::GETTING_READY;
/// What flashing a raw super image does, which is not like the other
/// partitions: the whole of it is written, and what is inside it is mapped
/// only at the next start.
const SUPER_NOTE: Text = crate::lang::legacy([
    "将写入整个 super 分区，其中的逻辑分区要重启手机后才会重新映射。刷入完成后请重启手机。",
    "將寫入整個 super 分割區，其中的邏輯分割區要重新啟動手機後才會重新對應。刷入完成後請重新啟動手機。",
    "The whole super partition is written, and the logical partitions inside it are mapped anew only when the phone next starts. Restart the phone once it's flashed.",
]);

/// Installing a package step by step: receiving what the computer sends
/// with `adb sideload`, or reading the file chosen, then installing it.
pub fn installing(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let (mut out, mut rows) = (Vec::new(), None);
    let top = t.m.content_top;
    let computer = s.source == Source::Computer;
    let bottom = match &s.install {
        Install::Idle | Install::Starting | Install::Ending if computer => {
            let row = standing(
                l.pick(SWITCHING_USB),
                GREEN,
                Icon::Usb,
                l.pick(crate::texts::installing::TO_SIDELOAD),
            );
            widgets::list(&mut c, t, f, top, &[row]).bottom
        }
        Install::Idle | Install::Starting | Install::Ending if s.source == Source::Rescue => {
            let row = standing(
                l.pick(SWITCHING_USB),
                GREEN,
                Icon::Usb,
                l.pick(crate::texts::installing::TO_RESCUE),
            );
            widgets::list(&mut c, t, f, top, &[row]).bottom
        }
        Install::Idle | Install::Starting | Install::Ending => {
            widgets::list(
                &mut c,
                t,
                f,
                top,
                &[Row::new(l.pick(READING_FILE)).icon(GREEN, Icon::Install).trail(Trail::None)],
            )
            .bottom
        }
        Install::Opening => {
            let opening = Row::new(l.pick(crate::texts::installing::OPENING_DATA))
                .icon(GREEN, Icon::Lock)
                .trail(Trail::None);
            let y = widgets::list(&mut c, t, f, top, &[opening]).bottom;
            asked_package(&mut c, cx, y)
        }
        Install::Locked(_) => {
            let row = standing(
                l.pick(crate::texts::installing::CANT_READ_YET),
                ORANGE,
                Icon::Lock,
                l.pick(DECRYPT_NEEDED),
            );
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let open = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(DECRYPT))]);
            out.extend(hits(&open.rows, &[Action::Open(Screen::Decrypt)]));
            let y = widgets::footnote(
                &mut c,
                t,
                f,
                open.bottom,
                l.pick(crate::texts::installing::INSTALLS_ONCE_DECRYPTED),
            );
            asked_package(&mut c, cx, y)
        }
        Install::Waiting if s.source == Source::Rescue => {
            let row = standing(l.pick(WAITING), GREEN, Icon::Usb, l.pick(CONNECT));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(RUN_ON_COMPUTER));
            let y = widgets::info_card(
                &mut c,
                t,
                f,
                y,
                &[
                    (
                        l.pick(crate::texts::installing::RESCUE_INSTALL),
                        l.pick(crate::texts::installing::RESCUE_INSTALL_NOTE)
                        .into(),
                    ),
                    (
                        "adb rescue getprop",
                        l.pick(crate::texts::installing::RESCUE_GETPROP_NOTE)
                        .into(),
                    ),
                    (
                        "adb rescue wipe userdata",
                        l.pick(crate::texts::installing::RESCUE_WIPE_NOTE)
                            .into(),
                    ),
                ],
            );
            let note = l.pick(crate::texts::installing::RESCUE_NOTE);
            widgets::footnote(&mut c, t, f, y, note)
        }
        Install::Waiting => {
            let row = standing(l.pick(WAITING), GREEN, Icon::Usb, l.pick(CONNECT));
            let y = widgets::list(&mut c, t, f, top, &[row]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(RUN_ON_COMPUTER));
            let y = widgets::info_card(
                &mut c,
                t,
                f,
                y,
                &[(
                    l.pick(crate::texts::installing::SIDELOAD),
                    l.pick(crate::texts::installing::SIDELOAD_FILE_NOTE)
                    .into(),
                )],
            );
            let note = l.pick(crate::texts::installing::SIDELOAD_NOTE);
            widgets::footnote(&mut c, t, f, y, note)
        }
        Install::Checking {
            stage: Some(Stage::Signature { done, total }),
            ..
        } => {
            let read = share(*done, *total);
            let p = percent(read);
            let detail = tr!(l, "已读取 {p}%", "已讀取 {p}%", "{p}% read");
            let y = widgets::progress_card(
                &mut c,
                t,
                f,
                top,
                l.pick(crate::texts::installing::VERIFYING_SIGNATURE),
                &detail,
                Some(read),
            );
            let note = l.pick(match s.source {
                Source::Computer => crate::texts::installing::CANCEL_ENDS_SIDELOAD,
                Source::Rescue => crate::texts::installing::CANCEL_LEAVES_RESCUE,
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => {
                    crate::texts::installing::CANCEL
                }
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => crate::texts::installing::CANCEL_KEEP_USB_DRIVE,
            });
            widgets::footnote(&mut c, t, f, y, note)
        }
        Install::Checking { size, .. } => {
            let size = fmt::bytes(*size);
            let detail = tr!(l, "文件大小 {size}", "檔案大小 {size}", "File size {size}");
            let y = widgets::list(&mut c, t, f, top, &[standing(l.pick(READING_FILE), GREEN, Icon::Install, &detail)]).bottom;
            match s.source {
                Source::Computer | Source::Rescue => widgets::footnote(&mut c, t, f, y, l.pick(KEEP_CABLE)),
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => y,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => widgets::footnote(&mut c, t, f, y, l.pick(KEEP_DRIVE)),
            }
        }
        Install::Choosing(choice) => {
            let (name, holds) = fmt::image_kind(l, choice.kind);
            let size = fmt::bytes(choice.size);
            let detail = tr!(l, "{size}，{holds}", "{size}，{holds}", "{size} · {holds}");
            let y = widgets::list(&mut c, t, f, top, &[standing(name, GREEN, Icon::Install, &detail)]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::image::role::PARTITION));
            let rows: Vec<Row> = choice
                .partitions
                .iter()
                .enumerate()
                .map(|(k, p)| {
                    Row::new(p)
                        .subtitle(fmt::partition_role(l, p))
                        .trail(Trail::Check(k == choice.partition))
                })
                .collect();
            let parts = widgets::list(&mut c, t, f, y, &rows);
            let choose: Vec<Action> = (0..choice.partitions.len()).map(Action::FlashPartition).collect();
            out.extend(hits(&parts.rows, &choose));
            // A raw super image has no slots of its own: the installer writes
            // the whole partition and ignores the slot picked, so the page
            // does not ask for one, and says what that means below the swipe.
            let super_only = super_image(&choice.partitions);
            let bottom = if super_only {
                parts.bottom
            } else {
                let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(crate::texts::slot::TITLE));
                let titles = Pick::ALL.map(|p| pick_title(l, p, i.slot));
                let rows: Vec<Row> = Pick::ALL
                    .iter()
                    .zip(&titles)
                    .map(|(p, title)| Row::new(title).trail(Trail::Check(*p == choice.pick)))
                    .collect();
                let picks = widgets::list(&mut c, t, f, y, &rows);
                out.extend(hits(&picks.rows, &Pick::ALL.map(Action::FlashSlots)));
                picks.bottom
            };
            let (label, danger) = confirm_label(Confirm::Flash, cx.state);
            let track = widgets::confirm(&mut c, t, f, bottom + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(Confirm::Flash),
            });
            let y = if super_only {
                widgets::footnote(&mut c, t, f, track.y + track.h, l.pick(SUPER_NOTE))
            } else {
                track.y + track.h
            };
            let note = l.pick(match s.source {
                Source::Computer => crate::texts::flash::OVERWRITES_ENDS_SIDELOAD,
                Source::Rescue => crate::texts::flash::OVERWRITES_LEAVES_RESCUE,
                Source::File(_) | Source::Opened { .. } | Source::Asked { .. } => crate::texts::flash::OVERWRITES,
            });
            widgets::footnote(&mut c, t, f, y, note)
        }
        Install::Confirming { version, wipe } => {
            let (title, color, icon, detail, which) = if *wipe {
                let reset = factory_reset(s);
                let detail = match version {
                    Some(v) => tr!(
                        l,
                        "安装 {v} 后将{reset}。",
                        "安裝 {v} 後將{reset}。",
                        "Installing {v} then {reset}."
                    ),
                    None => tr!(
                        l,
                        "安装该更新后将{reset}。",
                        "安裝這個更新後將{reset}。",
                        "Installing this update then {reset}."
                    ),
                };
                (
                    l.pick(crate::texts::installing::confirm::WIPE_NEEDED).to_string(),
                    ORANGE,
                    Icon::Wipe,
                    detail,
                    Confirm::InstallWipe,
                )
            } else {
                let title = match version {
                    Some(v) => tr!(l, "安装 {v}", "安裝 {v}", "Install {v}"),
                    None => l.pick(crate::texts::installing::confirm::UPDATE).into(),
                };
                let detail = match i.slot.map(|s| s.other().letter()) {
                    Some(slot) => tr!(
                        l,
                        "新系统将安装到槽位 {slot}，重启后将从该槽位启动。",
                        "新系統將安裝到槽位 {slot}，重新啟動後將從該槽位啟動。",
                        "The new system installs to slot {slot}, which starts after a reboot."
                    ),
                    None => l
                        .pick(crate::texts::installing::confirm::STARTS_AFTER_REBOOT)
                        .into(),
                };
                (title, GREEN, Icon::Install, detail, Confirm::Install)
            };
            let y = widgets::list(&mut c, t, f, top, &[standing(&title, color, icon, &detail)]).bottom;
            let (label, danger) = confirm_label(which, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(which),
            });
            let note = l.pick(crate::texts::installing::CANCEL_WRITES_NOTHING);
            widgets::footnote(&mut c, t, f, track.y + track.h, note)
        }
        Install::Updating {
            version,
            stage,
            overwriting,
        } => {
            let title = match version {
                Some(v) => tr!(l, "正在安装 {v}", "正在安裝 {v}", "Installing {v}"),
                None => l
                    .pick(crate::texts::installing::UPDATE)
                    .into(),
            };
            let (detail, done) = update_stage(l, stage.as_ref());
            let y = widgets::progress_card(&mut c, t, f, top, &title, &detail, done);
            let until = until_installed(l, s.source);
            let note = match i.slot.map(|s| s.other().letter()) {
                Some(slot) => tr!(
                    l,
                    "{until}新系统将安装到槽位 {slot}。",
                    "{until}新系統將安裝到槽位 {slot}。",
                    "{until} The new system installs to slot {slot}."
                ),
                None => until.to_string(),
            };
            let y = widgets::footnote(&mut c, t, f, y, &note);
            if *overwriting {
                let note = l.pick(crate::texts::installing::OVERWRITING);
                widgets::footnote(&mut c, t, f, y, note)
            } else {
                y
            }
        }
        Install::Flashing { partition, stage } => {
            let (detail, done) = match stage {
                Some(Stage::Flashing { done, total }) => {
                    let s = share(*done, *total);
                    let p = percent(s);
                    let detail = match done * 2 < *total {
                        true => tr!(l, "正在写入 {p}%", "正在寫入 {p}%", "Writing {p}%"),
                        false => tr!(l, "正在校验 {p}%", "正在驗證 {p}%", "Verifying {p}%"),
                    };
                    (detail, Some(s))
                }
                _ => (l.pick(GETTING_READY).into(), None),
            };
            let title = tr!(l, "正在刷入 {partition}", "正在刷入 {partition}", "Flashing {partition}");
            let y = widgets::progress_card(&mut c, t, f, top, &title, &detail, done);
            let keep = l.pick(match s.source {
                Source::Computer | Source::Rescue => crate::texts::flash::KEEP_CABLE,
                Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => crate::texts::flash::KEEP_POWER,
                Source::File(Place::UsbDrive) | Source::Opened { drive: true } => crate::texts::flash::KEEP_USB_DRIVE,
            });
            widgets::footnote(&mut c, t, f, y, keep)
        }
        Install::SystemChoosing(choice) => {
            let version = &choice.version;
            let detail = tr!(
                l,
                "{version}，将安装为第二系统",
                "{version}，將安裝為第二系統",
                "{version}, to install as the second system"
            );
            let y = widgets::list(&mut c, t, f, top, &[standing(&choice.name, PURPLE, Icon::Systems, &detail)]).bottom;
            let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::installing::ROOM));
            let sizes: Vec<String> = choice.sizes.iter().map(|&b| fmt::bytes(b)).collect();
            let least = l.pick(crate::texts::installing::LEAST_ROOM);
            let rows: Vec<Row> = sizes
                .iter()
                .enumerate()
                .map(|(k, size)| {
                    let row = Row::new(size).trail(Trail::Check(k == choice.size));
                    if k == 0 { row.subtitle(least) } else { row }
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            let picks: Vec<Action> = (0..choice.sizes.len()).map(Action::SystemSize).collect();
            out.extend(hits(&list.rows, &picks));
            let note = l.pick(crate::texts::installing::ROOM_NOTE);
            let y = widgets::footnote(&mut c, t, f, list.bottom, note);
            let (label, danger) = confirm_label(Confirm::InstallSystem, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(Confirm::InstallSystem),
            });
            let note = l.pick(crate::texts::installing::CANCEL_WRITES_NOTHING);
            widgets::footnote(&mut c, t, f, track.y + track.h, note)
        }
        Install::Systeming { name, stage } => {
            let title = tr!(l, "正在安装 {name}", "正在安裝 {name}", "Installing {name}");
            let (detail, done) = system_stage(l, stage.as_ref());
            let y = widgets::progress_card(&mut c, t, f, top, &title, &detail, done);
            widgets::footnote(&mut c, t, f, y, until_installed(l, s.source))
        }
        Install::ConfirmingScript => {
            let name = s.package_name.as_deref().unwrap_or(l.pick(crate::texts::installing::FLASHABLE_ZIP));
            let detail = l.pick(crate::texts::installing::SCRIPT_WARNING);
            let mut y = widgets::list(&mut c, t, f, top, &[standing(name, ORANGE, Icon::Install, detail)]).bottom;
            if let Some(note) = fmt::snapshot_script_note(l, &i.snapshot) {
                y = widgets::footnote(&mut c, t, f, y, note);
            }
            let (label, danger) = confirm_label(Confirm::RunScript, cx.state);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(Confirm::RunScript),
            });
            let note = l.pick(crate::texts::installing::CANCEL_RUNS_NOTHING);
            widgets::footnote(&mut c, t, f, track.y + track.h, note)
        }
        Install::Scripting { stage, .. } => {
            let (detail, done) = script_stage(l, stage.as_ref(), &s.script_output);
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(crate::texts::installing::SCRIPT), detail, done);
            let y = widgets::footnote(&mut c, t, f, y, until_installed(l, s.source));
            let y = script_warnings(&mut c, cx, y);
            script_output(&mut c, cx, y, &mut rows)
        }
        Install::Wiping => {
            let y = widgets::progress_card(&mut c, t, f, top, l.pick(WIPING), &capitalized(l, &factory_reset(s)), None);
            widgets::footnote(&mut c, t, f, y, l.pick(KEEP_POWER))
        }
        Install::Wiped(Ok(())) => {
            let made_anew = l.pick(crate::texts::installing::DATA_MADE_ANEW);
            let y = widgets::list(
                &mut c,
                t,
                f,
                top,
                &[standing(l.pick(crate::texts::wipe::WIPED), GREEN, Icon::Wipe, made_anew)],
            )
            .bottom;
            widgets::footnote(&mut c, t, f, y, l.pick(MORE_COMMANDS))
        }
        Install::Wiped(Err(stop)) => {
            let (what, detail) = fmt::erase_stop(l, stop);
            let mut y = widgets::list(&mut c, t, f, top, &[standing(l.pick(WIPE_INCOMPLETE), RED, Icon::Wipe, &what)]).bottom;
            if let Some(d) = detail {
                y = widgets::footnote(&mut c, t, f, y, &d);
            }
            widgets::footnote(&mut c, t, f, y, l.pick(MORE_COMMANDS))
        }
        Install::Finished { outcome, damage } => finished(&mut c, cx, top, outcome, *damage, &mut out, &mut rows),
    };
    // What the system asked for says why, where it said.
    let bottom = match (s.source, &s.asked.reason) {
        (Source::Asked { .. }, Some(reason)) => {
            widgets::info_card(&mut c, t, f, bottom + t.m.dp(12.0), &[(l.pick(REASON), reason.clone())])
        }
        _ => bottom,
    };
    Frame {
        rows,
        ..Frame::new(t, fit(c, t, bottom), out, install_title(l, s.source), top)
    }
}

/// What a request the system made gives as why.
const REASON: Text = crate::texts::installing::REASON;

const WIPING: Text = crate::texts::wipe::WIPING;
const WIPE_INCOMPLETE: Text = crate::texts::wipe::INCOMPLETE;
const KEEP_POWER: Text = crate::texts::wipe::KEEP_POWER;

/// What rescue says once a command is done with: the computer may send
/// another.
const MORE_COMMANDS: Text = crate::texts::installing::MORE_COMMANDS;

/// The package the system asked for, by the path it gave, at `y`.
fn asked_package(c: &mut Canvas, cx: &Ctx, y: f32) -> f32 {
    let name = cx.state.package_name.clone().unwrap_or_default();
    widgets::info_card(
        c,
        cx.t,
        cx.f,
        y + cx.t.m.dp(12.0),
        &[(cx.state.lang.pick(crate::texts::installing::PACKAGE), name)],
    )
}

/// What not to do while a package from `source` is installed.
fn until_installed(l: Lang, source: Source) -> &'static str {
    l.pick(match source {
        Source::Computer | Source::Rescue => crate::texts::installing::UNTIL_KEEP_CABLE,
        Source::File(Place::Storage) | Source::Opened { drive: false } | Source::Asked { .. } => crate::texts::installing::UNTIL_KEEP_POWER,
        Source::File(Place::UsbDrive) | Source::Opened { drive: true } => crate::texts::installing::UNTIL_KEEP_USB_DRIVE,
    })
}

/// A phrase that opens a line on its own: in English with its first letter
/// a capital.
fn capitalized(l: Lang, phrase: &str) -> String {
    let mut chars = phrase.chars();
    match (l, chars.next()) {
        (Lang::En, Some(first)) => first.to_uppercase().chain(chars).collect(),
        _ => phrase.to_string(),
    }
}

/// What a flash that ended says. A raw super image is written whole and has no
/// slots, which the installer reports as none: said without one, rather than
/// with a slot list that is not there.
fn flashed_detail(l: Lang, partition: &str, slots: &[Slot]) -> String {
    if slots.is_empty() {
        return tr!(l, "{partition} 已刷入。", "{partition} 已刷入。", "Flashed to {partition}.");
    }
    let slots = fmt::slots(l, slots);
    tr!(
        l,
        "{partition} 已刷入{slots}。",
        "{partition} 已刷入{slots}。",
        "Flashed to {partition} in {slots}."
    )
}

/// How an install ended, and where to go from there: another package from
/// the computer, or back to choose another file.
fn finished(
    c: &mut Canvas,
    cx: &Ctx,
    top: f32,
    outcome: &Outcome,
    damage: Option<Damage>,
    out: &mut Vec<Hit>,
    rows: &mut Option<Rows>,
) -> f32 {
    let (t, f, source, l) = (cx.t, cx.f, cx.state.source, cx.state.lang);
    let gap = t.m.card_gap;
    let top = if cx.state.system_detached {
        let notice = widgets::list(
            c,
            t,
            f,
            top,
            &[Row::new(l.pick(crate::lang::legacy(["请重启 recovery", "請重新啟動 recovery", "Reboot recovery"])))
                .icon(ORANGE, Icon::Power)
                .subtitle(l.pick(RESTART_AFTER_INSTALL))],
        );
        out.extend(hits(&notice.rows, &[Action::RebootTo(Target::Recovery)]));
        notice.bottom + gap
    } else {
        top
    };
    let choose_another = crate::texts::installed::CHOOSE_ANOTHER;
    let (again, another) = match source {
        Source::Computer => (
            Row::new(l.pick(crate::texts::installed::SEND_AGAIN)).icon(GREEN, Icon::Usb),
            Action::Sideload,
        ),
        Source::File(_) | Source::Opened { .. } => (Row::new(l.pick(choose_another)).icon(GREEN, Icon::Folder), Action::Back),
        Source::Asked { .. } => (
            Row::new(l.pick(choose_another)).icon(GREEN, Icon::Folder),
            Action::Open(Screen::Install),
        ),
        Source::Rescue => (
            Row::new(l.pick(crate::texts::installed::LEAVE_RESCUE)).icon(GREEN, Icon::Usb),
            Action::Back,
        ),
    };
    let restart = Row::new(l.pick(crate::texts::installed::REBOOT_TO_SYSTEM)).icon(ORANGE, Icon::Power);
    let installed = l.pick(crate::texts::installed::INSTALLED);
    let nothing_written = l.pick(crate::texts::installed::NOTHING_WRITTEN);
    match outcome {
        Outcome::Installed { slot, .. } => {
            let slot = slot.letter();
            let detail = tr!(
                l,
                "新系统已安装到槽位 {slot}，重启后将从该槽位启动。",
                "新系統已安裝到槽位 {slot}，重新啟動後將從該槽位啟動。",
                "The new system is installed to slot {slot}, which starts after a reboot."
            );
            let y = widgets::list(c, t, f, top, &[standing(installed, GREEN, Icon::Install, &detail)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[restart]);
            out.extend(hits(&next.rows, &[Action::RebootTo(Target::System)]));
            let y = reflashed(c, cx, next.bottom);
            afterwards_of(c, cx, y, true)
        }
        Outcome::Flashed { partition, slots } => {
            let detail = flashed_detail(l, partition, slots);
            let y = widgets::list(
                c,
                t,
                f,
                top,
                &[standing(l.pick(crate::texts::installed::FLASHED), GREEN, Icon::Install, &detail)],
            )
            .bottom;
            let mut rows = vec![restart];
            let mut acts = vec![Action::RebootTo(Target::System)];
            if *partition == "recovery" {
                rows.push(Row::new(l.pick(crate::texts::installed::REBOOT_TO_RECOVERY)).icon(ORANGE, Icon::Power));
                acts.push(Action::RebootTo(Target::Recovery));
            }
            rows.push(again);
            acts.push(another);
            let next = widgets::list(c, t, f, y + gap, &rows);
            out.extend(hits(&next.rows, &acts));
            afterwards_of(c, cx, next.bottom, true)
        }
        Outcome::Ran { .. } => {
            let ran = l.pick(crate::texts::installed::SCRIPT_RAN);
            let warned = !cx.state.script_warnings.is_empty();
            let title = if warned {
                l.pick(crate::lang::legacy(["脚本完成，存在警告", "指令碼完成，有警告", "Script completed with warnings"]))
            } else {
                installed
            };
            let y = widgets::list(
                c,
                t,
                f,
                top,
                &[standing(title, if warned { ORANGE } else { GREEN }, Icon::Install, ran)],
            )
            .bottom;
            let y = script_warnings(c, cx, y);
            let next = widgets::list(c, t, f, y + gap, &[restart, again]);
            out.extend(hits(&next.rows, &[Action::RebootTo(Target::System), another]));
            let y = reflashed(c, cx, next.bottom);
            let y = afterwards_of(c, cx, y, true);
            script_output(c, cx, y, rows)
        }
        Outcome::SystemInstalled { name } => {
            let detail = tr!(
                l,
                "{name} 已安装为第二系统，可在「双系统」中切换。",
                "{name} 已安裝為第二系統，可在「雙系統」中切換。",
                "{name} is installed as the second system. Switch to it from Dual boot."
            );
            let y = widgets::list(c, t, f, top, &[standing(installed, GREEN, Icon::Systems, &detail)]).bottom;
            let next = widgets::list(c, t, f, y + gap, &[Row::new(l.pick(DUAL_BOOT)).icon(PURPLE, Icon::Systems)]);
            out.extend(hits(&next.rows, &[Action::Open(Screen::Systems)]));
            afterwards_of(c, cx, next.bottom, true)
        }
        Outcome::Cancelled => {
            let y = widgets::list(
                c,
                t,
                f,
                top,
                &[standing(
                    l.pick(crate::texts::installed::CANCELLED),
                    SLATE,
                    Icon::Install,
                    nothing_written,
                )],
            )
            .bottom;
            let next = widgets::list(c, t, f, y + gap, &[again]);
            out.extend(hits(&next.rows, &[another]));
            next.bottom
        }
        Outcome::Refused(r) => {
            let reason = fmt::refusal(l, r, &cx.info.codename, source, cx.state.zone());
            let y = widgets::list(
                c,
                t,
                f,
                top,
                &[standing(
                    l.pick(crate::texts::installed::CANT_INSTALL),
                    ORANGE,
                    Icon::Install,
                    &reason,
                )],
            )
            .bottom;
            // What the system asked for and did not go in leaves the phone
            // where it was, from which it may go back to the system.
            let (rows, acts) = match source {
                Source::Asked { .. } => (vec![again, restart], vec![another, Action::RebootTo(Target::System)]),
                _ => (vec![again], vec![another]),
            };
            let next = widgets::list(c, t, f, y + gap, &rows);
            out.extend(hits(&next.rows, &acts));
            let y = widgets::footnote(c, t, f, next.bottom, nothing_written);
            afterwards_of(c, cx, y, false)
        }
        Outcome::Failed(failure) => {
            let (what, detail) = fmt::failure(l, failure, source);
            let y = widgets::list(
                c,
                t,
                f,
                top,
                &[standing(
                    l.pick(crate::texts::installed::FAILED),
                    RED,
                    Icon::Install,
                    &what,
                )],
            )
            .bottom;
            let next = widgets::list(c, t, f, y + gap, &[again, restart]);
            out.extend(hits(&next.rows, &[another, Action::RebootTo(Target::System)]));
            // The package is sent again, or installed or flashed again.
            let (install, flash) = match source {
                Source::Computer | Source::Rescue => (crate::lang::legacy(["重新发送", "重新傳送", "Send"]), crate::lang::legacy(["重新发送", "重新傳送", "Send"])),
                Source::File(_) | Source::Opened { .. } | Source::Asked { .. } => {
                    (crate::lang::legacy(["重新安装", "重新安裝", "Install"]), crate::lang::legacy(["重新刷入", "重新刷入", "Flash"]))
                }
            };
            let (install, flash) = (l.pick(install), l.pick(flash));
            let warning = match damage {
                Some(Damage::System) => tr!(
                    l,
                    "当前系统已被部分覆盖，重启后将无法进入系统。请{install}完整更新包，安装完成后再重启。",
                    "目前的系統已被部分覆寫，重新啟動後將無法進入系統。請{install}完整更新檔，安裝完成後再重新啟動。",
                    "The current system was partly written over, so a reboot starts no system. {install} the full OTA package again, and reboot once it's installed."
                ),
                Some(Damage::Partition(p)) => tr!(
                    l,
                    "{p} 仅写入了部分内容，重启后可能无法正常启动。请{flash}该镜像，刷入完成后再重启。",
                    "{p} 僅寫入了部分內容，重新啟動後可能無法正常啟動。請{flash}這個映像檔，刷入完成後再重新啟動。",
                    "{p} was only partly written, so the phone may not start properly after a reboot. {flash} the image again, and reboot once it's flashed."
                ),
                Some(Damage::Script) => tr!(
                    l,
                    "本机的分区可能已被部分改写，重启后可能无法正常启动。请{install}该刷机包，安装完成后再重启。",
                    "本機的分割區可能已被部分改寫，重新啟動後可能無法正常啟動。請{install}這個刷機包，安裝完成後再重新啟動。",
                    "This phone's partitions may have been partly rewritten, so it may not start properly after a reboot. {install} the zip again, and reboot once it's installed."
                ),
                None => l
                    .pick(crate::texts::installed::damage::NONE)
                    .into(),
            };
            let mut y = widgets::footnote(c, t, f, next.bottom, &warning);
            if let Some(d) = detail {
                y = widgets::footnote(c, t, f, y, &d);
            }
            let y = reflashed(c, cx, y);
            let y = afterwards_of(c, cx, y, false);
            script_output(c, cx, y, rows)
        }
    }
}

/// What comes after an install, where it is not the user's to choose:
/// rescue waits for the computer's next command, and a package the system
/// asked for, once in, has the phone leave on its own.
fn afterwards_of(c: &mut Canvas, cx: &Ctx, y: f32, installed: bool) -> f32 {
    let l = cx.state.lang;
    let leaving = match cx.state.asked.shutdown_after {
        true => crate::texts::installed::leaving::POWER_OFF,
        false => crate::texts::installed::leaving::REBOOT,
    };
    match cx.state.source {
        Source::Rescue => widgets::footnote(c, cx.t, cx.f, y, l.pick(MORE_COMMANDS)),
        Source::Asked { .. } if installed => widgets::footnote(c, cx.t, cx.f, y, l.pick(leaving)),
        _ => y,
    }
}

/// What the wipe page lists of [`WIPES`] beyond everything, under the
/// partition each is in.
const WIPE_GROUPS: [(Text, &[Wipe]); 2] = [
    (
        DATA_PART,
        &[
            Wipe::AppData,
            Wipe::Media,
            Wipe::AppCaches,
            Wipe::SystemCaches,
            Wipe::Art,
            Wipe::Logs,
            Wipe::Updates,
        ],
    ),
    (crate::texts::wipe::part::CACHE, &[Wipe::Cache]),
];

/// What a factory reset erases, as the wipe page and an update that asks
/// for one tell it; in English as it goes on from what leads to it, its
/// first letter small.
fn factory_reset(s: &State) -> String {
    let also = s.lang.pick(match s.cache_partition {
        true => crate::lang::legacy([
            "metadata 和 cache 分区",
            "metadata 和 cache 分割區",
            "the metadata and cache partitions",
        ]),
        false => crate::lang::legacy(["metadata 分区", "metadata 分割區", "the metadata partition"]),
    });
    tr!(
        s.lang,
        "清空 data 分区，包括应用、账户、设置和内部存储，并格式化 {also}",
        "清空 data 分割區，包括應用程式、帳戶、設定和內部儲存空間，並格式化 {also}",
        "empties the data partition, apps, accounts, settings and internal storage included, and formats {also}"
    )
}

/// A choice of the wipe page as it lists it: its title, and exactly what it
/// erases.
fn wipe_row(w: Wipe, s: &State) -> (&'static str, String) {
    let l = s.lang;
    let (title, what): (Text, String) = match w {
        Wipe::All => (crate::texts::wipe::ALL, capitalized(l, &factory_reset(s))),
        Wipe::AppData => (
            crate::texts::wipe::part::APP_DATA,
            l.pick(crate::texts::wipe::what::APP_DATA)
            .into(),
        ),
        Wipe::Media => (
            crate::texts::wipe::part::MEDIA,
            l.pick(crate::texts::wipe::what::MEDIA).into(),
        ),
        Wipe::AppCaches => (
            crate::texts::wipe::part::APP_CACHES,
            l.pick(crate::texts::wipe::what::APP_CACHES)
            .into(),
        ),
        Wipe::SystemCaches => (
            crate::texts::wipe::part::SYSTEM_CACHES,
            l.pick(crate::texts::wipe::what::SYSTEM_CACHES)
            .into(),
        ),
        Wipe::Art => (
            crate::texts::wipe::part::ART,
            l.pick(crate::texts::wipe::what::ART)
            .into(),
        ),
        Wipe::Logs => {
            let own = match (l, s.phone_logs) {
                (_, 0) => String::new(),
                (Lang::En, 1) => ", and this phone's own log folder".into(),
                (_, n) => tr!(l, "，以及本机的 {n} 个日志文件夹", "，以及本機的 {n} 個記錄資料夾", ", and this phone's own {n} log folders"),
            };
            let what = tr!(
                l,
                "/data/tombstones、/data/anr、/data/system/dropbox、/data/misc/logd 等系统日志和崩溃记录，各用户的错误报告{own}",
                "/data/tombstones、/data/anr、/data/system/dropbox、/data/misc/logd 等系統記錄和當機記錄，各使用者的錯誤報告{own}",
                "System logs and crash records, such as /data/tombstones, /data/anr, /data/system/dropbox and /data/misc/logd, each user's bug reports{own}"
            );
            (crate::texts::wipe::part::LOGS, what)
        }
        Wipe::Updates => (
            crate::texts::wipe::part::UPDATES,
            l.pick(crate::texts::wipe::what::UPDATES).into(),
        ),
        Wipe::Cache => (
            crate::texts::wipe::EVERYTHING,
            l.pick(crate::texts::wipe::what::CACHE).into(),
        ),
    };
    (l.pick(title), what)
}

/// What to erase: everything, as a factory reset does, or parts of it on
/// their own, the cache partition among them where the phone has one; then
/// a swipe erases what is chosen.
pub fn wipe(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let chosen = |w: Wipe| s.wipe[WIPES.iter().position(|&x| x == w).expect("every choice is one of WIPES")];
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::wipe::RESET));
    let (name, sub) = wipe_row(Wipe::All, s);
    let all = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(name).subtitle(&sub).trail(Trail::Check(chosen(Wipe::All)))],
    );
    out.extend(hits(&all.rows, &[Action::WipeItem(Wipe::All)]));
    let note = l.pick(crate::texts::wipe::STAYS);
    let mut y = widgets::footnote(&mut c, t, f, all.bottom, note);
    for (title, items) in WIPE_GROUPS {
        let shown: Vec<Wipe> = items.iter().copied().filter(|&w| w != Wipe::Cache || s.cache_partition).collect();
        if shown.is_empty() {
            continue;
        }
        let below = widgets::section(&mut c, t, f, y, l.pick(title));
        let texts: Vec<(&str, String)> = shown.iter().map(|&w| wipe_row(w, s)).collect();
        let rows: Vec<Row> = shown
            .iter()
            .zip(&texts)
            .map(|(&w, (name, sub))| Row::new(name).subtitle(sub).trail(Trail::Check(chosen(w))))
            .collect();
        let list = widgets::list(&mut c, t, f, below, &rows);
        out.extend(hits(&list.rows, &shown.iter().map(|&w| Action::WipeItem(w)).collect::<Vec<_>>()));
        y = list.bottom;
    }
    let note = l.pick(crate::texts::wipe::NOTE);
    let y = widgets::footnote(&mut c, t, f, y, note);
    let (label, danger) = confirm_label(Confirm::Wipe, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::Wipe),
    });
    Frame::new(
        t,
        fit(c, t, track.y + track.h),
        out,
        l.pick(crate::texts::page::WIPE),
        top,
    )
}

/// A page laid out above the on-screen keyboard: where the keyboard's top
/// is, and its keys as touch targets.
fn keyboard_hits(c: &mut Canvas, t: &Theme, f: &Fonts, s: &State, enter: &str, out: &mut Vec<Hit>) -> f32 {
    let top = t.m.h as f32 - widgets::keyboard_height(t);
    if cfg!(feature = "pinyin-input") {
        out.extend(crate::ime::bar(c, t, f, &s.ime, top - crate::ime::height(t), false));
    }
    let keys = widgets::keyboard(c, t, f, s.lang, top, s.layer, s.key_held(), enter);
    out.extend(keys.into_iter().map(|(rect, cap)| Hit {
        rect,
        action: Action::Type(cap),
    }));
    top
}

/// Finding a feature or partition: the field where the title would be, the
/// button that leaves the search beside it, what it finds, and the
/// keyboard.
pub fn search(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let (at, cancel) = widgets::search_layout(t);
    let field = widgets::search_field(&mut c, t, f, at.x, at.w, at.y + at.h / 2.0, &s.search, l.pick(SEARCH_HINT));
    widgets::cancel_button(&mut c, t, cancel);
    out.push(Hit {
        rect: cancel,
        action: Action::CloseSearch,
    });
    let top = field.field.y + field.field.h + m.dp(12.0);
    let kb_top = m.h as f32 - widgets::keyboard_height(t) - crate::ime::height(t);
    if s.search.trim().is_empty() {
        let how = l.pick(crate::texts::search::HOW);
        widgets::footnote(&mut c, t, f, top, how);
    } else {
        let found = search::find_on(&s.search, l, |screen| s.offers(screen));
        if found.is_empty() {
            let typed = s.search.trim();
            let none = tr!(l, "未找到\"{typed}\"", "找不到「{typed}」", "Nothing found for “{typed}”");
            widgets::list(&mut c, t, f, top, &[Row::new(&none).trail(Trail::None)]);
        } else {
            let rows: Vec<Row> = found.iter().map(|e| Row::new(l.pick(e.title)).subtitle(l.pick(e.place))).collect();
            let list = widgets::list(&mut c, t, f, top, &rows);
            // The keyboard covers what does not fit above it.
            let shown = list.rows.iter().zip(&found).filter(|(r, _)| r.y + r.h <= kb_top);
            out.extend(shown.map(|(&rect, e)| Hit {
                rect,
                action: Action::Open(e.screen),
            }));
        }
    }
    let find = l.pick(crate::texts::search::SEARCH);
    keyboard_hits(&mut c, t, f, s, find, &mut out);
    if let Some(rect) = field.clear {
        out.push(Hit {
            rect,
            action: Action::ClearSearch,
        });
    }
    Frame {
        back: false,
        header: false,
        caret: Some(field.caret),
        ..Frame::new(t, c, out, find, 0.0)
    }
}

/// Backing up: what to take, where it goes, and how.
pub fn backup(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let images = widgets::list(
        &mut c,
        t,
        f,
        top,
        &[Row::new(l.pick(crate::lang::legacy(["分区镜像备份", "分割區映像檔備份", "Partition image backup"])))
            .icon(BLUE, Icon::Backup)
            .subtitle(l.pick(crate::lang::legacy([
                "单个分区或整个 super，raw .img",
                "單個分割區或整個 super，raw .img",
                "One partition or whole super, raw .img",
            ])))],
    );
    out.extend(hits(&images.rows, &[Action::Open(Screen::ImageBackup)]));
    let y = widgets::section(&mut c, t, f, images.bottom, l.pick(crate::texts::backup::CONTENTS));
    let system = fmt::snapshot_backup_note(l, &i.snapshot).unwrap_or(l.pick(crate::texts::backup::SYSTEM_HOLDS));
    let data = l.pick(match s.data == Data::Open {
        true => crate::texts::backup::DATA_HOLDS,
        false => crate::texts::backup::DATA_HOLDS_LOCKED,
    });
    let boot = l.pick(crate::texts::backup::BOOT_HOLDS);
    let modem = l.pick(crate::texts::backup::MODEM_HOLDS);
    let subs = [boot, system, data, modem];
    let available: Vec<_> = (0..BACKUP_PARTS.len()).filter(|&k| k != 3 || s.efs_backup_available).collect();
    let rows: Vec<Row> = BACKUP_PARTS
        .iter()
        .zip(subs)
        .enumerate()
        .filter(|(k, _)| available.contains(k))
        .map(|(k, (name, sub))| Row::new(l.pick(*name)).subtitle(sub).trail(Trail::Check(s.backup[k])))
        .collect();
    let parts = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(
        &parts.rows,
        &available.into_iter().map(Action::BackupPart).collect::<Vec<_>>(),
    ));
    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(crate::texts::backup::SAVE_TO));
    let storage = match (s.data.storage_open(), i.storage.free.map(fmt::gb)) {
        (true, Some(free)) => tr!(l, "剩余 {free}", "剩餘 {free}", "{free} free"),
        (true, None) => l.pick(crate::texts::backup::FREE_UNKNOWN).to_string(),
        (false, _) if s.data == Data::Erased => l.pick(DATA_ERASED).to_string(),
        (false, _) => l.pick(DECRYPT_NEEDED).to_string(),
    };
    let usb = l.pick(match &s.usb_drive {
        Some(UsbDrive::Shown) => crate::texts::backup::CONNECTED,
        Some(UsbDrive::Unreadable(_)) => USB_DRIVE_UNREADABLE,
        None => USB_DRIVE_WANTED,
    });
    let places = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(place_name(l, Place::Storage))
                .subtitle(&storage)
                .trail(Trail::Check(s.backup_to == Place::Storage)),
            Row::new(place_name(l, Place::UsbDrive))
                .subtitle(usb)
                .trail(Trail::Check(s.backup_to == Place::UsbDrive)),
        ],
    );
    out.extend(hits(
        &places.rows,
        &[Action::BackupTo(Place::Storage), Action::BackupTo(Place::UsbDrive)],
    ));
    let y = widgets::section(&mut c, t, f, places.bottom, l.pick(crate::texts::common::OPTIONS));
    let options = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(crate::texts::backup::COMPRESS))
                .subtitle(l.pick(crate::texts::backup::COMPRESS_NOTE))
                .switch(s.compress),
            Row::new(l.pick(crate::texts::backup::VERIFY))
                .subtitle(l.pick(crate::texts::backup::VERIFY_NOTE))
                .switch(s.verify),
        ],
    );
    out.extend(hits(
        &options.rows,
        &[Action::Toggle(Switch::Compress), Action::Toggle(Switch::Verify)],
    ));
    let note = l.pick(crate::texts::backup::NOTE);
    let y = widgets::footnote(&mut c, t, f, options.bottom, note);
    let (label, danger) = confirm_label(Confirm::Backup, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::Backup),
    });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(crate::texts::page::BACKUP), top)
}

pub fn image_backup(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::image::role::PARTITION));
    let y = match &s.image_partitions {
        None => widgets::footnote(&mut c, t, f, y, l.pick(crate::lang::legacy(["正在读取分区", "正在讀取分割區", "Reading partitions"]))),
        Some(Err(error)) => widgets::footnote(&mut c, t, f, y, error),
        Some(Ok(parts)) => {
            let details: Vec<_> = parts
                .iter()
                .map(|(name, size)| {
                    if name == "super" {
                        format!(
                            "{} · {}",
                            fmt::bytes(*size),
                            l.pick([
                                "共享分区，包含双槽逻辑分区",
                                "共用分割區，包含雙槽邏輯分割區",
                                "Shared, includes both slots' logical partitions"
                            ])
                        )
                    } else {
                        fmt::bytes(*size)
                    }
                })
                .collect();
            let rows: Vec<_> = parts
                .iter()
                .zip(&details)
                .enumerate()
                .map(|(index, ((name, _), detail))| {
                    Row::new(name)
                        .subtitle(detail)
                        .trail(Trail::Check(s.image_partition == Some(index)))
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            out.extend(hits(&list.rows, &(0..parts.len()).map(Action::ImagePartition).collect::<Vec<_>>()));
            list.bottom
        }
    };
    let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::backup::SAVE_TO));
    let places = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(place_name(l, Place::Storage)).trail(Trail::Check(s.backup_to == Place::Storage)),
            Row::new(place_name(l, Place::UsbDrive)).trail(Trail::Check(s.backup_to == Place::UsbDrive)),
        ],
    );
    out.extend(hits(
        &places.rows,
        &[Action::BackupTo(Place::Storage), Action::BackupTo(Place::UsbDrive)],
    ));
    let y = widgets::footnote(
        &mut c,
        t,
        f,
        places.bottom,
        l.pick(crate::lang::legacy([
            "仅列当前槽位；super 为双槽共享。保存到 Skyward/images，完成后读回校验。",
            "僅列目前槽位；super 為雙槽共用。儲存到 Skyward/images，完成後讀回驗證。",
            "Current slot only; super is shared by both slots. Saved to Skyward/images and verified after saving.",
        ])),
    );
    let (label, danger) = confirm_label(Confirm::ImageBackup, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::ImageBackup),
    });
    Frame::new(
        t,
        fit(c, t, track.y + track.h),
        out,
        l.pick(crate::lang::legacy(["分区镜像备份", "分割區映像檔備份", "Partition image backup"])),
        top,
    )
}

/// The backups found, newest first, to restore one; those that cannot be
/// read are listed after them, to be deleted.
pub fn restore(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::restore::BACKUPS));
    let bottom = if s.backups.is_empty() {
        widgets::list(
            &mut c,
            t,
            f,
            y,
            &[Row::new(l.pick(crate::texts::restore::NONE)).trail(Trail::None)],
        )
        .bottom
    } else {
        let texts: Vec<(String, String)> = s
            .backups
            .iter()
            .map(|b| match (&b.held, &b.image) {
                (Ok(h), Some(image)) => (
                    b.name.clone(),
                    format!("{} · {} · {}", image.partition, fmt::bytes(h.size), place_name(l, b.place)),
                ),
                (Ok(h), None) => (
                    fmt::datetime(l, h.made, s.zone()),
                    format!("{} · {} · {}", fmt::parts(l, &h.parts), fmt::bytes(h.size), place_name(l, b.place)),
                ),
                (Err(_), _) => (b.name.clone(), format!("{} · {}", l.pick(UNREADABLE), place_name(l, b.place))),
            })
            .collect();
        let rows: Vec<Row> = texts.iter().map(|(title, sub)| Row::new(title).subtitle(sub)).collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(list.rows.iter().enumerate().map(|(k, &rect)| Hit {
            rect,
            action: Action::OpenBackup(k),
        }));
        list.bottom
    };
    let note = l.pick(crate::lang::legacy([
        "扫描 Skyward/backups 和 Skyward/images。内部存储需先解密。",
        "掃描 Skyward/backups 與 Skyward/images。內部儲存空間需先解密。",
        "Scan Skyward/backups and Skyward/images. Decrypt internal storage first.",
    ]));
    let bottom = widgets::footnote(&mut c, t, f, bottom, note);
    Frame::new(t, fit(c, t, bottom), out, l.pick(crate::texts::page::RESTORE), top)
}

/// One backup: when and where it was made, and which of its parts to
/// restore. One made on another phone is not restored here, and one that
/// cannot be read can only be deleted.
pub fn restore_backup(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let (entry, chosen) = s.restore.as_ref().expect("the restore page shows a backup");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let location = l.pick(crate::texts::common::LOCATION);
    let y = if let (Some(image), Ok(held)) = (&entry.image, &entry.held) {
        let y = widgets::info_card(
            &mut c,
            t,
            f,
            top,
            &[
                (l.pick(crate::lang::legacy(["镜像", "映像檔", "Image"])), format!("{}.img", image.partition)),
                (l.pick(crate::lang::legacy(["目标分区", "目標分割區", "Target partition"])), image.partition.clone()),
                (l.pick(crate::texts::common::SIZE), fmt::bytes(held.size)),
                (location, format!("{}/Skyward/images/{}", place_name(l, entry.place), entry.name)),
            ],
        );
        let note = l.pick(crate::lang::legacy([
            "校验 SHA-256 后覆盖上面标明的原分区，不自动改到当前槽位。镜像未记录机型，请确认来自本机。",
            "驗證 SHA-256 後覆寫上方標明的原分割區，不自動改至目前槽位。映像檔未記錄機型，請確認來自本機。",
            "Verify SHA-256, then overwrite the named original partition, not the current slot. This format has no device identity; confirm it belongs to this phone.",
        ]));
        let mut y = widgets::footnote(&mut c, t, f, y, note);
        if image.partition == "super" {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y,
                l.pick(crate::lang::legacy([
                    "super 为双槽共享，恢复会覆盖双槽逻辑分区，完成后需重启。",
                    "super 為雙槽共用，還原會覆寫雙槽邏輯分割區，完成後需重新啟動。",
                    "Shared super contains both slots. Restoring overwrites both slots' logical partitions and requires a reboot.",
                ])),
            );
        }
        let (label, danger) = confirm_label(Confirm::Restore, s);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit {
            rect: track,
            action: Action::Confirm(Confirm::Restore),
        });
        track.y + track.h
    } else {
        match &entry.held {
            Err(why) => {
                let facts = [
                    (l.pick(crate::texts::restore::FOLDER), entry.name.clone()),
                    (location, place_name(l, entry.place).to_string()),
                ];
                let y = widgets::info_card(&mut c, t, f, top, &facts);
                let note = tr!(
                    l,
                    "无法读取该备份，备份可能未完成或已损坏。{why}",
                    "無法讀取這份備份，備份可能未完成或已損壞。{why}",
                    "This backup can't be read. It may not have finished, or it may be damaged. {why}"
                );
                widgets::footnote(&mut c, t, f, y, &note)
            }
            Ok(held) => {
                let mut facts = vec![
                    (l.pick(["备份时间", "備份時間", "Made"]), fmt::datetime(l, held.made, s.zone())),
                    (l.pick(["大小", "大小", "Size"]), fmt::bytes(held.size)),
                    (location, place_name(l, entry.place).to_string()),
                ];
                if let Some(system) = &held.system {
                    facts.push((l.pick(crate::texts::restore::SYSTEM_VERSION), system.clone()));
                }
                if let Some(slot) = held.slot {
                    facts.push((l.pick(crate::texts::slot::TITLE), slot.letter().to_string()));
                }
                let y = widgets::info_card(&mut c, t, f, top, &facts);
                if held.device != i.codename {
                    let device = &held.device;
                    let note = tr!(
                        l,
                        "该备份来自其他设备（{device}），无法恢复到本机。",
                        "這份備份來自其他裝置（{device}），無法還原到本機。",
                        "This backup is from another device ({device}) and can't be restored to this phone."
                    );
                    widgets::footnote(&mut c, t, f, y, &note)
                } else {
                    let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::restore::CONTENTS));
                    let kept: Vec<usize> = (0..BACKUP_PARTS.len()).filter(|&k| held.parts[k]).collect();
                    let rows: Vec<Row> = kept
                        .iter()
                        .map(|&k| Row::new(l.pick(BACKUP_PARTS[k])).trail(Trail::Check(chosen[k])))
                        .collect();
                    let list = widgets::list(&mut c, t, f, y, &rows);
                    out.extend(list.rows.iter().zip(&kept).map(|(&rect, &k)| Hit {
                        rect,
                        action: Action::RestorePart(k),
                    }));
                    let note = l.pick(crate::texts::restore::OVERWRITES);
                    let mut y = widgets::footnote(&mut c, t, f, list.bottom, note);
                    if chosen[2] {
                        let note = l.pick(crate::texts::restore::DATA_NOTE);
                        y = widgets::footnote(&mut c, t, f, y, note);
                    }
                    // The boot partitions' vbmeta checks the system's: one of
                    // another version without the other may not start.
                    if let (Some(was), Some(now)) = (&held.system, &i.os_version)
                        && was != now
                        && chosen[0] != chosen[1]
                    {
                        let warn = tr!(
                            l,
                            "该备份来自 {was}，本机当前为 {now}。仅恢复启动分区或系统分区中的一项，可能导致无法开机。",
                            "這份備份來自 {was}，本機目前為 {now}。僅還原啟動分割區或系統分割區中的一項，可能導致無法開機。",
                            "This backup is from {was}, and this phone runs {now}. Restoring only one of the boot and system partitions may keep the phone from starting."
                        );
                        y = widgets::footnote(&mut c, t, f, y, &warn);
                    }
                    let (label, danger) = confirm_label(Confirm::Restore, cx.state);
                    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
                    out.push(Hit {
                        rect: track,
                        action: Action::Confirm(Confirm::Restore),
                    });
                    track.y + track.h
                }
            }
        }
    };
    let delete = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.dp(28.0),
        &[Row::new(l.pick(crate::texts::restore::DELETE))
            .tint(t.pal.danger)
            .trail(Trail::None)],
    );
    out.extend(hits(&delete.rows, &[Action::AskDelete]));
    let mut bottom = delete.bottom;
    if s.asking_delete {
        let ask = l.pick(crate::texts::restore::ASK_DELETE);
        let y = widgets::footnote(&mut c, t, f, delete.bottom, ask);
        let (label, danger) = confirm_label(Confirm::DeleteBackup, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit {
            rect: track,
            action: Action::Confirm(Confirm::DeleteBackup),
        });
        bottom = track.y + track.h;
    }
    Frame::new(t, fit(c, t, bottom), out, l.pick(crate::texts::restore::TITLE), top)
}

/// The share of its pass the part under way has done.
fn part_share(a: &BackupAt) -> f32 {
    if a.of == 0 { 1.0 } else { share(a.done, a.of).min(1.0) }
}

/// How far backing up or restoring has come over all, 0 to 1, and the
/// bytes of the parts the pass under way has gone through, the part under
/// way by its share. Each part counts by its bytes in each pass: a restore
/// checks all its parts first, and a backup checks them last.
fn job_progress(job: &BackupJob, a: &BackupAt) -> (f32, u64) {
    let total: u64 = job.plan.iter().map(|(_, n)| n).sum();
    let before: u64 = job.plan[..a.item].iter().map(|(_, n)| n).sum();
    let passed = before as f64 + f64::from(part_share(a)) * job.plan[a.item].1 as f64;
    let second = job.checks && a.checking != job.restore;
    let passes = if job.checks { 2.0 } else { 1.0 };
    let over = (if second { total as f64 } else { 0.0 } + passed) / (passes * total.max(1) as f64);
    (over as f32, passed as u64)
}

/// What a backup, or a restore, has done to a part, and does to it, in
/// each language.
fn job_words(restore: bool) -> (Text, Text) {
    match restore {
        true => (crate::texts::backup::job::RESTORED, crate::texts::backup::job::RESTORING),
        false => (crate::texts::backup::job::BACKED_UP, crate::texts::backup::job::BACKING_UP),
    }
}

const VERIFIED: Text = crate::texts::backup::job::VERIFIED;
const VERIFYING: Text = crate::texts::backup::job::VERIFYING;
const WAITING_TURN: Text = crate::texts::common::WAITING;
const DONE: Text = crate::texts::common::DONE;

/// Where part `k` of a backup or restore stands, as its row says it.
fn part_state(l: Lang, job: &BackupJob, k: usize) -> String {
    use std::cmp::Ordering;
    let (did, doing) = job_words(job.restore);
    let ended = job.outcome.is_some();
    let a = match (&job.outcome, job.at) {
        (Some(BackupEnd::Saved(_) | BackupEnd::ImageSaved(_)), _) => return l.pick(if job.checks { VERIFIED } else { did }).to_string(),
        (Some(BackupEnd::Restored { .. }), _) => return l.pick(did).to_string(),
        (_, None) => {
            return l
                .pick(if ended {
                    crate::texts::backup::job::NOT_STARTED
                } else {
                    WAITING_TURN
                })
                .to_string();
        }
        (_, Some(a)) => a,
    };
    let state = match k.cmp(&a.item) {
        Ordering::Less if a.checking => VERIFIED,
        Ordering::Less => did,
        Ordering::Equal if ended => crate::texts::common::UNFINISHED,
        Ordering::Equal => {
            return format!(
                "{} {}%",
                l.pick(if a.checking { VERIFYING } else { doing }),
                percent(part_share(&a))
            );
        }
        // Ahead of the pass under way: a backup checks its parts once all
        // are written, and a restore writes them once all are checked.
        Ordering::Greater => match (a.checking, job.restore, ended) {
            (true, false, false) => did,
            (true, false, true) => crate::texts::backup::job::NOT_VERIFIED,
            (false, false, false) | (true, true, false) => WAITING_TURN,
            (false, false, true) => crate::texts::backup::job::NOT_BACKED_UP,
            (false, true, false) => VERIFIED,
            (_, true, true) => crate::texts::backup::job::NOT_RESTORED,
        },
    };
    l.pick(state).to_string()
}

/// Backing up or restoring: how far it has come over all, each part and
/// where it stands, and a way to stop while stopping breaks nothing. Once
/// it has ended, how, and where to go.
pub fn backup_job(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let job = s.backup_job.as_ref().expect("the backup page shows work");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let (did, doing) = job_words(job.restore);
    let icon = if job.restore { Icon::Restore } else { Icon::Backup };
    let writing = job.at.is_some_and(|a| !a.checking);
    let mut y = match &job.outcome {
        None => {
            let (title, detail, done) = match job.at {
                Some(a) => {
                    let (over, passed) = job_progress(job, &a);
                    let total = fmt::bytes(job.plan.iter().map(|(_, n)| n).sum());
                    let (verb, past) = if a.checking {
                        (l.pick(VERIFYING), l.pick(VERIFIED))
                    } else {
                        (l.pick(doing), l.pick(did))
                    };
                    let (name, passed) = (&job.plan[a.item].0, fmt::bytes(passed));
                    (
                        format!("{verb} {name}"),
                        tr!(
                            l,
                            "{past} {passed}，共 {total}",
                            "{past} {passed}，共 {total}",
                            "{past} {passed} of {total}"
                        ),
                        over,
                    )
                }
                None if job.restore => (
                    l.pick(crate::texts::backup::job::READING).to_string(),
                    l.pick(crate::texts::backup::job::CHECKING)
                        .to_string(),
                    0.0,
                ),
                None => (
                    l.pick(GETTING_READY).to_string(),
                    l.pick(crate::texts::backup::job::SIZING)
                        .to_string(),
                    0.0,
                ),
            };
            widgets::progress_card(&mut c, t, f, top, &title, &detail, Some(done))
        }
        Some(end) => {
            let gone = l.pick(if job.restore {
                crate::texts::installed::NOTHING_WRITTEN
            } else {
                crate::texts::backup::job::NOTHING_SAVED
            });
            let failed = l.pick(if job.restore {
                RESTORE_FAILED
            } else {
                crate::texts::backup::job::BACKUP_FAILED
            });
            let (title, color, detail) = match end {
                BackupEnd::ImageSaved(path) => (l.pick(BACKUP_COMPLETE), GREEN, path.clone()),
                BackupEnd::Saved(name) => {
                    let saved = match job.place {
                        Place::Storage => tr!(
                            l,
                            "已保存到内部存储的 Skyward/backups/{name}",
                            "已儲存到內部儲存空間的 Skyward/backups/{name}",
                            "Saved to Skyward/backups/{name} in internal storage"
                        ),
                        Place::UsbDrive => tr!(
                            l,
                            "已保存到 U 盘的 Skyward/backups/{name}",
                            "已儲存到 USB 隨身碟的 Skyward/backups/{name}",
                            "Saved to Skyward/backups/{name} on the USB drive"
                        ),
                    };
                    (l.pick(BACKUP_COMPLETE), GREEN, saved)
                }
                BackupEnd::Restored { .. } => (
                    l.pick(RESTORE_COMPLETE),
                    GREEN,
                    l.pick(crate::texts::backup::job::USES_RESTORED)
                    .to_string(),
                ),
                BackupEnd::Stopped => (l.pick(crate::texts::common::STOPPED), SLATE, gone.to_string()),
                BackupEnd::Refused(r) => {
                    let title = l.pick(if job.restore {
                        crate::texts::backup::job::CANT_RESTORE
                    } else {
                        crate::texts::backup::job::CANT_BACK_UP
                    });
                    (title, ORANGE, fmt::backup_refusal(l, r, job.restore))
                }
                BackupEnd::Failed(_) if job.restore && writing => {
                    let detail = l.pick(crate::texts::backup::job::PARTLY_WRITTEN);
                    (failed, RED, detail.to_string())
                }
                BackupEnd::Failed(_) => (failed, RED, gone.to_string()),
            };
            widgets::list(&mut c, t, f, top, &[standing(title, color, icon, &detail)]).bottom
        }
    };
    let keep = l.pick(match job.place {
        Place::Storage => crate::lang::legacy(["请不要关机。", "請不要關機。", "Don't power off"]),
        Place::UsbDrive => crate::lang::legacy([
            "请不要拔出 U 盘或关机。",
            "請不要拔出 USB 隨身碟或關機。",
            "Keep the USB drive in and don't power off",
        ]),
    });
    match &job.outcome {
        None => {
            let note = match (job.restore, job.at) {
                (false, _) => tr!(
                    l,
                    "备份完成前，{keep}停止后将删除未完成的备份。",
                    "備份完成前，{keep}停止後將刪除未完成的備份。",
                    "{keep} until the backup is done. Stopping deletes the unfinished backup."
                ),
                (true, Some(a)) if !a.checking => tr!(l, "恢复完成前，{keep}", "還原完成前，{keep}", "{keep} until the restore is done."),
                (true, _) => l
                    .pick(crate::texts::backup::job::NOTHING_UNTIL_VERIFIED)
                    .to_string(),
            };
            y = widgets::footnote(&mut c, t, f, y, &note);
            // A restore that has begun writing goes on to its end.
            if !(job.restore && writing) {
                let label = l.pick(if job.stopping {
                    crate::texts::common::STOPPING
                } else {
                    STOP
                });
                let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
                if !job.stopping {
                    out.extend(hits(&stop.rows, &[Action::StopBackup]));
                }
                y = stop.bottom;
            }
        }
        Some(end) => {
            match end {
                BackupEnd::Restored { unmade } if !unmade.is_empty() => {
                    let folders = unmade.join(l.pick(crate::texts::list::SEPARATOR));
                    let note = tr!(
                        l,
                        "以下文件夹所属的用户已不存在，其中的内容未恢复：{folders}",
                        "以下資料夾所屬的使用者已不存在，其中的內容未還原：{folders}",
                        "The users these folders belonged to no longer exist, so what they hold wasn't restored: {folders}"
                    );
                    y = widgets::footnote(&mut c, t, f, y, &note);
                }
                BackupEnd::Failed(why) => y = widgets::footnote(&mut c, t, f, y, why),
                _ => {}
            }
            let (rows, acts): (Vec<Row>, Vec<Action>) = match end {
                BackupEnd::Restored { .. } => (
                    vec![
                        Row::new(l.pick(["重启到系统", "重新啟動到系統", "Reboot to system"])).icon(ORANGE, Icon::Power),
                        Row::new(l.pick(DONE)).trail(Trail::None),
                    ],
                    vec![Action::RebootTo(Target::System), Action::Back],
                ),
                _ => (vec![Row::new(l.pick(DONE)).trail(Trail::None)], vec![Action::Back]),
            };
            let next = widgets::list(&mut c, t, f, y + t.m.card_gap, &rows);
            out.extend(hits(&next.rows, &acts));
            y = next.bottom;
        }
    }
    // The parts come after what can be done, which a long list would push
    // off the screen.
    if !job.plan.is_empty() {
        let heading = l.pick(if job.restore {
            crate::texts::restore::CONTENTS
        } else {
            crate::texts::backup::CONTENTS
        });
        let y0 = widgets::section(&mut c, t, f, y, heading);
        let states: Vec<String> = (0..job.plan.len()).map(|k| part_state(l, job, k)).collect();
        let rows: Vec<Row> = job
            .plan
            .iter()
            .zip(&states)
            .map(|((name, _), state)| Row::new(name).trail(Trail::Text(state)))
            .collect();
        y = widgets::list(&mut c, t, f, y0, &rows).bottom;
    }
    let title = l.pick(match (&job.outcome, job.restore) {
        (None, restore) => job_words(restore).1,
        (Some(BackupEnd::Saved(_) | BackupEnd::ImageSaved(_)), _) => BACKUP_COMPLETE,
        (Some(BackupEnd::Restored { .. }), _) => RESTORE_COMPLETE,
        (Some(_), false) => crate::texts::backup::job::BACKUP_INCOMPLETE,
        (Some(_), true) => crate::texts::backup::job::RESTORE_INCOMPLETE,
    });
    Frame {
        back: job.outcome.is_some(),
        ..Frame::new(t, fit(c, t, y), out, title, top)
    }
}

const BACKUP_COMPLETE: Text = crate::texts::backup::job::BACKUP_COMPLETE;
const RESTORE_COMPLETE: Text = crate::texts::backup::job::RESTORE_COMPLETE;
const RESTORE_FAILED: Text = crate::texts::backup::job::RESTORE_FAILED;
const STOP: Text = crate::texts::common::STOP;

/// Tools for the experienced.
pub fn advanced(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["文件与网络", "檔案與網路", "Files and network"])));
    let files = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(FILES)).icon(BLUE, Icon::Folder),
            Row::new(l.pick(crate::lang::legacy(["浏览器", "瀏覽器", "Browser"]))).icon(BLUE, Icon::Globe),
            Row::new(l.pick(TERMINAL)).icon(SLATE, Icon::Terminal),
        ],
    );
    let mut out = hits(
        &files.rows,
        &[
            Action::Open(Screen::Files),
            Action::Open(Screen::Browser),
            Action::Open(Screen::Terminal),
        ],
    );
    let y = widgets::section(
        &mut c,
        t,
        f,
        files.bottom,
        l.pick(crate::lang::legacy(["系统与分区", "系統與分割區", "System and partitions"])),
    );
    let system = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(UPDATE))
                .icon(GREEN, Icon::Layers)
                .trail(Trail::Value(update_value(cx))),
            Row::new(l.pick(crate::lang::legacy(["Root 模块", "Root 模組", "Root modules"]))).icon(GREEN, Icon::Shield),
            Row::new(l.pick(crate::lang::legacy(["分区管理", "分割區管理", "Partitions"]))).icon(BLUE, Icon::Layers),
            Row::new(l.pick(AVB)).icon(RED, Icon::Shield),
        ],
    );
    out.extend(hits(
        &system.rows,
        &[
            Action::Open(Screen::Update),
            Action::Open(Screen::RootModules),
            Action::Open(Screen::Partitions),
            Action::Open(Screen::Avb),
        ],
    ));
    let y = widgets::section(&mut c, t, f, system.bottom, l.pick(crate::lang::legacy(["维护与诊断", "維護與診斷", "Maintenance"])));
    let maintenance = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(FILESYSTEMS)).icon(RED, Icon::Mount),
            Row::new(l.pick(RELABEL)).icon(PURPLE, Icon::Shield),
            Row::new(l.pick(LOGS)).icon(ORANGE, Icon::Log),
        ],
    );
    out.extend(hits(
        &maintenance.rows,
        &[
            Action::Open(Screen::Filesystems),
            Action::Open(Screen::Relabel),
            Action::Open(Screen::Logs),
        ],
    ));
    Frame::new(t, fit(c, t, maintenance.bottom), out, l.pick(ADVANCED), top)
}

const RELABEL: Text = crate::texts::page::RELABEL;
const USB_MANAGER: Text = crate::lang::legacy(["U 盘管理", "USB 隨身碟管理", "USB drive manager"]);

/// The two AVB checks, what each does, and the switch that turns it off.
///
/// **Turning a check off is two acts and turning one on is one.** A switch that
/// could be turned off by a stray slide is what this page was, and what it did
/// was make a phone whose bootloader is unlocked start a system nothing
/// checked: the owner asked for that to take more than one touch. So turning
/// one off asks for this page's own second step — `Action::AvbConfirmOpen` —
/// which draws what it costs and offers the only swipe that writes
/// (`Confirm::AvbDisable`). Turning one on keeps the single swipe, since it can
/// only make the phone check more.
pub fn avb(cx: &Ctx) -> Frame {
    use skyward_install::commands::avbctl::Check;
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    let mut y = top;
    let state = match &s.avb {
        Some(Ok(state)) => Some(state),
        Some(Err(why)) => {
            y = widgets::footnote(&mut c, t, f, y, why);
            None
        }
        None => {
            y = widgets::footnote(&mut c, t, f, y, l.pick(crate::lang::legacy(["正在读取 vbmeta", "正在讀取 vbmeta", "Reading vbmeta"])));
            None
        }
    };
    if state.is_some_and(|state| !state.unlocked) {
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(crate::lang::legacy([
                "未确认 Bootloader 已解锁，禁止修改。",
                "未確認 Bootloader 已解鎖，禁止修改。",
                "Bootloader unlock is not confirmed. Changes are blocked.",
            ])),
        );
        return Frame::new(t, fit(c, t, y), out, l.pick(AVB), top);
    }
    // The second confirmation stands in the page's place: what turning a check
    // off means, which check it is, and the one swipe that writes it. Back
    // leaves it without changing anything, and so does Cancel.
    if let (true, Some((_, check, _))) = (s.avb_confirming, &s.avb_pending) {
        let name = match check {
            Check::Verification => l.pick(crate::lang::legacy(["启动验证", "啟動驗證", "Boot verification"])),
            Check::Verity => "dm-verity",
        };
        let heading = tr!(l, "关闭 {name}？", "關閉 {name}？", "Turn off {name}?");
        y = widgets::section_first(&mut c, t, f, y, &heading);
        let consequence = l.pick(crate::lang::legacy([
            "关闭后，这台手机的引导器不再校验将要启动的系统。引导器已解锁时，任何未经手机自身密钥签名的系统都会照常启动——这是安装第三方系统的必要一步，也是这台手机从此不再能确认自己启动的是什么。",
            "關閉後，這台手機的引導器不再驗證即將啟動的系統。引導器已解鎖時，任何未經手機自身金鑰簽署的系統都會照常啟動——這是安裝第三方系統的必要一步，也是這台手機從此不再能確認自己啟動的是什麼。",
            "With this off, the phone's bootloader no longer checks the system it is about to start. On a phone whose bootloader is unlocked, any system at all will start, whether or not it was signed with this phone's own keys. It is what installing a third-party system needs, and it is also this phone no longer being able to tell what it is starting.",
        ]));
        y = widgets::paragraph(&mut c, t, f, y, consequence);
        let writing = l.pick(crate::lang::legacy([
            "只写入当前槽位 vbmeta 的 4 个标志字节，写入前原镜像会备份，写入后会读回校验。重启后生效。",
            "只寫入目前槽位 vbmeta 的 4 個旗標位元組，寫入前原映像檔會備份，寫入後會讀回驗證。重新啟動後生效。",
            "Only the 4 flag bytes of this slot's vbmeta are written; the image as it was is kept first, and what was written is read back. It takes effect from the next start.",
        ]));
        y = widgets::footnote(&mut c, t, f, y, writing);
        let (label, danger) = confirm_label(Confirm::AvbDisable, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
        out.push(Hit {
            rect: track,
            action: Action::Confirm(Confirm::AvbDisable),
        });
        y = track.y + track.h;
        let back = widgets::button(&mut c, t, f, y + t.m.dp(12.0), l.pick(crate::lang::legacy(["返回", "返回", "Go back"])));
        out.push(Hit {
            rect: back,
            action: Action::CancelAvb,
        });
        y = back.y + back.h;
        // Nothing of the page underneath is drawn or reachable: the only
        // writes on offer here are this swipe and leaving.
        return Frame::new(t, fit(c, t, y), out, l.pick(AVB), top);
    }
    if let Some(state) = state {
        let target = if state.suffix.is_empty() {
            "vbmeta".to_string()
        } else {
            format!("vbmeta{}", state.suffix)
        };
        y = widgets::info_card(
            &mut c,
            t,
            f,
            y,
            &[
                (l.pick(crate::lang::legacy(["目标分区", "目標分割區", "Target partition"])), target),
                (
                    l.pick(crate::lang::legacy(["引导器", "引導器", "Bootloader"])),
                    l.pick(crate::texts::specs::UNLOCKED).to_string(),
                ),
            ],
        );
        // A card's bottom is where it ends, not where the next one starts:
        // the gap between two of them is the one every other page leaves.
        let rows: Vec<Row> = [
            (
                Check::Verification,
                l.pick(crate::lang::legacy(["启动验证", "啟動驗證", "Boot verification"])),
                l.pick(crate::lang::legacy([
                    "引导器校验各分区的签名",
                    "引導器驗證各分割區的簽章",
                    "The bootloader checks each partition's signature",
                ])),
            ),
            (
                Check::Verity,
                "dm-verity",
                l.pick(crate::lang::legacy([
                    "读取时逐块校验分区内容",
                    "讀取時逐塊驗證分割區內容",
                    "Each block is checked as it is read",
                ])),
            ),
        ]
        .iter()
        .map(|(check, label, what)| {
            let disabled = match &s.avb_pending {
                Some((_, pending, want)) if pending == check => !*want,
                _ => state.disabled(*check),
            };
            Row::new(label).subtitle(what).switch(!disabled)
        })
        .collect();
        let list = widgets::list(&mut c, t, f, y + t.m.card_gap, &rows);
        out.extend(hits(
            &list.rows,
            &[Action::AvbToggle(Check::Verification), Action::AvbToggle(Check::Verity)],
        ));
        y = list.bottom;
        let explain = l.pick(crate::lang::legacy([
            "关闭后，安装了非官方系统的手机才能正常启动。改动只写入当前槽位的 vbmeta，重启后生效；写入前原镜像会备份到 /tmp，并可在「vbmeta 备份」中留一份长期副本。",
            "停用後，安裝了非官方系統的手機才能正常啟動。變更只寫入目前槽位的 vbmeta，重新啟動後生效；寫入前原映像檔會備份到 /tmp，並可在「vbmeta 備份」中留一份長期副本。",
            "Turning these off is what lets a phone start a system that was not signed with its own keys. Only this slot's vbmeta is written, from the next start; the image as it was is kept in /tmp, and a lasting copy can be kept under vbmeta copies.",
        ]));
        y = widgets::footnote(&mut c, t, f, y, explain);
        // Before anything here is changed, the way to keep a copy of what the
        // slot holds now: a write to vbmeta is the one act on this page the
        // owner cannot undo from the phone unless a copy was kept first.
        let keep = widgets::list(
            &mut c,
            t,
            f,
            y + t.m.card_gap,
            &[Row::new(l.pick(crate::lang::legacy(["vbmeta 备份与写回", "vbmeta 備份與寫回", "vbmeta copies and writing one back"]))).trail(Trail::None)],
        );
        out.extend(hits(&keep.rows, &[Action::Open(Screen::Vbmeta)]));
        y = keep.bottom;
        if let Some((_, check, want)) = &s.avb_pending {
            let name = match check {
                Check::Verification => l.pick(crate::lang::legacy(["启动验证", "啟動驗證", "Boot verification"])),
                Check::Verity => "dm-verity",
            };
            let said = match want {
                true => tr!(l, "将关闭 {name}", "將停用 {name}", "Turn off {name}"),
                false => tr!(l, "将开启 {name}", "將啟用 {name}", "Turn on {name}"),
            };
            y = widgets::footnote(&mut c, t, f, y, &said);
            // Turning a check OFF is the direction that can leave a phone
            // unable to start, so it never writes from here: this asks for the
            // second confirmation, which says what that means on its own page
            // and has its own swipe. Turning one ON keeps the single swipe,
            // since it can only make the phone check more.
            if *want {
                let ask = l.pick(crate::lang::legacy(["继续关闭…", "繼續關閉…", "Continue turning it off…"]));
                let next = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(ask).trail(Trail::None)]);
                out.extend(hits(&next.rows, &[Action::AvbConfirmOpen]));
                y = next.bottom;
            } else {
                let (label, danger) = confirm_label(Confirm::Avb, cx.state);
                let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
                out.push(Hit {
                    rect: track,
                    action: Action::Confirm(Confirm::Avb),
                });
                y = track.y + track.h;
            }
            let cancel = widgets::button(&mut c, t, f, y + t.m.dp(12.0), l.pick(crate::texts::common::CANCEL));
            out.push(Hit {
                rect: cancel,
                action: Action::CancelAvb,
            });
            y = cancel.y + cancel.h;
        }
        if let Some(notice) = &s.avb_notice {
            y = widgets::footnote(&mut c, t, f, y, notice);
        }
    }
    Frame::new(t, fit(c, t, y), out, l.pick(AVB), top)
}
const FILESYSTEMS: Text = crate::texts::page::FILESYSTEMS;
const STOPPED: Text = crate::texts::common::STOPPED;
const STOPPING: Text = crate::texts::common::STOPPING;

/// Giving the data partition's files the SELinux labels the installed
/// system's rules give them: what it does, and a swipe to start it; then
/// how far it has come, a way to stop it, and how it ended.
pub fn relabel(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(RELABEL);
    let Some(job) = &s.relabel else {
        let what = l.pick(crate::texts::relabel::WHAT);
        let mut y = widgets::paragraph(&mut c, t, f, top, what);
        let note = match s.data {
            Data::Open | Data::Locked { .. } => None,
            Data::Erased => Some(crate::texts::relabel::DATA_ERASED),
            Data::Closed | Data::Opening | Data::Failed(_) => Some(crate::texts::relabel::OPEN_FIRST),
        };
        if let Some(note) = note {
            y = widgets::footnote(&mut c, t, f, y, l.pick(note));
        }
        let (label, danger) = confirm_label(Confirm::Relabel, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit {
            rect: track,
            action: Action::Confirm(Confirm::Relabel),
        });
        return Frame::new(t, c, out, title, top);
    };
    let repair = Verb::of(&crate::app::PartKind::Repair);
    let heading = match &job.outcome {
        None => repair.going(l),
        Some(JobEnd::Done) => repair.complete(l),
        Some(JobEnd::Stopped) => l.pick(STOPPED).into(),
        Some(JobEnd::Failed(_)) => repair.failed(l),
    };
    let detail = match (job.total, job.checked) {
        (0, _) if job.outcome.is_none() => l.pick(GETTING_READY).to_string(),
        (total, checked) => tr!(
            l,
            "已检查 {checked} 项，共 {total} 项",
            "已檢查 {checked} 項，共 {total} 項",
            "{checked} of {total} checked"
        ),
    };
    let y = widgets::progress_card(&mut c, t, f, top, &heading, &detail, Some(share(job.checked, job.total)));
    match &job.outcome {
        None => {
            let label = l.pick(if job.stopping { STOPPING } else { STOP });
            let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopRelabel]));
            }
        }
        Some(end) => {
            let changed = job.changed;
            let note = match end {
                JobEnd::Done if changed == 0 => l
                    .pick(crate::texts::relabel::NOTHING_NEEDED)
                    .to_string(),
                JobEnd::Done => tr!(
                    l,
                    "已按规则重新设置 {changed} 项的 SELinux 上下文。",
                    "已按規則重新設定 {changed} 項的 SELinux 上下文。",
                    "{changed} entries got the SELinux context the rules give them."
                ),
                JobEnd::Stopped => tr!(
                    l,
                    "已停止，已重新设置 {changed} 项。",
                    "已停止，已重新設定 {changed} 項。",
                    "Stopped, with {changed} entries set again."
                ),
                JobEnd::Failed(why) => tr!(l, "修复失败：{why}", "修復失敗：{why}", "Repair failed: {why}"),
            };
            let y = widgets::footnote(&mut c, t, f, y, &note);
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
        }
    }
    Frame {
        back: job.outcome.is_some(),
        ..Frame::new(t, c, out, title, top)
    }
}

/// The partitions whose filesystems can be repaired or made afresh as
/// another, each with what it holds and its size.
pub fn filesystems(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::mount::PARTITIONS));
    let bottom = match &s.filesystems {
        None => {
            widgets::list(
                &mut c,
                t,
                f,
                y,
                &[Row::new(l.pick(crate::texts::filesystems::READING)).trail(Trail::None)],
            )
            .bottom
        }
        Some(Err(why)) => {
            let y = widgets::list(
                &mut c,
                t,
                f,
                y,
                &[Row::new(l.pick(crate::texts::mount::UNREADABLE)).trail(Trail::None)],
            )
            .bottom;
            widgets::footnote(&mut c, t, f, y, why)
        }
        Some(Ok(rows)) if rows.is_empty() => {
            widgets::list(
                &mut c,
                t,
                f,
                y,
                &[Row::new(l.pick(crate::texts::filesystems::NONE)).trail(Trail::None)],
            )
            .bottom
        }
        Some(Ok(rows)) => {
            let subs: Vec<String> = rows
                .iter()
                .map(|r| format!("{} · {}", fs_held(l, r, &s.data), fmt::bytes(r.size)))
                .collect();
            let shown: Vec<Row> = rows.iter().zip(&subs).map(|(r, sub)| Row::new(&r.name).subtitle(sub)).collect();
            let list = widgets::list(&mut c, t, f, y, &shown);
            out.extend(hits(&list.rows, &(0..rows.len()).map(Action::OpenFilesystem).collect::<Vec<_>>()));
            let note = l.pick(crate::texts::filesystems::NOTE);
            widgets::footnote(&mut c, t, f, list.bottom, note)
        }
    };
    Frame::new(t, fit(c, t, bottom), out, l.pick(FILESYSTEMS), top)
}

/// What a partition holds, as its row and page name it: the data
/// partition's is read only once it is open, and until then the row says
/// where it stands, as the home page does.
fn fs_held(l: Lang, row: &FsRow, data: &Data) -> String {
    match (&row.filesystem, row.name == DATA_PARTITION) {
        (Some(filesystem), _) => filesystem.clone(),
        (None, true) => fmt::data(l, data).into(),
        (None, false) => l.pick(crate::texts::filesystems::UNRECOGNIZED).into(),
    }
}

/// One partition: what it holds, what can be done to it, and a swipe to do
/// what is chosen.
pub fn filesystem(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let Some(Ok(rows)) = &s.filesystems else {
        unreachable!("a partition's page opens from the list read")
    };
    let row = &rows[s.fs_chosen];
    let data = row.name == DATA_PARTITION;
    let facts = [
        (l.pick(crate::texts::filesystems::MOUNT_POINT), row.mount_point.clone()),
        (l.pick(crate::texts::filesystems::FILESYSTEM), fs_held(l, row, &s.data)),
        (l.pick(crate::texts::common::SIZE), fmt::bytes(row.size)),
    ];
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    let y = widgets::section(&mut c, t, f, y, l.pick(ACTIONS));
    let chosen = |action: &FsAction| s.fs_action == *action;
    let remakes: Vec<String> = row
        .others
        .iter()
        .map(|fs| tr!(l, "更换为 {fs}", "更換為 {fs}", "Change to {fs}"))
        .collect();
    let made: Vec<String> = row.others.iter().map(|_| remade(row, s)).collect();
    let repair = Row::new(l.pick(crate::texts::filesystems::REPAIR))
        .subtitle(l.pick(crate::texts::filesystems::REPAIR_NOTE))
        .trail(Trail::Check(chosen(&FsAction::Repair)));
    let mut shown = vec![repair];
    let mut actions = vec![Action::ChooseFs(FsChoice::Repair)];
    for (k, fs) in row.others.iter().enumerate() {
        shown.push(
            Row::new(&remakes[k])
                .subtitle(&made[k])
                .trail(Trail::Check(chosen(&FsAction::Remake(fs.clone())))),
        );
        actions.push(Action::ChooseFs(FsChoice::Remake(k)));
    }
    let list = widgets::list(&mut c, t, f, y, &shown);
    out.extend(hits(&list.rows, &actions));
    let mut y = list.bottom;
    if row.others.is_empty() {
        let note = l.pick(crate::texts::filesystems::CANT_CHANGE);
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if data {
        let note = l.pick(crate::texts::filesystems::DATA_CLOSES);
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if !row.others.is_empty() {
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(crate::texts::filesystems::CANT_UNDO),
        );
    }
    let (label, danger) = confirm_label(Confirm::Filesystem, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::Filesystem),
    });
    Frame::new(t, fit(c, t, track.y + track.h), out, partition_title(l, &row.name), top)
}

/// The title of the page of a partition the filesystem page lists.
fn partition_title(l: Lang, name: &str) -> &'static str {
    l.pick(match name {
        DATA_PARTITION => DATA_PART,
        "metadata" => crate::texts::wipe::part::METADATA,
        "cache" => crate::texts::wipe::part::CACHE,
        _ => crate::texts::image::role::PARTITION,
    })
}

/// What making `row` afresh as another filesystem erases: the data
/// partition goes as a factory reset goes, with /metadata and the cache
/// partition beside it.
fn remade(row: &FsRow, s: &State) -> String {
    match row.name == DATA_PARTITION {
        true => {
            let parts = s.lang.pick(match s.cache_partition {
                true => crate::lang::legacy([
                    "data、cache、metadata 分区",
                    "data、cache、metadata 分割區",
                    "the data, cache and metadata partitions",
                ]),
                false => crate::lang::legacy(["data、metadata 分区", "data、metadata 分割區", "the data and metadata partitions"]),
            });
            tr!(
                s.lang,
                "格式化 {parts}，清空本机的全部数据",
                "格式化 {parts}，清空本機的全部資料",
                "Formats {parts}, emptying all of this phone's data"
            )
        }
        false => {
            let name = &row.name;
            tr!(
                s.lang,
                "格式化 {name} 分区，清空其中的全部内容",
                "格式化 {name} 分割區，清空其中的全部內容",
                "Formats the {name} partition, emptying all it holds"
            )
        }
    }
}

/// Work on whole partitions: each partition and where it stands, what the
/// program said of the last, and once it has ended, how. What the system
/// asked for leaves on its own once it went through every partition, and
/// offers where to go otherwise; what the user chose goes back to its page.
pub fn part_work(cx: &Ctx) -> Frame {
    use crate::app::{ItemEnd, ItemState, PartKind};
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let work = s.part_work.as_ref().expect("the page of partition work shows work");
    let mut c = long_page(cx);
    let (mut out, mut rows) = (Vec::new(), None);
    let top = t.m.content_top;
    let verb = Verb::of(&work.kind);
    // Work on one partition fails; work through several may get partway.
    let (title, several) = match &work.kind {
        PartKind::Disable => (crate::texts::parts::title::WIPE, true),
        PartKind::Unshare => (crate::texts::parts::title::UNSHARE, true),
        PartKind::Repair => (crate::texts::parts::title::REPAIR, false),
        PartKind::Remake(_) => (crate::texts::parts::title::FORMAT, false),
    };
    let failed = |state: &ItemState| matches!(state, ItemState::Ended(ItemEnd::Failed(_)));
    let short = matches!(work.outcome, Some(Err(_))) || work.items.iter().any(|(_, state)| failed(state));
    let heading = match (&work.outcome, short, several) {
        (None, _, _) => verb.going(l),
        (Some(_), false, _) => verb.complete(l),
        (Some(_), true, true) => verb.incomplete(l),
        (Some(_), true, false) => verb.failed(l),
    };
    let mut y = widgets::section_first(&mut c, t, f, top, &heading);
    if !work.items.is_empty() {
        let states: Vec<String> = work
            .items
            .iter()
            .map(|(_, state)| match state {
                ItemState::Waiting if work.outcome.is_some() => verb.not_done(l),
                ItemState::Waiting => l.pick(WAITING_TURN).into(),
                ItemState::Working => verb.going(l),
                ItemState::Ended(end) => fmt::item_end(l, end, verb),
            })
            .collect();
        let held: Vec<Option<&str>> = work
            .items
            .iter()
            .map(|(_, state)| match state {
                ItemState::Ended(ItemEnd::NotShared(fs)) if !fs.is_empty() => Some(fs.as_str()),
                _ => None,
            })
            .collect();
        let shown: Vec<Row> = work
            .items
            .iter()
            .zip(&states)
            .zip(&held)
            .map(|(((name, _), state), held)| {
                let row = Row::new(name).trail(Trail::Text(state));
                match held {
                    Some(fs) => row.subtitle(fs),
                    None => row,
                }
            })
            .collect();
        y = widgets::list(&mut c, t, f, y, &shown).bottom;
    }
    let leaving = match s.asked.shutdown_after {
        true => crate::texts::parts::leaving::POWER_OFF,
        false => crate::texts::parts::leaving::REBOOT,
    };
    let mut notes: Vec<String> = Vec::new();
    match &work.outcome {
        None if work.asked => notes.push(l.pick(leaving).into()),
        None => notes.push(verb.hold_on(l)),
        Some(Err(stop)) => {
            let (what, detail) = fmt::part_stop(l, stop, &cx.info.codename);
            notes.push(what);
            notes.extend(detail);
        }
        Some(Ok(())) => {}
    }
    for (name, state) in &work.items {
        if let ItemState::Ended(ItemEnd::Failed(Some(why))) = state {
            let failed = verb.failed_on(l, name);
            notes.push(tr!(l, "{failed}：{why}", "{failed}：{why}", "{failed}: {why}"));
        }
    }
    for note in &notes {
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let (true, Some(reason)) = (work.asked, &s.asked.reason) {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(REASON), reason.clone())]);
    }
    match (&work.outcome, work.asked) {
        (None, _) => {}
        (Some(_), true) if !short => {}
        // The system's work ran without the slot's partitions, which going
        // on in the recovery brings up first, as leaving fastboot does.
        (Some(_), true) => {
            let below = widgets::section(&mut c, t, f, y, l.pick(ACTIONS));
            let rows: Vec<Row> = AFTERWARDS
                .iter()
                .map(|&(title, _)| Row::new(l.pick(title)).trail(Trail::None))
                .collect();
            let actions = AFTERWARDS.map(|(_, action)| if action == Action::Dismiss { Action::EnterRecovery } else { action });
            let choices = widgets::list(&mut c, t, f, below, &rows);
            out.extend(hits(&choices.rows, &actions));
            y = choices.bottom;
        }
        (Some(_), false) => {
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
            y = finish.bottom;
        }
    }
    if !work.output.is_empty() {
        let below = widgets::section(&mut c, t, f, y, l.pick(crate::texts::parts::OUTPUT));
        rows = Some(Rows {
            edges: widgets::output_edges(&c, t, f, below, &work.output),
            action: None,
        });
        y = below;
    }
    let back = work.outcome.is_some() && !work.asked;
    Frame {
        rows,
        back,
        ..Frame::new(t, fit(c, t, y), out, l.pick(title), top)
    }
}

const ACTIONS: Text = crate::texts::common::ACTIONS;

/// The system on the phone now; the newest system the phone maker's
/// server knows, and downloading its full package to install it; and
/// where an update the system installed stands, merging it into the slot it
/// went to: how far a merge under way has come, and a swipe to merge one
/// that waits there for the system to start it.
pub fn update(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    if let Standing::Merging(done) = s.update {
        let share = done as f32;
        let p = percent(share);
        let detail = tr!(l, "已完成 {p}%", "已完成 {p}%", "{p}% done");
        let y = widgets::progress_card(
            &mut c,
            t,
            f,
            top,
            l.pick(crate::texts::snapshot::MERGING),
            &detail,
            Some(share),
        );
        let note = l.pick(crate::texts::update::MERGING_NOTE);
        let y = widgets::footnote(&mut c, t, f, y, note);
        return Frame::new(t, fit(c, t, y), out, l.pick(UPDATE), top);
    }
    let mut y = top;
    if let Some(v) = &i.os_version {
        y = widgets::section_first(&mut c, t, f, y, l.pick(crate::texts::common::THIS_PHONE));
        y = widgets::info_card(&mut c, t, f, y, &[(l.pick(crate::texts::update::CURRENT_VERSION), v.clone())]);
    }
    if s.update_server {
        let online_update = l.pick(crate::texts::update::ONLINE);
        y = if y == top {
            widgets::section_first(&mut c, t, f, y, online_update)
        } else {
            widgets::section(&mut c, t, f, y, online_update)
        };
        y = online(&mut c, cx, y, &mut out);
        if s.recovery_partitions {
            let row = Row::new(l.pick(KEEP_RECOVERY)).trail(Trail::Switch(s.keep_recovery));
            let options = widgets::list(&mut c, t, f, y + t.m.card_gap, &[row]);
            out.extend(hits(&options.rows, &[Action::Toggle(Switch::KeepRecovery)]));
            y = options.bottom;
        }
    }
    let next = match &s.boot_slots {
        Some(Ok(slots)) => Some(slots.active),
        _ => None,
    };
    let (title, detail) = fmt::update(l, &s.update, next);
    let color = match s.update {
        Standing::None => SLATE,
        Standing::Merged => GREEN,
        Standing::Failed(_) | Standing::Unknown => RED,
        _ => ORANGE,
    };
    let mut row = Row::new(title).icon(color, Icon::Layers).trail(Trail::None);
    if let Some(detail) = &detail {
        row = row.subtitle(detail);
    }
    let merging = l.pick(crate::texts::update::MERGING);
    y = if y == top {
        widgets::section_first(&mut c, t, f, y, merging)
    } else {
        widgets::section(&mut c, t, f, y, merging)
    };
    y = widgets::list(&mut c, t, f, y, &[row]).bottom;
    if s.update == Standing::Waiting {
        let (label, danger) = confirm_label(Confirm::Merge, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
        out.push(Hit {
            rect: track,
            action: Action::Confirm(Confirm::Merge),
        });
        let note = l.pick(crate::texts::update::CANT_GO_BACK);
        y = widgets::footnote(&mut c, t, f, track.y + track.h, note);
    }
    Frame::new(t, fit(c, t, y), out, l.pick(UPDATE), top)
}

/// What downloading or checking for updates needs first, while the phone
/// has joined no network that reaches the internet.
const WLAN_FIRST: Text = crate::texts::update::WLAN_FIRST;
const CHECKING: Text = crate::texts::update::CHECKING;
const CHECK_FOR_UPDATES: Text = crate::texts::update::CHECK;
const DOWNLOADING: Text = crate::texts::update::DOWNLOADING;
const CHECKING_PART: Text = crate::texts::update::CHECKING_PART;
const PAUSED: Text = crate::lang::legacy(["已暂停下载", "已暫停下載", "Download paused"]);
const PAUSING: Text = crate::texts::update::PAUSING;
const PAUSE: Text = crate::texts::update::PAUSE;
// What an earlier download left that the server's package no longer fits:
// going on starts it again rather than adding to it.
const STARTS_AGAIN: Text = crate::lang::legacy([
    "先前下载的部分与当前安装包不符，将继续时会重新下载",
    "先前下載的部分與目前安裝檔不符，將繼續時會重新下載",
    "What was downloaded before doesn't match this package, so continuing downloads it again",
]);
const CONTINUE_DOWNLOAD: Text = crate::texts::update::RESUME;
const INSTALLED_PACKAGE: Text = crate::texts::update::DOWNLOADED_TO_STORAGE;
const DELETE_PACKAGE: Text = crate::lang::legacy(["删除安装包", "刪除安裝檔", "Delete the package"]);

/// Asking the update server for the newest system, then its full package:
/// downloading it, how far that has come, and installing it once it is
/// whole; then what to note before installing it.
fn online(c: &mut Canvas, cx: &Ctx, y: f32, out: &mut Vec<Hit>) -> f32 {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let release = match &s.check {
        Check::Asking => return widgets::list(c, t, f, y, &[Row::new(l.pick(CHECKING)).trail(Trail::None)]).bottom,
        Check::Answered(Some(r)) => r,
        other => {
            let mut row = Row::new(l.pick(CHECK_FOR_UPDATES));
            if !s.online() {
                row = row.subtitle(l.pick(WLAN_FIRST));
            }
            let check = widgets::list(c, t, f, y, &[row]);
            out.extend(hits(&check.rows, &[Action::CheckUpdate]));
            return match other {
                Check::Failed(why) => widgets::footnote(c, t, f, check.bottom, &fmt::unanswered(l, why)),
                Check::Answered(None) => {
                    let none = l.pick(crate::texts::update::NONE_KNOWN);
                    widgets::footnote(c, t, f, check.bottom, none)
                }
                _ => check.bottom,
            };
        }
    };
    let newer = i.os_version.as_deref() != Some(release.version.as_str());
    let version = &release.version;
    let title = if newer {
        tr!(l, "新版本 {version}", "新版本 {version}", "New version {version}")
    } else {
        l.pick(crate::texts::update::UP_TO_DATE).to_string()
    };
    let size = match release.length {
        Some(length) => fmt::file_size(length),
        None => fmt::package_size(&release.size),
    };
    let package = tr!(l, "完整更新包 {size}", "完整更新檔 {size}", "Full OTA package {size}");
    let mut y = widgets::list(
        c,
        t,
        f,
        y,
        &[Row::new(&title).icon(GREEN, Icon::Layers).subtitle(&package).trail(Trail::None)],
    )
    .bottom;
    match (&s.download, s.have) {
        // Downloading: what has come, how fast, and the pause that keeps
        // what came of it. Nothing here says the download failed — a paused
        // one has not.
        (Some(job), _) if job.outcome.is_none() => {
            let (heading, detail, share) = match job.progress {
                None => (l.pick(GETTING_READY), String::new(), None),
                Some(p) => downloaded(l, p, job.rate),
            };
            let download_theme = Theme {
                pal: crate::theme::Palette { accent: GREEN, ..t.pal },
                ..*t
            };
            y = widgets::progress_card(c, &download_theme, f, y + t.m.card_gap, heading, &detail, share);
            let pause = widgets::list(
                c,
                t,
                f,
                y + t.m.card_gap,
                &[Row::new(l.pick(if job.stopping { PAUSING } else { PAUSE })).trail(Trail::None)],
            );
            if !job.stopping {
                out.extend(hits(&pause.rows, &[Action::PauseDownload]));
            }
            y = pause.bottom;
        }
        // Paused: what it kept is still there, and going on from it costs
        // only the rest.
        (Some(job), Have::Part(kept)) if matches!(job.outcome, Some(DownloadEnd::Stopped)) => {
            let (detail, share) = kept_detail(l, release.length, kept);
            y = widgets::progress_card(c, t, f, y + t.m.card_gap, l.pick(PAUSED), &detail, share);
            let resume = widgets::list(c, t, f, y + t.m.card_gap, &[Row::new(l.pick(CONTINUE_DOWNLOAD)).subtitle(&detail)]);
            out.extend(hits(&resume.rows, &[Action::Download]));
            y = resume.bottom;
        }
        // Whole: installing it, and deleting it from internal storage once
        // nothing is downloading or installing.
        (_, Have::Whole(size)) => {
            let row = Row::new(l.pick(crate::texts::page::INSTALL)).subtitle(l.pick(INSTALLED_PACKAGE));
            let install = widgets::list(c, t, f, y + t.m.card_gap, &[row]);
            out.extend(hits(&install.rows, &[Action::InstallDownloaded]));
            let mut bottom = install.bottom;
            // A package cannot be deleted while an install reads it, and a
            // download under way does not leave one to delete.
            let busy =
                !matches!(s.install, Install::Idle | Install::Finished { .. }) || s.download.as_ref().is_some_and(|j| j.outcome.is_none());
            if !busy {
                let size = fmt::file_size(size);
                let forget = tr!(
                    l,
                    "删除后，下次安装需重新下载 {size}",
                    "刪除後，下次安裝需重新下載 {size}",
                    "Deleting it means downloading {size} again to install"
                );
                let delete = widgets::list(
                    c,
                    t,
                    f,
                    bottom + t.m.dp(28.0),
                    &[Row::new(l.pick(DELETE_PACKAGE)).tint(t.pal.danger).trail(Trail::None)],
                );
                out.extend(hits(&delete.rows, &[Action::AskDeleteDownload]));
                bottom = delete.bottom;
                if s.asking_delete {
                    let ask = l.pick(crate::lang::legacy([
                        "删除该安装包？删除后需重新下载。",
                        "刪除這個安裝檔？刪除後需重新下載。",
                        "Delete this package? It would have to be downloaded again.",
                    ]));
                    let y = widgets::footnote(c, t, f, delete.bottom, ask);
                    let (label, danger) = confirm_label(Confirm::DeleteDownload, s);
                    let track = widgets::confirm(c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
                    out.push(Hit {
                        rect: track,
                        action: Action::Confirm(Confirm::DeleteDownload),
                    });
                    bottom = widgets::footnote(c, t, f, track.y + track.h, &forget);
                }
            }
            y = bottom;
        }
        (job, have) => {
            // A failed download says why; a paused one never reaches here
            // with bytes kept, since its own arm above shows the card.
            if let Some(DownloadEnd::Failed(why)) = job.as_ref().and_then(|j| j.outcome.as_ref()) {
                y = widgets::footnote(c, t, f, y, &fmt::download_failure(l, why));
            }
            let title = l.pick(if matches!(have, Have::Part(_)) {
                CONTINUE_DOWNLOAD
            } else {
                crate::texts::update::DOWNLOAD
            });
            let had = match have {
                Have::Part(done) => {
                    let done = fmt::file_size(done);
                    Some(tr!(l, "已下载 {done}", "已下載 {done}", "{done} downloaded"))
                }
                _ => None,
            };
            let sub = if s.data == Data::Erased {
                Some(l.pick(DATA_ERASED))
            } else if !s.data.storage_open() {
                Some(l.pick(DECRYPT_NEEDED))
            } else if !s.online() {
                Some(l.pick(WLAN_FIRST))
            } else {
                had.as_deref()
            };
            let row = Row::new(title);
            let download = widgets::list(c, t, f, y + t.m.card_gap, &[sub.map_or(row, |sub| row.subtitle(sub))]);
            out.extend(hits(&download.rows, &[Action::Download]));
            y = download.bottom;
        }
    }
    let saved = tr!(
        l,
        "保存到 /sdcard/{DOWNLOADS}，安装后保留文件。",
        "儲存至 /sdcard/{DOWNLOADS}，安裝後保留檔案。",
        "Saved in /sdcard/{DOWNLOADS}; the file is kept after installation."
    );
    let points = [
        l.pick(crate::lang::legacy([
            "安装前请备份数据，是否清除以更新包要求和所选选项为准。",
            "安裝前請備份資料，是否清除依更新檔要求與所選選項而定。",
            "Back up first. Data erasure depends on the package and selected options.",
        ])),
        saved.as_str(),
    ];
    widgets::notes(c, t, f, y + t.m.card_gap, l.pick(NOTES), &points)
}

const NOTES: Text = crate::texts::common::NOTES;

/// What a progress card says of a download, and the share of it done:
/// what an earlier download left being checked, or the rest coming at
/// `rate` bytes a second.
fn downloaded(l: Lang, progress: Progress, rate: u64) -> (&'static str, String, Option<f32>) {
    match progress {
        Progress::Checking { done, total } => (
            l.pick(CHECKING_PART),
            format!("{} / {}", fmt::file_size(done), fmt::file_size(total)),
            Some(share(done, total)),
        ),
        Progress::Downloading { done, total } => (
            l.pick(DOWNLOADING),
            format!(
                "{}% · {} / {} · {}/s",
                percent(share(done, total)),
                fmt::file_size(done),
                fmt::file_size(total),
                fmt::file_size(rate)
            ),
            Some(share(done, total)),
        ),
    }
}

/// What a paused download kept of the package, and what is left to fetch:
/// the bytes kept, and what the rest comes to where the package's length is
/// known. A part longer than the package the server offers now is of another
/// package, and is let go when the download starts again, so nothing of it
/// is said to be kept.
fn kept_detail(l: Lang, length: Option<u64>, kept: u64) -> (String, Option<f32>) {
    let kept_size = fmt::file_size(kept);
    match length.filter(|&total| total > 0) {
        Some(total) if kept > total => (l.pick(STARTS_AGAIN).to_string(), None),
        Some(total) => {
            let rest = fmt::file_size(total.saturating_sub(kept));
            let detail = tr!(
                l,
                "已保留 {kept_size}，还需 {rest}",
                "已保留 {kept_size}，還需 {rest}",
                "{kept_size} kept, {rest} to go"
            );
            (detail, Some(share(kept, total)))
        }
        None => (tr!(l, "已保留 {kept_size}", "已保留 {kept_size}", "{kept_size} kept"), None),
    }
}

/// Skyward's own update: the build running now; Skyward's newest release
/// on GitHub, and updating to it; then what to note before updating.
pub fn skyward_update(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut out = Vec::new();
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::common::THIS_PHONE));
    let y = widgets::info_card(
        &mut c,
        t,
        f,
        y,
        &[(l.pick(crate::texts::update::CURRENT_VERSION), VERSION.to_string())],
    );
    let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::update::ONLINE));
    let y = skyward_online(&mut c, cx, y, &mut out);
    let y = widgets::section(&mut c, t, f, y, l.pick(MIRROR_SECTION));
    let mirror = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(MIRROR)).switch(cx.state.mirror)]);
    out.extend(hits(&mirror.rows, &[Action::Toggle(Switch::Mirror)]));
    let y = widgets::footnote(&mut c, t, f, mirror.bottom, l.pick(MIRROR_NOTE));
    let points = [
        l.pick(crate::lang::legacy([
            "校验发布签名和镜像后，仅更新当前槽位的 recovery，不改动系统、用户数据或 AVB。",
            "驗證發佈簽章與映像檔後，僅更新目前槽位的 recovery，不變更系統、使用者資料或 AVB。",
            "After release signature and image checks, update only the current slot's recovery. System, user data and AVB are unchanged.",
        ])),
        l.pick(crate::texts::skyward_update::REBOOT_TO_USE),
    ];
    let y = widgets::notes(&mut c, t, f, y + t.m.card_gap, l.pick(NOTES), &points);
    Frame::new(t, fit(c, t, y), out, l.pick(SKYWARD_UPDATE), top)
}

/// Asking GitHub for Skyward's newest release, then updating to it: its
/// build for the phone downloaded and written into the recovery partitions,
/// how far that has come, and how it ended.
fn skyward_online(c: &mut Canvas, cx: &Ctx, y: f32, out: &mut Vec<Hit>) -> f32 {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let gap = t.m.card_gap;
    let newest = match &s.skyward_check {
        SkywardCheck::Asking => return widgets::list(c, t, f, y, &[Row::new(l.pick(CHECKING)).trail(Trail::None)]).bottom,
        SkywardCheck::Answered(n) => n,
        other => {
            let mut row = Row::new(l.pick(CHECK_FOR_UPDATES));
            if !s.online() {
                row = row.subtitle(l.pick(WLAN_FIRST));
            }
            let check = widgets::list(c, t, f, y, &[row]);
            out.extend(hits(&check.rows, &[Action::CheckSkyward]));
            return match other {
                SkywardCheck::Failed(why) => widgets::footnote(c, t, f, check.bottom, &fmt::unanswered(l, why)),
                _ => check.bottom,
            };
        }
    };
    let version = &newest.version;
    let (title, detail) = match (newest.newer, &newest.build) {
        (true, Some(build)) => {
            let size = fmt::file_size(build.size);
            (
                tr!(l, "新版本 {version}", "新版本 {version}", "New version {version}"),
                tr!(l, "安装包 {size}", "安裝包 {size}", "Package {size}"),
            )
        }
        (true, None) => (
            tr!(l, "新版本 {version}", "新版本 {version}", "New version {version}"),
            l.pick(crate::texts::skyward_update::NO_PACKAGE)
                .to_string(),
        ),
        (false, _) => (
            l.pick(crate::texts::update::UP_TO_DATE).to_string(),
            tr!(
                l,
                "最新发布的版本为 {version}",
                "最新發佈的版本為 {version}",
                "The newest release is {version}"
            ),
        ),
    };
    let y = widgets::list(
        c,
        t,
        f,
        y,
        &[Row::new(&title).icon(GREEN, Icon::Install).subtitle(&detail).trail(Trail::None)],
    )
    .bottom;
    if !newest.newer || newest.build.is_none() {
        return y;
    }
    match &s.skyward_job {
        Some(SkywardJob {
            outcome: Some(SkywardEnd::Ready(slot)),
            ..
        }) => {
            let target = format!("recovery{}", slot.suffix());
            let detail = tr!(
                l,
                "签名与 SHA-256 验证通过，目标：{target}",
                "簽章與 SHA-256 驗證通過，目標：{target}",
                "Signature and SHA-256 verified. Target: {target}"
            );
            let y = widgets::footnote(c, t, f, y + gap, &detail);
            let (label, danger) = confirm_label(Confirm::SkywardUpdate, s);
            let track = widgets::confirm(c, t, f, y + gap, label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(Confirm::SkywardUpdate),
            });
            track.y + track.h
        }
        Some(job) if job.outcome.is_none() => {
            let (heading, detail, share) = match job.stage {
                None => (l.pick(GETTING_READY), String::new(), None),
                Some(SkywardStage::Download(p)) => downloaded(l, p, job.rate),
                Some(SkywardStage::Write { done, total }) => {
                    let part = share(done, total);
                    let p = percent(part);
                    (
                        l.pick(crate::texts::skyward_update::WRITING),
                        tr!(l, "已完成 {p}%", "已完成 {p}%", "{p}% done"),
                        Some(part),
                    )
                }
            };
            let y = widgets::progress_card(c, t, f, y + gap, heading, &detail, share);
            if matches!(job.stage, Some(SkywardStage::Write { .. })) {
                return widgets::footnote(
                    c,
                    t,
                    f,
                    y,
                    l.pick(crate::texts::skyward_update::KEEP_POWER),
                );
            }
            let stop = widgets::list(
                c,
                t,
                f,
                y + gap,
                &[Row::new(l.pick(if job.stopping { STOPPING } else { STOP })).trail(Trail::None)],
            );
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopSkywardUpdate]));
            }
            stop.bottom
        }
        Some(SkywardJob {
            outcome: Some(SkywardEnd::Updated(slots)),
            ..
        }) => {
            let names = fmt::slots(l, slots);
            let detail = match (l, slots.len()) {
                (Lang::En, 1) => format!("Skyward {version} is in {names}'s recovery partition."),
                _ => tr!(
                    l,
                    "已将 Skyward {version} 刷入{names} 的 recovery 分区。",
                    "已將 Skyward {version} 刷入{names} 的 recovery 分割區。",
                    "Skyward {version} is in the recovery partitions of {names}."
                ),
            };
            let y = widgets::list(
                c,
                t,
                f,
                y + gap,
                &[standing(l.pick(crate::texts::skyward_update::SUCCEEDED), GREEN, Icon::Install, &detail)],
            )
            .bottom;
            let restart = widgets::list(
                c,
                t,
                f,
                y + gap,
                &[Row::new(l.pick(crate::texts::installed::REBOOT_TO_RECOVERY)).icon(ORANGE, Icon::Power)],
            );
            out.extend(hits(&restart.rows, &[Action::RebootTo(Target::Recovery)]));
            restart.bottom
        }
        job => {
            let mut y = y;
            match job.as_ref().and_then(|j| j.outcome.as_ref()) {
                Some(SkywardEnd::Stopped) => {
                    y = widgets::footnote(c, t, f, y, l.pick(crate::texts::skyward_update::STOPPED))
                }
                Some(SkywardEnd::Download(why)) => y = widgets::footnote(c, t, f, y, &fmt::skyward_download_failure(l, why)),
                Some(SkywardEnd::NotWritten(why)) => {
                    y = widgets::footnote(
                        c,
                        t,
                        f,
                        y,
                        l.pick(crate::texts::skyward_update::NOT_WRITTEN),
                    );
                    y = widgets::footnote(c, t, f, y, why);
                }
                Some(SkywardEnd::WriteFailed(slots, why)) => {
                    let names = fmt::slots(l, slots);
                    let warning = match (l, slots.len()) {
                        (Lang::En, 1) => format!(
                            "{}'s recovery partition was only partly written, so rebooting to recovery may not start Skyward. Update again, and reboot once it's done.",
                            capitalized(l, &names)
                        ),
                        _ => tr!(
                            l,
                            "{names} 的 recovery 分区仅写入了部分内容，重启到恢复模式可能无法进入 Skyward。请重新更新，完成后再重启。",
                            "{names} 的 recovery 分割區僅寫入了部分內容，重新啟動到復原模式可能無法進入 Skyward。請重新更新，完成後再重新啟動。",
                            "The recovery partitions of {names} were only partly written, so rebooting to recovery may not start Skyward. Update again, and reboot once it's done."
                        ),
                    };
                    y = widgets::footnote(c, t, f, y, &warning);
                    y = widgets::footnote(c, t, f, y, why);
                }
                Some(SkywardEnd::Updated(_) | SkywardEnd::Ready(_)) => unreachable!("a ready or completed update has its own controls"),
                None => {}
            }
            let mut row = Row::new(l.pick(crate::texts::skyward_update::DOWNLOAD_AND_UPDATE));
            if !s.online() {
                row = row.subtitle(l.pick(WLAN_FIRST));
            }
            let update = widgets::list(c, t, f, y + gap, &[row]);
            out.extend(hits(&update.rows, &[Action::UpdateSkyward]));
            update.bottom
        }
    }
}

/// The WLAN switch; then the network chosen and how joining it goes, and
/// the networks in range, strongest first.
pub fn wlan(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let w = s.wlan.as_ref().expect("the WLAN page shows on a phone whose WLAN is brought up");
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let on = !matches!(w.power, Power::Off | Power::Failed(_));
    let switch = widgets::list(
        &mut c,
        t,
        f,
        top,
        &[
            Row::new(l.pick(WLAN)).switch(on),
            Row::new(l.pick(REMEMBER_WLAN)).switch(s.remember_wlan),
        ],
    );
    out.extend(hits(
        &switch.rows,
        &[Action::Toggle(Switch::Wlan), Action::Toggle(Switch::RememberWlan)],
    ));
    let mut y = switch.bottom;
    match &w.power {
        Power::Off => {
            let note = l.pick(crate::texts::wlan::TURN_ON_NOTE);
            y = widgets::footnote(&mut c, t, f, y, note)
        }
        Power::Failed(why) => {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y,
                &tr!(l, "无法开启 WLAN：{why}", "無法開啟 Wi-Fi：{why}", "Couldn't turn on Wi-Fi: {why}"),
            )
        }
        Power::Starting => y = widgets::footnote(&mut c, t, f, y, l.pick(crate::texts::wlan::TURNING_ON_WLAN)),
        Power::On => {
            let locked = |s: Security| !matches!(s, Security::Open | Security::Owe);
            // The network chosen stands out in the accent, as the stock
            // page shows the one it is on.
            if let Some(j) = &w.joined {
                let found = w.networks.iter().find(|n| n.ssid == j.ssid);
                let (signal, bands) = match (&j.stage, found) {
                    (Joining::Connected(link), found) => (Some(link.signal), found.map_or(Bands::of(link.frequency), |n| n.bands)),
                    (_, Some(n)) => (Some(n.signal), n.bands),
                    (_, None) => (None, Bands::default()),
                };
                let tag = fmt::band_tag(bands);
                let mut row = Row::new(&j.name)
                    .fan(signal.map_or(0, fmt::signal_level))
                    .subtitle(fmt::joining(l, &j.stage));
                row = row.trail(Trail::Network {
                    locked: locked(j.security),
                });
                if let Some(tag) = &tag {
                    row = row.tag(tag);
                }
                let joined = widgets::list_toned(&mut c, t, f, y + t.m.card_gap, &[row], &Tone::lit(t));
                out.extend(hits(&joined.rows, &[Action::OpenJoined]));
                out.push(Hit {
                    rect: widgets::round_button(t, joined.rows[0]),
                    action: Action::OpenJoined,
                });
                y = joined.bottom;
                if let Joining::Connected(link) = &j.stage
                    && let Some(note) = link.reach.as_ref().and_then(|r| fmt::reach(l, r))
                {
                    y = widgets::footnote(&mut c, t, f, y, &note);
                }
            }
            let others: Vec<(usize, &Network)> = w
                .networks
                .iter()
                .enumerate()
                .filter(|(_, n)| w.joined.as_ref().is_none_or(|j| j.ssid != n.ssid))
                .collect();
            let nearby = l.pick(crate::texts::wlan::NEARBY);
            let (below, rescan) = widgets::section_action(&mut c, t, f, y, nearby, l.pick(crate::texts::wlan::REFRESH));
            out.push(Hit {
                rect: rescan,
                action: Action::Rescan,
            });
            y = below;
            if others.is_empty() {
                let looking = l.pick(crate::texts::wlan::LOOKING);
                y = widgets::footnote(&mut c, t, f, y - t.m.dp(10.0), looking) + t.m.card_gap;
            }
            let tags: Vec<Option<String>> = others.iter().map(|(_, n)| fmt::band_tag(n.bands)).collect();
            let mut rows: Vec<Row> = others
                .iter()
                .zip(&tags)
                .map(|((_, n), tag)| {
                    let row = Row::new(&n.name).fan(fmt::signal_level(n.signal)).trail(Trail::Network {
                        locked: locked(n.security),
                    });
                    match tag {
                        Some(tag) => row.tag(tag),
                        None => row,
                    }
                })
                .collect();
            // The list ends with adding a network by hand, as the stock one
            // does.
            rows.push(Row::new(l.pick(ADD_NETWORK)).tint(t.pal.accent).trail(Trail::None));
            let list = widgets::list(&mut c, t, f, y, &rows);
            let mut picks: Vec<Action> = others.iter().map(|(k, _)| Action::JoinNetwork(*k)).collect();
            picks.push(Action::AddNetwork);
            out.extend(hits(&list.rows, &picks));
            // The round buttons come after the rows they sit on, and a touch
            // finds the last control under it.
            for (rect, (k, _)) in list.rows.iter().zip(&others) {
                out.push(Hit {
                    rect: widgets::round_button(t, *rect),
                    action: Action::NetworkDetails(*k),
                });
            }
            y = list.bottom;
        }
    }
    Frame::new(t, fit(c, t, y), out, l.pick(WLAN), top)
}

const REMEMBER_WLAN: Text = crate::lang::legacy(["记住网络", "記住網路", "Remember networks"]);

const ADD_NETWORK: Text = crate::texts::wlan::ADD_NETWORK;
const SHOW_PASSWORD: Text = crate::texts::wlan::SHOW_PASSWORD;
const CONNECT_KEY: Text = crate::texts::wlan::CONNECT;
const SECURITY: Text = crate::texts::wlan::SECURITY_TITLE;

/// The password of the network chosen: its name, what is typed, a switch
/// to show it, and the keyboard.
pub fn wlan_password(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let network = s.joining.as_ref().expect("the password page is for a network");
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, &network.name);
    let shown = if s.show_password {
        s.password.clone()
    } else {
        "•".repeat(s.password.chars().count())
    };
    let (y, caret) = widgets::text_field(&mut c, t, f, y, &shown);
    let show = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.card_gap,
        &[Row::new(l.pick(SHOW_PASSWORD)).switch(s.show_password)],
    );
    out.extend(hits(&show.rows, &[Action::Toggle(Switch::ShowPassword)]));
    keyboard_hits(&mut c, t, f, s, l.pick(CONNECT_KEY), &mut out);
    Frame {
        caret: Some(caret),
        ..Frame::new(t, c, out, l.pick(crate::texts::wlan::ENTER_PASSWORD), top)
    }
}

/// A network added by hand: its name, how it is protected, its password
/// where it takes one with the switch that shows it, and the keyboard,
/// which types into the field touched. The protections open over the page
/// from their row, which alone then answers a touch.
pub fn wlan_add(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let a = s.adding.as_ref().expect("the page adds a network");
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let field = |y: f32| Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0));
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::wlan::NETWORK_NAME));
    let (below, mut caret) = widgets::text_field(&mut c, t, f, y, &a.name);
    out.push(Hit {
        rect: field(y),
        action: Action::TypeInto(Field::Name),
    });
    // Held down while its list is open, as the stock row stays.
    let tone = if a.choosing { Tone::page(&t.pressed()) } else { Tone::page(t) };
    let protection = [Row::new(l.pick(SECURITY)).trail(Trail::Choice(fmt::security(l, a.security)))];
    let security = widgets::list_toned(&mut c, t, f, below + m.card_gap, &protection, &tone);
    out.extend(hits(&security.rows, &[Action::ChooseSecurity]));
    if a.security.password() {
        let y = widgets::section(&mut c, t, f, security.bottom, l.pick(crate::texts::wlan::PASSWORD));
        let shown = if s.show_password {
            s.password.clone()
        } else {
            "•".repeat(s.password.chars().count())
        };
        let (below, typed) = widgets::text_field(&mut c, t, f, y, &shown);
        out.push(Hit {
            rect: field(y),
            action: Action::TypeInto(Field::Password),
        });
        if a.field == Field::Password {
            caret = typed;
        }
        let show = widgets::list(
            &mut c,
            t,
            f,
            below + m.card_gap,
            &[Row::new(l.pick(SHOW_PASSWORD)).switch(s.show_password)],
        );
        out.extend(hits(&show.rows, &[Action::Toggle(Switch::ShowPassword)]));
    }
    keyboard_hits(&mut c, t, f, s, l.pick(CONNECT_KEY), &mut out);
    if !a.choosing {
        return Frame {
            caret: Some(caret),
            ..Frame::new(t, c, out, l.pick(ADD_NETWORK), top)
        };
    }
    // A touch anywhere off the list closes it.
    out.clear();
    out.push(Hit {
        rect: Rect::new(0.0, 0.0, m.w as f32, m.h as f32),
        action: Action::CloseChoices,
    });
    let names = ADDABLE.map(|p| fmt::security(l, p));
    let chosen = ADDABLE.iter().position(|&p| p == a.security);
    let held = ADDABLE.iter().position(|&p| s.held == Some(Action::PickSecurity(p)));
    let list = widgets::choices(&mut c, t, f, security.rows[0], &names, chosen, held);
    out.extend(hits(&list, &ADDABLE.map(Action::PickSecurity)));
    Frame::new(t, c, out, l.pick(ADD_NETWORK), top)
}

/// A network's details, in the stock page's grid: for the one chosen, how
/// joining it goes and what it gave, and forgetting it; for any other, how
/// strong and how protected it is.
pub fn wlan_network(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let network = s.details.as_ref().expect("the details page is of a network");
    let joined = s.wlan.as_ref().and_then(|w| w.joined.as_ref()).filter(|j| j.ssid == network.ssid);
    let y = widgets::section_first(&mut c, t, f, top, &network.name);
    let detail = |icon, title, value: String| Detail { icon, title, value };
    let security = detail(Outline::Shield, l.pick(SECURITY), fmt::security(l, network.security).to_string());
    let signal = l.pick(crate::texts::wlan::SIGNAL_STRENGTH);
    let title = l.pick(crate::texts::wlan::DETAILS);
    let Some(j) = joined else {
        let items = [
            detail(Outline::Wifi, l.pick(crate::texts::wlan::BANDS), fmt::bands(l, network.bands)),
            detail(Outline::Signal, signal, fmt::signal(l, network.signal).to_string()),
            security,
        ];
        let y = widgets::detail_grid(&mut c, t, f, y, &items);
        let y = wlan_forget(cx, &mut c, y, &mut out, network.remembered);
        return Frame::new(t, fit(c, t, y), out, title, top);
    };
    let mut items = vec![detail(
        Outline::Info,
        l.pick(["状态信息", "狀態資訊", "Status"]),
        fmt::joining(l, &j.stage).to_string(),
    )];
    match &j.stage {
        Joining::Connected(link) => {
            if let Some(n) = link.generation {
                items.push(detail(
                    Outline::Wifi,
                    l.pick(crate::texts::wlan::STANDARD),
                    fmt::generation(l, n),
                ));
            }
            if let Some(speed) = link.speed {
                items.push(detail(
                    Outline::Speed,
                    l.pick(crate::texts::wlan::LINK_SPEED),
                    format!("{speed} Mbps"),
                ));
            }
            items.push(detail(Outline::Signal, signal, fmt::signal(l, link.signal).to_string()));
            items.push(security);
            items.push(detail(
                Outline::Address,
                l.pick(crate::texts::wlan::IP_ADDRESS),
                link.address.to_string(),
            ));
            items.push(detail(
                Outline::Mask,
                l.pick(crate::texts::wlan::SUBNET_MASK),
                link.mask.to_string(),
            ));
            if let Some(router) = link.router {
                items.push(detail(Outline::Router, l.pick(crate::texts::wlan::ROUTER), router.to_string()));
            }
        }
        _ => {
            items.push(detail(Outline::Signal, signal, fmt::signal(l, network.signal).to_string()));
            items.push(security);
        }
    }
    let y = widgets::detail_grid(&mut c, t, f, y, &items);
    let forget = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.card_gap,
        &[Row::new(l.pick(crate::lang::legacy(["断开连接", "中斷連線", "Disconnect"]))).trail(Trail::None)],
    );
    out.extend(hits(&forget.rows, &[Action::LeaveNetwork]));
    let y = wlan_forget(cx, &mut c, forget.bottom, &mut out, network.remembered);
    Frame::new(t, fit(c, t, y), out, title, top)
}

fn wlan_forget(cx: &Ctx, canvas: &mut Canvas, y: f32, out: &mut Vec<Hit>, remembered: bool) -> f32 {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    if !remembered {
        return y;
    }
    let rows = widgets::list(
        canvas,
        t,
        f,
        y + t.m.card_gap,
        &[Row::new(l.pick(crate::lang::legacy(["忘记网络", "忘記網路", "Forget network"])))
            .tint(t.pal.danger)
            .trail(Trail::None)],
    );
    out.extend(hits(&rows.rows, &[Action::ForgetNetwork]));
    if !s.asking_delete {
        return rows.bottom;
    }
    let (label, danger) = confirm_label(Confirm::ForgetNetwork, s);
    let track = widgets::confirm(canvas, t, f, rows.bottom + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::ForgetNetwork),
    });
    track.y + track.h
}

/// The file manager, every entry of the folder shown that its search box
/// finds, folders first. The box and the button that changes the order are
/// held over the list, and the keyboard stands at the bottom while it is
/// open.
pub fn files(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    // An entry picked to copy or move goes to the folder shown, from the
    // bar held at the bottom; the keyboard, while it is up, holds that
    // place; with neither, nothing is held there.
    let dock = match (&s.clip, s.files_search) {
        (Some(clip), false) => {
            let go = l.pick(match clip.op {
                FileOp::Copy => crate::texts::files::COPY_HERE,
                FileOp::Move => crate::texts::files::MOVE_HERE,
            });
            let (canvas, rects) = widgets::dock_bar(t, f, files::page(t), &[l.pick(CANCEL), go]);
            Some(Dock {
                canvas,
                hits: hits(&rects, &[Action::Unpick, Action::Paste]),
            })
        }
        (_, true) => Some(keyboard_dock(cx, s)),
        _ => None,
    };
    let button = Some((widgets::Glyph::Terminal, Action::TerminalHere));
    Frame {
        dock,
        button,
        ..file_listing(cx)
    }
}

/// The file manager's keyboard, held at the bottom of the page, its keys
/// typing into the search box.
fn keyboard_dock(cx: &Ctx, s: &State) -> Dock {
    let (t, f, l) = (cx.t, cx.f, s.lang);
    let ime_h = crate::ime::height(t);
    let mut c = Canvas::new(t.m.w, (widgets::keyboard_height(t) + ime_h).round() as u32);
    c.clear(files::page(t));
    let enter = l.pick(FILES_SEARCH_ENTER);
    let keys = widgets::keyboard(&mut c, t, f, l, ime_h, s.layer, s.key_held(), enter);
    let mut hits: Vec<_> = keys
        .into_iter()
        .map(|(rect, cap)| Hit {
            rect,
            action: Action::Type(cap),
        })
        .collect();
    if cfg!(feature = "pinyin-input") {
        hits.extend(crate::ime::bar(&mut c, t, f, &s.ime, 0.0, false));
    }
    Dock { canvas: c, hits }
}

/// The folder the file manager shows, laid out with the search box and the
/// order button held over it.
fn file_listing(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let bg = files::page(t);
    let mut c = sheet(cx, t.m.h, bg);
    let rows = match &s.files.entries {
        Err(why) => {
            files::note(
                &mut c,
                t,
                f,
                files::SEARCHED_ROWS_TOP_DP,
                l.pick(crate::texts::files::UNREADABLE),
                Some(why),
            );
            None
        }
        Ok(_) if !s.files_query.trim().is_empty() => search_rows(cx, &mut c),
        Ok(entries) if entries.is_empty() => {
            files::note(
                &mut c,
                t,
                f,
                files::SEARCHED_ROWS_TOP_DP,
                l.pick(crate::texts::files::NONE),
                None,
            );
            None
        }
        Ok(_) => {
            let shown = folder_rows(&s.files);
            Some(files::rows(t, f, files::SEARCHED_ROWS_TOP_DP, &shown, Action::OpenEntry))
        }
    };
    let root = l.pick(crate::texts::files::ROOT);
    let hint = l.pick(FILES_SEARCH_HINT);
    let pinned = Some(
        files::head(t, f, &s.files_query, hint, sort_label(l, s.sort), root, &s.files.path).hits(
            Action::FilesSearch,
            Action::ClearFilesQuery,
            Action::CycleSort,
        ),
    );
    Frame {
        bg,
        rows,
        pinned,
        ..Frame::new(t, c, Vec::new(), l.pick(FILES), t.m.dp(files::STRIP_TOP_DP))
    }
}

/// The entries a folder's rows show, in order, as the rows take them.
fn folder_rows(folder: &Folder) -> Vec<&FileRow> {
    folder.shown_entries()
}

/// The rows of what the search box found in the folders below this one, or
/// in place of them why there is nothing to show yet: the search still
/// looking, or that it could not look at all.
fn search_rows(cx: &Ctx, c: &mut Canvas) -> Option<Rows> {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let typed = s.files_query.trim();
    let top = files::SEARCHED_ROWS_TOP_DP;
    match &s.found {
        None => {
            files::note(c, t, f, top, l.pick(crate::lang::legacy(["正在搜索…", "正在搜尋…", "Searching…"])), None);
            None
        }
        Some(Err(why)) => {
            files::note(
                c,
                t,
                f,
                top,
                l.pick(crate::lang::legacy(["无法搜索该文件夹", "無法搜尋這個資料夾", "This folder can't be searched"])),
                Some(why),
            );
            None
        }
        Some(Ok(found)) if found.is_empty() => {
            let none = tr!(
                l,
                "没有名称含“{typed}”的条目，子文件夹也搜过了",
                "沒有名稱含「{typed}」的項目，子資料夾也搜過了",
                "Nothing below here is named “{typed}”, subfolders included"
            );
            files::note(c, t, f, top, &none, None);
            None
        }
        Some(Ok(found)) => {
            let shown: Vec<&Found> = found.iter().collect();
            Some(files::found_rows(t, f, top, &shown, Action::OpenFound))
        }
    }
}

const FILES_SEARCH_HINT: Text = crate::lang::legacy(["搜索文件", "搜尋檔案", "Search files"]);
const FILES_SEARCH_ENTER: Text = crate::texts::common::DONE;

/// What the entries of a folder are in the order of, as the bar of them
/// names it. The order itself is `sort::sort`, which is where a `SortKey`
/// is turned into an order.
fn sort_label(l: Lang, key: SortKey) -> &'static str {
    l.pick(match key {
        SortKey::Name => crate::texts::files::NAME,
        SortKey::Modified => crate::lang::legacy(["时间", "時間", "Time"]),
        SortKey::Kind => crate::lang::legacy(["类型", "類型", "Type"]),
    })
}

const CANCEL: Text = crate::texts::common::CANCEL;

/// A place's folders and the files in them that can be installed, to
/// choose one. A USB drive's page waits for one to be plugged in, and the
/// drive's files show as soon as it is.
pub fn choose(cx: &Ctx, place: Place) -> Frame {
    let (s, l) = (cx.state, cx.state.lang);
    let unreadable;
    let absent = match (place, &s.usb_drive) {
        (Place::UsbDrive, None) => Some((
            l.pick(crate::texts::choose::NO_USB_DRIVE),
            l.pick(crate::texts::choose::CONNECT_USB_DRIVE),
        )),
        (Place::UsbDrive, Some(UsbDrive::Unreadable(why))) => {
            unreadable = tr!(
                l,
                "请使用 FAT32 或 exFAT 格式的 U 盘。{why}",
                "請使用 FAT32 或 exFAT 格式的 USB 隨身碟。{why}",
                "Use a USB drive formatted as FAT32 or exFAT. {why}"
            );
            Some((l.pick(USB_DRIVE_UNREADABLE), unreadable.as_str()))
        }
        _ => None,
    };
    let empty = l.pick(crate::texts::choose::NONE);
    listing(
        cx,
        s.folder(Screen::Choose(place)),
        place_name(l, place),
        absent,
        empty,
        Action::ChooseEntry,
        l.pick(crate::texts::choose::TITLE),
    )
}

/// A folder laid out as the stock file manager lays one out: the path as
/// capsules from `top`, held under the bar once scrolled there, then the
/// entries in one list on the plain page, each row drawn as it scrolls into
/// view and touched for `action`. In place of the list, `absent` says why
/// nothing of the folder can be shown, and `empty` that it holds nothing.
fn listing(
    cx: &Ctx,
    folder: &Folder,
    top: &str,
    absent: Option<(&str, &str)>,
    empty: &str,
    action: fn(usize) -> Action,
    title: &'static str,
) -> Frame {
    let (t, f) = (cx.t, cx.f);
    let bg = files::page(t);
    let mut c = sheet(cx, t.m.h, bg);
    let rows = match (absent, &folder.entries) {
        (Some((note, why)), _) => {
            files::note(&mut c, t, f, files::ROWS_TOP_DP, note, Some(why));
            None
        }
        (None, Err(why)) => {
            files::note(
                &mut c,
                t,
                f,
                files::ROWS_TOP_DP,
                cx.state
                    .lang
                    .pick(crate::texts::files::UNREADABLE),
                Some(why),
            );
            None
        }
        (None, Ok(entries)) if entries.is_empty() => {
            files::note(&mut c, t, f, files::ROWS_TOP_DP, empty, None);
            None
        }
        (None, Ok(_)) => {
            let shown: Vec<&FileRow> = folder.entries.as_ref().map(|e| e.iter().collect()).unwrap_or_default();
            Some(files::rows(t, f, files::ROWS_TOP_DP, &shown, action))
        }
    };
    let pinned = Some(files::strip(t, f, top, &folder.path));
    Frame {
        bg,
        rows,
        pinned,
        ..Frame::new(t, c, Vec::new(), title, t.m.dp(files::STRIP_TOP_DP))
    }
}

/// Row `i` of the page's long list over the whole of `c`, sized for it,
/// drawn as the row comes into view.
pub fn row(screen: Screen, cx: &Ctx, i: usize, c: &mut Canvas) {
    match screen {
        Screen::Browser => crate::browser::row(cx, i, c),
        // What the search box found comes from anywhere below the folder on
        // the page, so its rows are not the folder's rows.
        Screen::Files if !cx.state.files_query.trim().is_empty() => {
            let Some(list) = cx.state.found.as_ref().and_then(|f| f.as_ref().ok()) else {
                return;
            };
            let Some(found) = list.get(i) else { return };
            files::found_row(
                c,
                cx.t,
                cx.f,
                cx.state.lang,
                found,
                &cx.state.files.path,
                cx.state.files.listed,
                cx.state.zone(),
            );
        }
        Screen::Files | Screen::Choose(_) => {
            let folder = cx.state.folder(screen);
            let Some(e) = folder.entry(i) else { return };
            files::row(c, cx.t, cx.f, cx.state.lang, e, folder.listed, cx.state.zone());
        }
        Screen::Installing => widgets::output_row(c, cx.t, cx.f, &cx.state.script_output, i),
        Screen::PartWork => {
            let work = cx.state.part_work.as_ref().expect("the page of partition work shows work");
            widgets::output_row(c, cx.t, cx.f, &work.output, i)
        }
        other => unreachable!("{other:?} draws no list row by row"),
    }
}

/// One entry of a folder, a file or a folder: what it is, and what can be
/// done with it. Deleting asks first, with a swipe.
pub fn file_info(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = s.file.as_ref().expect("the file page shows an entry");
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let folder = matches!(e.kind, FileKind::Folder { .. });
    let mut facts = vec![
        (l.pick(NAME), e.name.clone()),
        (
            l.pick(["位置", "位置", "Location"]),
            Path::new(&s.files.path).join(&e.name).to_string_lossy().into_owned(),
        ),
    ];
    let holds = l.pick(crate::texts::files::HOLDS);
    match &e.kind {
        FileKind::Link { target, .. } => facts.push((l.pick(crate::texts::files::POINTS_TO), target.clone())),
        FileKind::Folder { items: Some(n) } => {
            let items = match (l, n) {
                (Lang::En, 1) => "1 item".to_string(),
                (_, n) => tr!(l, "{n} 项", "{n} 項", "{n} items"),
            };
            facts.push((holds, items))
        }
        FileKind::Folder { items: None } => facts.push((holds, l.pick(UNREADABLE).to_string())),
        FileKind::File | FileKind::Special => {
            let (size, bytes) = (fmt::file_size(e.size), e.size);
            facts.push((
                l.pick(crate::texts::common::SIZE),
                tr!(l, "{size}（{bytes} 字节）", "{size}（{bytes} 位元組）", "{size} ({bytes} bytes)"),
            ))
        }
    }
    facts.push((
        l.pick(crate::texts::files::MODIFIED),
        fmt::datetime(l, e.modified, s.zone()),
    ));
    let (mode, uid, gid) = (fmt::mode(e.mode), e.uid, e.gid);
    facts.push((
        l.pick(PERMISSIONS),
        tr!(
            l,
            "{mode}，所有者 {uid}，用户组 {gid}",
            "{mode}，擁有者 {uid}，群組 {gid}",
            "{mode}, owner {uid}, group {gid}"
        ),
    ));
    let y = widgets::info_card(&mut c, t, f, top, &facts);
    let mut rows = Vec::new();
    let mut acts = Vec::new();
    // Images are flashed; packages and zips that install themselves are
    // installed.
    if installs(e) {
        let image = Path::new(&e.name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("img"));
        rows.push(Row::new(l.pick(if image {
            crate::texts::files::FLASH
        } else {
            crate::texts::page::INSTALL
        })));
        acts.push(Action::InstallEntry);
    }
    if e.kind == FileKind::File {
        rows.push(Row::new(l.pick(crate::texts::files::EDIT)));
        acts.push(Action::EditEntry);
    }
    rows.extend([
        Row::new(l.pick(crate::texts::files::COPY)),
        Row::new(l.pick(crate::texts::files::MOVE_TO)),
        Row::new(l.pick(RENAME)),
    ]);
    acts.extend([
        Action::PickEntry(FileOp::Copy),
        Action::PickEntry(FileOp::Move),
        Action::RenameEntry,
    ]);
    // A link's own permissions mean nothing; its target is changed where
    // it is.
    if !matches!(e.kind, FileKind::Link { .. }) {
        rows.push(Row::new(l.pick(CHANGE_PERMISSIONS)));
        acts.push(Action::OpenPermissions);
    }
    rows.push(Row::new(l.pick(crate::texts::keyboard::DELETE)).tint(t.pal.danger).trail(Trail::None));
    acts.push(Action::AskDelete);
    let actions = widgets::list(&mut c, t, f, y + t.m.card_gap, &rows);
    let mut out = hits(&actions.rows, &acts);
    let mut bottom = actions.bottom;
    if s.asking_delete {
        let name = &e.name;
        let what = match folder {
            true => tr!(
                l,
                "删除\"{name}\"及其中的全部内容？删除后无法恢复。",
                "刪除「{name}」及其中的全部內容？刪除後無法復原。",
                "Delete “{name}” and everything in it? It can't be brought back."
            ),
            false => tr!(
                l,
                "删除\"{name}\"？删除后无法恢复。",
                "刪除「{name}」？刪除後無法復原。",
                "Delete “{name}”? It can't be brought back."
            ),
        };
        let y = widgets::footnote(&mut c, t, f, actions.bottom, &what);
        let (label, danger) = confirm_label(Confirm::DeleteEntry, cx.state);
        let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
        out.push(Hit {
            rect: track,
            action: Action::Confirm(Confirm::DeleteEntry),
        });
        bottom = track.y + track.h;
    }
    let title = l.pick(if folder {
        crate::texts::files::FOLDER_INFO
    } else {
        crate::texts::files::FILE_INFO
    });
    Frame::new(t, fit(c, t, bottom), out, title, top)
}

const NAME: Text = crate::texts::files::NAME;
const PERMISSIONS: Text = crate::texts::files::PERMISSIONS;
const CHANGE_PERMISSIONS: Text = crate::texts::files::CHANGE_PERMISSIONS;
const RENAME: Text = crate::texts::files::RENAME;

/// The permission bits of the entry the file page shows, a switch each that
/// takes effect at once: reading, writing and running for its owner, its
/// group and everyone else, then the set-ID and sticky bits.
pub fn permissions(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = s.file.as_ref().expect("the permissions page shows an entry");
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mode = e.mode & 0o7777;
    let bits = fmt::mode(e.mode);
    let facts = [
        (l.pick(NAME), e.name.clone()),
        (
            l.pick(PERMISSIONS),
            tr!(l, "{bits}，{mode:04o}", "{bits}，{mode:04o}", "{bits}, {mode:04o}"),
        ),
    ];
    let mut y = widgets::info_card(&mut c, t, f, top, &facts);
    let mut out = Vec::new();
    let classes = [
        (crate::texts::files::mode::OWNER, 6),
        (crate::texts::files::mode::GROUP, 3),
        (crate::texts::files::mode::OTHERS, 0),
    ];
    let (read, write, run) = (
        l.pick(crate::texts::files::mode::READ),
        l.pick(crate::texts::files::mode::WRITE),
        l.pick(crate::texts::files::mode::EXECUTE),
    );
    let groups = classes.map(|(title, shift)| (l.pick(title), [(read, 4 << shift), (write, 2 << shift), (run, 1 << shift)]));
    let special = (
        l.pick(crate::texts::files::mode::SPECIAL),
        [("SUID", 0o4000), ("SGID", 0o2000), (l.pick(crate::texts::files::mode::STICKY), 0o1000)],
    );
    for (title, bits) in groups.into_iter().chain([special]) {
        y = widgets::section(&mut c, t, f, y, title);
        let rows = bits.map(|(name, bit)| Row::new(name).switch(mode & bit != 0));
        let list = widgets::list(&mut c, t, f, y, &rows);
        out.extend(hits(&list.rows, &bits.map(|(_, bit)| Action::ModeBit(bit))));
        y = list.bottom;
    }
    Frame::new(t, fit(c, t, y), out, l.pick(CHANGE_PERMISSIONS), top)
}

/// A new name for the entry the file page shows, typed on the keyboard.
pub fn rename(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let (y, caret) = widgets::text_field(&mut c, t, f, top, &s.new_name);
    widgets::footnote(
        &mut c,
        t,
        f,
        y,
        l.pick(crate::texts::files::RENAME_NOTE),
    );
    keyboard_hits(&mut c, t, f, s, l.pick(DONE), &mut out);
    Frame {
        caret: Some(caret),
        ..Frame::new(t, c, out, l.pick(RENAME), top)
    }
}

/// The folder an entry goes to already holds its name: what to do.
pub fn clash(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let (clip, into, _) = s.clash.as_ref().expect("the clash page shows a clash");
    let name = Path::new(&clip.path)
        .file_name()
        .map_or_else(|| clip.path.clone(), |n| n.to_string_lossy().into_owned());
    let verb = l.pick(match clip.op {
        FileOp::Copy => crate::lang::legacy(["复制", "複製", "copy"]),
        FileOp::Move => crate::lang::legacy(["移动", "移動", "move"]),
    });
    let mut c = page(cx);
    let top = t.m.content_top;
    let there = tr!(
        l,
        "\"{into}\"中已有同名的\"{name}\"。",
        "「{into}」中已有同名的「{name}」。",
        "“{into}” already holds an entry named “{name}”."
    );
    let y = widgets::paragraph(&mut c, t, f, top, &there);
    let skip = tr!(
        l,
        "保留已有的同名项，不{verb}该项",
        "保留已有的同名項目，不{verb}這個項目",
        "Keeps the entry there, and doesn't {verb} this one"
    );
    let choices = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.dp(12.0),
        &[
            Row::new(l.pick(crate::texts::files::clash::REPLACE)).subtitle(l.pick(crate::texts::files::clash::REPLACE_NOTE)),
            Row::new(l.pick(crate::texts::files::clash::SKIP)).subtitle(&skip),
            Row::new(l.pick(crate::texts::files::clash::KEEP_BOTH)).subtitle(l.pick(crate::texts::files::clash::KEEP_BOTH_NOTE)),
        ],
    );
    let out = hits(
        &choices.rows,
        &[
            Action::Resolve(ClashChoice::Replace),
            Action::Resolve(ClashChoice::Skip),
            Action::Resolve(ClashChoice::KeepBoth),
        ],
    );
    Frame::new(t, c, out, l.pick(crate::texts::files::clash::TITLE), top)
}

/// How work on files is told of: the Chinese verb its sentences build on,
/// in simplified and in traditional characters, then English's own words
/// for what is done and for doing it; the page's titles while it goes on
/// and once it ended short; and what stopping it may have left.
struct FileWords {
    verb: [&'static str; 2],
    done: &'static str,
    base: &'static str,
    titles: (Text, Text),
    stopped: Text,
}

fn file_words(task: &FileTask) -> FileWords {
    match task {
        FileTask::Copy { .. } => FileWords {
            verb: ["复制", "複製"],
            done: "copied",
            base: "copy",
            titles: (crate::texts::files::job::COPYING, crate::texts::files::job::COPY_INCOMPLETE),
            stopped: crate::texts::files::job::COPY_STOPPED,
        },
        FileTask::Move { .. } => FileWords {
            verb: ["移动", "移動"],
            done: "moved",
            base: "move",
            titles: (crate::texts::files::job::MOVING, crate::texts::files::job::MOVE_INCOMPLETE),
            stopped: crate::texts::files::job::MOVE_STOPPED,
        },
        FileTask::Delete { .. } => FileWords {
            verb: ["删除", "刪除"],
            done: "deleted",
            base: "delete",
            titles: (
                crate::texts::files::job::DELETING,
                crate::texts::files::job::DELETE_INCOMPLETE,
            ),
            stopped: crate::texts::files::job::DELETE_STOPPED,
        },
    }
}

/// Copying, moving or deleting: how far it has come and a way to stop it,
/// then, if it did not finish, why.
pub fn file_job(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let job = s.job.as_ref().expect("the job page shows work on files");
    let path = match &job.task {
        FileTask::Copy { path, .. } | FileTask::Move { path, .. } | FileTask::Delete { path } => path,
    };
    let words = file_words(&job.task);
    let ([hans, hant], done_en, base) = (words.verb, words.done, words.base);
    let name = Path::new(path)
        .file_name()
        .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
    let share = |(done, total): (u64, u64)| if total == 0 { 0.0 } else { done as f32 / total as f32 };
    // The bar stays while the work measures, so that the button under it
    // does not move. Deleting, and copying entries that hold no bytes, is
    // counted in entries.
    let (detail, done) = match (job.progress, &job.task) {
        (None, _) => (l.pick(GETTING_READY).to_string(), 0.0),
        (Some((bytes, _)), FileTask::Copy { .. } | FileTask::Move { .. }) if bytes.1 > 0 => {
            let (did, total) = (fmt::file_size(bytes.0), fmt::file_size(bytes.1));
            (
                tr!(
                    l,
                    "已{hans} {did}，共 {total}",
                    "已{hant} {did}，共 {total}",
                    "{did} of {total} {done_en}"
                ),
                share(bytes),
            )
        }
        (Some((_, entries)), _) => {
            let (did, total) = entries;
            (
                tr!(
                    l,
                    "已{hans} {did} 项，共 {total} 项",
                    "已{hant} {did} 項，共 {total} 項",
                    "{did} of {total} items {done_en}"
                ),
                share(entries),
            )
        }
    };
    let mut c = page(cx);
    let top = t.m.content_top;
    let y = widgets::progress_card(&mut c, t, f, top, &name, &detail, Some(done));
    let mut out = Vec::new();
    match &job.outcome {
        None => {
            let label = l.pick(if job.stopping { STOPPING } else { STOP });
            let stop = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(label).trail(Trail::None)]);
            if !job.stopping {
                out.extend(hits(&stop.rows, &[Action::StopJob]));
            }
        }
        Some(end) => {
            let why = match end {
                JobEnd::Stopped => l.pick(words.stopped).to_string(),
                JobEnd::Failed(why) => tr!(l, "{hans}失败：{why}", "{hant}失敗：{why}", "Couldn't {base} it: {why}"),
                JobEnd::Done => unreachable!("work done leaves its page"),
            };
            let y = widgets::footnote(&mut c, t, f, y, &why);
            let finish = widgets::list(&mut c, t, f, y + t.m.card_gap, &[Row::new(l.pick(DONE)).trail(Trail::None)]);
            out.extend(hits(&finish.rows, &[Action::Back]));
        }
    }
    let title = l.pick(if job.outcome.is_none() { words.titles.0 } else { words.titles.1 });
    Frame {
        back: job.outcome.is_some(),
        ..Frame::new(t, c, out, title, top)
    }
}

/// A shell on a terminal: the screen it writes, the keys a phone keyboard
/// lacks, and the keyboard, every key of which goes to the shell.
pub fn terminal(cx: &Ctx) -> Frame {
    let (t, f, s) = (cx.t, cx.f, cx.state);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    keyboard_hits(&mut c, t, f, s, s.lang.pick(crate::texts::keyboard::ENTER), &mut out);
    let l = term::layout(t, f, top);
    term::draw(&mut c, t, f, &l, cx.term);
    let held = cx.term.is_some_and(|term| term.ctrl);
    let keys = term::keys(&mut c, t, f, &l, &term::TERM_KEYS, held);
    out.extend(keys.into_iter().map(|(rect, key)| Hit {
        rect,
        action: Action::TermKey(key),
    }));
    Frame {
        scrollback: Some((l.card, l.pitch)),
        ..Frame::new(t, c, out, s.lang.pick(TERMINAL), top)
    }
}

/// A text file being edited: where it is, its lines wrapped on a card, the
/// keys a phone keyboard lacks for moving the cursor, and the keyboard.
/// Once it is changed, the round button saves it.
pub fn editor(cx: &Ctx) -> Frame {
    let (t, f, s) = (cx.t, cx.f, cx.state);
    let ed = s.editor.as_ref().expect("the editor page shows a file");
    let mut c = page(cx);
    let mut out = Vec::new();
    keyboard_hits(&mut c, t, f, s, s.lang.pick(crate::texts::keyboard::NEW_LINE), &mut out);
    editor::caption(&mut c, t, f, &ed.path);
    let l = editor::layout(t, f);
    let caret = editor::draw(&mut c, t, f, &l, ed);
    let keys = term::keys(&mut c, t, f, &l, &editor::EDITOR_KEYS, false);
    out.extend(keys.into_iter().map(|(rect, key)| Hit {
        rect,
        action: Action::TermKey(key),
    }));
    let button = ed.changed.then_some((widgets::Glyph::Done, Action::SaveText));
    Frame {
        scrollback: Some((l.card, l.pitch)),
        caret,
        button,
        ..Frame::new(t, c, out, s.lang.pick(crate::texts::files::EDIT), t.m.content_top)
    }
}

/// The languages the pages speak, each named in itself, to speak another.
pub fn language(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let rows: Vec<Row> = Lang::ALL.iter().map(|&k| Row::new(k.name()).trail(Trail::Check(k == l))).collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let out = hits(&list.rows, &Lang::ALL.map(Action::Language));
    Frame::new(t, c, out, l.pick(LANGUAGE), top)
}

/// The time zone the status bar's clock and the times on the pages keep.
/// On a phone whose installed system's zone is known, a switch follows it
/// and tells which it is; a phone with no system, or one that has set no
/// zone, shows none. Under it a zone is chosen from a part of the world,
/// whose row names the zone kept while that is one of its own.
pub fn time_zone(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = match &s.system_zone {
        Some(zone) => {
            let name = listed(&zone.name).map_or(zone.name.as_str(), |at| l.pick(TIME_ZONES[at].name));
            let system = format!("{} · {name}", fmt::utc_offset(s.now, &zone.rules));
            let follow = Row::new(l.pick(texts::time_zone::FOLLOW_SYSTEM)).subtitle(&system).trail(Trail::Switch(s.follow_system_zone));
            let follow = widgets::list(&mut c, t, f, top, &[follow]);
            out.extend(hits(&follow.rows, &[Action::Toggle(Switch::SystemTimeZone)]));
            widgets::section(&mut c, t, f, follow.bottom, l.pick(texts::time_zone::CHOOSE))
        }
        None => top,
    };
    let kept = s.zone_listed();
    let rows = Region::ALL.map(|region| {
        let row = Row::new(l.pick(region.name()));
        match kept.filter(|&at| TIME_ZONES[at].region == Some(region)) {
            Some(at) => row.trail(Trail::Value(l.pick(TIME_ZONES[at].name))),
            None => row,
        }
    });
    let regions = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&regions.rows, &Region::ALL.map(|region| Action::Open(Screen::TimeZones(region)))));
    let utc = listed(UTC).expect("UTC is a zone listed");
    let offset = fmt::utc_offset(s.now, fmt::rules(utc));
    let universal = Row::new(l.pick(TIME_ZONES[utc].name)).subtitle(&offset).trail(Trail::Check(kept == Some(utc)));
    let universal = widgets::list(&mut c, t, f, regions.bottom + t.m.card_gap, &[universal]);
    out.extend(hits(&universal.rows, &[Action::TimeZone(utc)]));
    let bottom = widgets::footnote(&mut c, t, f, universal.bottom, l.pick(texts::time_zone::NOTE));
    Frame::new(t, fit(c, t, bottom), out, l.pick(texts::page::TIME_ZONE), top)
}

/// Saving the logs, as a bug report asks for them.
pub fn logs(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::logs::CONTENTS));
    let parts = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(crate::texts::logs::RECOVERY_LOG))
                .subtitle(l.pick(crate::texts::logs::RECOVERY_LOG_NOTE))
                .trail(Trail::Check(s.logs[0])),
            Row::new(l.pick(crate::texts::logs::KERNEL_LOG))
                .subtitle(l.pick(crate::texts::logs::KERNEL_LOG_NOTE))
                .trail(Trail::Check(s.logs[1])),
        ],
    );
    out.extend(hits(&parts.rows, &[Action::LogPart(0), Action::LogPart(1)]));
    let y = widgets::section(&mut c, t, f, parts.bottom, l.pick(crate::texts::logs::SAVE_TO));
    let storage = l.pick(match cx.state.data.storage_open() {
        true => crate::texts::logs::FOLDER,
        false => DECRYPT_NEEDED,
    });
    let places = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[
            Row::new(l.pick(crate::texts::logs::TEMPORARY))
                .subtitle(l.pick(crate::texts::logs::TEMPORARY_NOTE))
                .trail(Trail::Check(s.logs_to == 0)),
            Row::new(place_name(l, Place::Storage))
                .subtitle(storage)
                .trail(Trail::Check(s.logs_to == 1)),
        ],
    );
    out.extend(hits(&places.rows, &[Action::LogsTo(0), Action::LogsTo(1)]));
    let note = l.pick(crate::texts::logs::NOTE);
    let y = widgets::footnote(&mut c, t, f, places.bottom, note);
    let export = widgets::button(&mut c, t, f, y + t.m.dp(24.0), l.pick(crate::texts::logs::EXPORT));
    out.push(Hit {
        rect: export,
        action: Action::ExportLogs,
    });
    Frame::new(t, c, out, l.pick(LOGS), top)
}

/// Skyward's licence, in Chinese, ahead of the words the licence gives for
/// a program's notice; English reads those words alone.
const LICENSE_NOTICE: [&str; 2] = [
    "Skyward 是自由软件：你可以依据自由软件基金会发布的 GNU 通用公共许可证第 3 版，或（由你选择）任何更新的版本，重新发布和修改它。\n\n\
发布 Skyward 是希望它有用，但不附带任何担保，也不担保其适销性或适用于特定用途。详见 GNU 通用公共许可证。",
    "Skyward 是自由軟體：你可以依據自由軟體基金會發布的 GNU 通用公眾授權條款第 3 版，或（由你選擇）任何更新的版本，重新發布和修改它。\n\n\
發布 Skyward 是希望它有用，但不附帶任何擔保，也不擔保其適銷性或適用於特定用途。詳見 GNU 通用公眾授權條款。",
];

const LICENSE_ORIGINAL: &str = "Skyward\nCopyright (C) 2026 Kirk Lin\n\n\
This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.\n\n\
This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.\n\n\
You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.";

const LICENSE_TERMS: Text = crate::texts::license::TERMS_TITLE;

/// Skyward's own licence.
pub fn license(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let terms = l.pick(crate::texts::license::TERMS);
    let y = widgets::info_card(&mut c, t, f, top, &[(l.pick(LICENSE_TERMS), terms.to_string())]);
    let bottom = match l {
        Lang::Hans | Lang::Hant => {
            let notice = if l == Lang::Hans { LICENSE_NOTICE[0] } else { LICENSE_NOTICE[1] };
            let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), notice);
            let y = widgets::section(&mut c, t, f, y, "原文");
            widgets::paragraph(&mut c, t, f, y - t.m.dp(16.0), LICENSE_ORIGINAL)
        }
        _ => widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), LICENSE_ORIGINAL),
    };
    Frame::new(t, fit(c, t, bottom), Vec::new(), l.pick(LICENSE), top)
}

/// The monospaced typeface's licence, which names the copyright holders on
/// its first lines; the image carries the same file beside the font.
const MONO_LICENSE: &str = include_str!("../../../resources/fonts/KirkMono-OFL.txt");

/// The typefaces, as they name themselves, and the terms each is used under.
pub fn font_license(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let (typeface, version, copyright) = (
        l.pick(crate::texts::fonts::TYPEFACE),
        l.pick(crate::texts::fonts::VERSION),
        l.pick(crate::texts::fonts::COPYRIGHT),
    );
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::fonts::INTERFACE_TEXT));
    let y = widgets::info_card(
        &mut c,
        t,
        f,
        y,
        &[
            (typeface, f.info.family.clone()),
            (version, f.info.version.clone()),
            (copyright, f.info.copyright.clone().expect("MiSans names its copyright holder")),
        ],
    );
    let misans = l.pick(crate::texts::fonts::MISANS);
    let y = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), misans);
    let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::fonts::TERMINAL_TEXT));
    let holders: Vec<&str> = MONO_LICENSE.lines().take_while(|l| l.starts_with("Copyright")).collect();
    let y = widgets::info_card(
        &mut c,
        t,
        f,
        y,
        &[
            (typeface, f.mono_info.family.clone()),
            (version, f.mono_info.version.clone()),
            (copyright, holders.join("\n")),
            (
                l.pick(LICENSE_TERMS),
                l.pick(crate::texts::fonts::OFL)
                .to_string(),
            ),
        ],
    );
    let mono = l.pick(crate::texts::fonts::MONO);
    let bottom = widgets::paragraph(&mut c, t, f, y + t.m.dp(4.0), mono);
    Frame::new(t, fit(c, t, bottom), Vec::new(), l.pick(FONTS), top)
}

const BOOT_SLOT: Text = crate::texts::slot::BOOT_SLOT;

/// Which slot the phone starts from next, as the boot control holds it,
/// and what it holds of each; picking the other and swiping switches.
/// While an update is in flight the slot stays as the update set it.
pub fn slot(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(BOOT_SLOT);
    let Some(current) = i.slot else {
        widgets::list(
            &mut c,
            t,
            f,
            top,
            &[Row::new(l.pick(crate::texts::slot::NO_SLOTS)).trail(Trail::None)],
        );
        return Frame::new(t, c, out, title, top);
    };
    let slots = match &s.boot_slots {
        Some(Ok(slots)) => *slots,
        Some(Err(why)) => {
            let list = widgets::list(
                &mut c,
                t,
                f,
                top,
                &[Row::new(l.pick(crate::texts::slot::UNREADABLE)).trail(Trail::None)],
            );
            widgets::footnote(&mut c, t, f, list.bottom, why);
            return Frame::new(t, c, out, title, top);
        }
        None => return Frame::new(t, c, out, title, top),
    };
    let chosen = s.next_slot.unwrap_or(slots.active);
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::reboot::NEXT_BOOT));
    let titles = [Slot::A, Slot::B].map(|slot| {
        let slot = slot.letter();
        tr!(l, "槽位 {slot}", "槽位 {slot}", "Slot {slot}")
    });
    let subs = [Slot::A, Slot::B].map(|slot| {
        let k = slot.number() as usize;
        let which = l.pick(if slot == current {
            crate::texts::slot::RUNNING
        } else {
            crate::texts::slot::OTHER
        });
        match (slots.bootable[k], slots.successful[k]) {
            (false, _) => tr!(
                l,
                "{which}，已标记为无法启动",
                "{which}，已標記為無法啟動",
                "{which}, marked unbootable"
            ),
            (true, false) => tr!(
                l,
                "{which}，尚未完整启动过",
                "{which}，尚未完整啟動過",
                "{which}, not yet fully started"
            ),
            (true, true) => which.to_string(),
        }
    });
    let rows: Vec<Row> = [Slot::A, Slot::B]
        .iter()
        .enumerate()
        .map(|(k, &slot)| Row::new(&titles[k]).subtitle(&subs[k]).trail(Trail::Check(slot == chosen)))
        .collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&list.rows, &[Slot::A, Slot::B].map(Action::PickSlot)));
    if let Some(note) = fmt::snapshot_slot_note(l, &i.snapshot) {
        widgets::footnote(&mut c, t, f, list.bottom, note);
        return Frame::new(t, c, out, title, top);
    }
    let note = l.pick(crate::texts::slot::SWITCH_NOTE);
    let y = widgets::footnote(&mut c, t, f, list.bottom, note);
    let (label, danger) = confirm_label(Confirm::Slot, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::Slot),
    });
    Frame::new(t, c, out, title, top)
}

const MAIN_SYSTEM: Text = crate::texts::systems::MAIN;
const SECOND_SYSTEM: Text = crate::texts::systems::SECOND;

/// What the home page and the systems page say once the second system's
/// start stopped.
const SECOND_FAILED: Text = crate::texts::systems::SECOND_FAILED;

/// The two systems as big cards one over the other, laid out as the boot
/// slot page lays out its slots: the main system, in the phone's own
/// partitions, and the second, whose files lie in the data partition, or
/// the place to install one, the one that starts next checked. Picking the
/// other and swiping writes its boot images into the slot that starts
/// next, and restarts into it. Under them, the second system, and removing
/// it.
pub fn systems(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let title = l.pick(DUAL_BOOT);
    // While the boot partitions are written, or the second system removed,
    // the page only says so: of a switch the system asked for, perhaps
    // before the systems are read.
    if let Some(work) = s.systems_work {
        let (heading, detail) = match work {
            SystemsWork::Switching(to) => {
                let name = match (&s.systems, to) {
                    (Some(Ok(systems)), Who::Installed) => Some(systems.installed.name.as_str()),
                    (Some(Ok(systems)), Who::Other) => systems.other.as_ref().map(|o| o.name.as_str()),
                    _ => None,
                };
                let heading = match name {
                    Some(name) => tr!(l, "正在切换到 {name}", "正在切換到 {name}", "Switching to {name}"),
                    None => l.pick(crate::texts::systems::SWITCHING).to_string(),
                };
                (
                    heading,
                    l.pick(crate::texts::systems::WRITING_BOOT_IMAGES),
                )
            }
            SystemsWork::Removing => (
                l.pick(crate::texts::systems::REMOVING)
                    .to_string(),
                l.pick(crate::texts::systems::GIVING_ROOM_BACK),
            ),
        };
        let y = widgets::progress_card(&mut c, t, f, top, &heading, detail, None);
        let y = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(crate::texts::systems::KEEP_POWER),
        );
        return Frame::new(t, fit(c, t, y), out, title, top);
    }
    let systems = match &s.systems {
        Some(Ok(systems)) => systems,
        Some(Err(why)) => {
            let list = widgets::list(
                &mut c,
                t,
                f,
                top,
                &[Row::new(l.pick(crate::texts::systems::UNREADABLE)).trail(Trail::None)],
            );
            let y = widgets::footnote(&mut c, t, f, list.bottom, why);
            return Frame::new(t, fit(c, t, y), out, title, top);
        }
        None => return Frame::new(t, fit(c, t, top), out, title, top),
    };
    let starts = starting(systems);
    let chosen = s.system_pick.unwrap_or(starts);
    let second_role = l.pick(SECOND_SYSTEM);
    let main = SystemCard {
        role: l.pick(MAIN_SYSTEM),
        name: &systems.installed.name,
        detail: systems.installed.version.as_deref().unwrap_or_default(),
        mark: SystemMark::Check(chosen == Who::Installed && systems.held != Held::Unrecorded),
    };
    let second = match &systems.other {
        Some(other) => SystemCard {
            role: second_role,
            name: &other.name,
            detail: &other.version,
            mark: SystemMark::Check(chosen == Who::Other),
        },
        None => SystemCard {
            role: second_role,
            name: l.pick(crate::texts::systems::INSTALL),
            detail: l.pick(crate::texts::systems::CHOOSE_PACKAGE),
            mark: SystemMark::Opens,
        },
    };
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::reboot::NEXT_BOOT));
    let shown = widgets::system_cards(&mut c, t, f, y, &[main, second]);
    let second_opens = match systems.other {
        Some(_) => Action::PickSystem(Who::Other),
        None => Action::Open(Screen::Install),
    };
    out.extend(hits(&shown.rows, &[Action::PickSystem(Who::Installed), second_opens]));
    let mut y = shown.bottom + t.m.dp(12.0);
    if let Some(why) = &s.system_notice {
        y = widgets::info_card(&mut c, t, f, y, &[(l.pick(SECOND_FAILED), why.clone())]);
    }
    if chosen != starts {
        match (chosen, fmt::snapshot_system_note(l, &i.snapshot)) {
            (Who::Other, Some(note)) => y = widgets::footnote(&mut c, t, f, y, note),
            _ => {
                let (label, danger) = confirm_label(Confirm::SwitchSystem, s);
                let track = widgets::confirm(&mut c, t, f, y + t.m.dp(12.0), label, danger, s.confirm);
                out.push(Hit {
                    rect: track,
                    action: Action::Confirm(Confirm::SwitchSystem),
                });
                y = track.y + track.h;
            }
        }
    }
    let what = l.pick(crate::texts::systems::WHAT);
    y = widgets::footnote(&mut c, t, f, y, what);
    match systems.held {
        Held::Recorded => {}
        Held::Rewritten => {
            let note = l.pick(crate::lang::legacy([
                "第二系统的启动分区被外部改写过。选择主系统并滑动确认，才会用保存的启动镜像覆盖当前内容；此前拒绝安装、清除或备份。",
                "第二系統的啟動分割區被外部改寫過。選擇主系統並滑動確認，才會用儲存的啟動映像檔覆寫目前內容；此前拒絕安裝、清除或備份。",
                "The second system's boot partitions were rewritten externally. Select the main system and slide to confirm before saved boot images may replace them. Installation, wiping and backup are refused until then.",
            ]));
            y = widgets::footnote(&mut c, t, f, y, note);
        }
        Held::Unrecorded => {
            let note = l.pick(crate::lang::legacy([
                "第二系统的启动镜像仍在，但保存的记录已丢失。无法恢复主系统启动镜像，请重新刷入主系统的 boot 和 init_boot。",
                "第二系統的啟動映像檔仍在，但儲存記錄已遺失。無法還原主系統啟動映像檔，請重新刷入主系統的 boot 和 init_boot。",
                "Second-system boot images remain, but their records are gone. Flash the main system's boot and init_boot images again.",
            ]));
            y = widgets::footnote(&mut c, t, f, y, note);
        }
    }
    if systems.other.is_some() && systems.changed {
        let note = l.pick(crate::texts::systems::CHANGED);
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let Some(other) = &systems.other {
        y = widgets::section(&mut c, t, f, y, second_role);
        let size = fmt::bytes(other.size);
        let facts = widgets::list(
            &mut c,
            t,
            f,
            y,
            &[Row::new(l.pick(crate::texts::systems::SPACE_USED)).trail(Trail::Text(&size))],
        );
        let mounted = matches!(s.data, Data::Open | Data::Locked { .. });
        let remove = Row::new(l.pick(crate::texts::systems::REMOVE)).tint(t.pal.danger);
        let (remove, act) = match mounted {
            true => (remove.trail(Trail::None), Action::AskDelete),
            false => (remove.subtitle(l.pick(DECRYPT_NEEDED)), Action::Open(Screen::Decrypt)),
        };
        let removing = widgets::list(&mut c, t, f, facts.bottom + t.m.card_gap, &[remove]);
        out.extend(hits(&removing.rows, &[act]));
        y = removing.bottom;
        if s.asking_delete && mounted {
            let ask = match starts {
                Who::Other => l.pick(crate::texts::systems::ASK_REMOVE_RUNNING),
                Who::Installed => l.pick(crate::texts::systems::ASK_REMOVE),
            };
            let y2 = widgets::footnote(&mut c, t, f, y, ask);
            let (label, danger) = confirm_label(Confirm::RemoveSystem, s);
            let track = widgets::confirm(&mut c, t, f, y2 + t.m.dp(24.0), label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(Confirm::RemoveSystem),
            });
            y = track.y + track.h;
        }
    }
    Frame::new(t, fit(c, t, y), out, title, top)
}

/// The phone's own fastbootd serving the computer: whether it runs, how far
/// the computer has the port, and where to go from here, chosen with the
/// keys and lit where they are.
pub fn fastboot(cx: &Ctx) -> Frame {
    let (t, f, s, i, l) = (cx.t, cx.f, cx.state, cx.info, cx.state.lang);
    let mut c = page(cx);
    let top = t.m.content_top;
    let running = l.pick(if s.fastboot.running {
        crate::texts::fastboot::RUNNING
    } else {
        crate::texts::fastboot::NOT_RUNNING
    });
    let link = l.pick(match s.fastboot.link {
        UsbLink::Connected => crate::texts::fastboot::CONNECTED,
        UsbLink::Connecting => crate::texts::wlan::CONNECTING,
        UsbLink::Unplugged => crate::texts::fastboot::NO_COMPUTER,
    });
    let slot = i.slot.map(|s| s.letter());
    let mut facts = vec![
        Row::new("fastbootd").trail(Trail::Text(running)),
        Row::new("USB").trail(Trail::Text(link)),
        Row::new(l.pick(["设备代号", "裝置代號", "Codename"])).trail(Trail::Text(&i.codename)),
    ];
    if let Some(letter) = slot {
        facts.push(Row::new(l.pick(CURRENT_SLOT)).trail(Trail::Text(letter)));
    }
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::texts::fastboot::STATUS));
    let state = widgets::list(&mut c, t, f, y, &facts);
    let note = l.pick(crate::texts::fastboot::NOTE);
    let y = widgets::footnote(&mut c, t, f, state.bottom, note);
    let y = widgets::section(&mut c, t, f, y, l.pick(crate::texts::fastboot::LEAVE));
    let rows: Vec<Row> = FASTBOOT_CHOICES
        .iter()
        .map(|&(title, _)| Row::new(l.pick(title)).trail(Trail::None))
        .collect();
    let choices = widgets::list(&mut c, t, f, y, &rows);
    let actions: Vec<Action> = FASTBOOT_CHOICES.iter().map(|&(_, action)| action).collect();
    let out = hits(&choices.rows, &actions);
    widgets::footnote(
        &mut c,
        t,
        f,
        choices.bottom,
        l.pick(crate::texts::fastboot::KEYS),
    );
    Frame {
        back: false,
        ..Frame::new(t, c, out, "Fastboot", top)
    }
}

/// Where to go once what the system asked for has stopped or cannot be
/// done, in the order the pages list them.
const AFTERWARDS: [(Text, Action); 3] = [
    (
        crate::texts::installed::REBOOT_TO_SYSTEM,
        Action::RebootTo(Target::System),
    ),
    (crate::texts::afterwards::ENTER_RECOVERY, Action::Dismiss),
    (crate::texts::reboot::target::POWER_OFF, Action::RebootTo(Target::PowerOff)),
];

/// What work the system asked for says while it goes on of how the phone
/// leaves once it is done.
const POWERS_OFF_WHEN_DONE: Text = crate::texts::parts::leaving::POWER_OFF;
const REBOOTS_WHEN_DONE: Text = crate::texts::parts::leaving::REBOOT;

/// The list of [`AFTERWARDS`] at `y`, and where it ends. Work the user
/// chose in the recovery goes back to its home page, rather than into it.
fn afterwards(c: &mut Canvas, t: &Theme, f: &Fonts, l: Lang, y: f32, asked: bool, out: &mut Vec<Hit>) -> f32 {
    let rows: Vec<Row> = AFTERWARDS
        .iter()
        .map(|&(title, action)| {
            Row::new(l.pick(if action == Action::Dismiss && !asked {
                crate::texts::afterwards::BACK_HOME
            } else {
                title
            }))
            .trail(Trail::None)
        })
        .collect();
    let choices = widgets::list(c, t, f, y, &rows);
    out.extend(hits(&choices.rows, &AFTERWARDS.map(|(_, action)| action)));
    choices.bottom
}

/// Erasing: each part and how far it has come, and once erasing has
/// stopped, why. What the system asked for leaves on its own once it is
/// done, and offers where to go only if it stops; what the user chose
/// offers where to go once it ends either way.
pub fn erase(cx: &Ctx) -> Frame {
    use std::cmp::Ordering;
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let e = &s.erase;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let failed = matches!(e.outcome, Some(Err(_)));
    // Only a part that went wrong was under way; whatever else stopped
    // erasing stopped it before any part began.
    let begun = !failed || matches!(e.outcome, Some(Err(EraseStop::Failed(..))));
    let wiped = crate::texts::wipe::WIPED;
    let heading = l.pick(match &e.outcome {
        None => WIPING,
        Some(Ok(())) => wiped,
        Some(Err(_)) => WIPE_INCOMPLETE,
    });
    let mut y = widgets::section_first(&mut c, t, f, top, heading);
    if !e.parts.is_empty() {
        let states: Vec<String> = (0..e.parts.len())
            .map(|k| match (k.cmp(&e.done), failed) {
                (Ordering::Less, _) => l.pick(wiped).into(),
                (Ordering::Equal, false) => match e.progress {
                    Some((done, total)) if total > 0 => format!("{} {}%", l.pick(WIPING), percent(share(done, total))),
                    _ => l.pick(WIPING).into(),
                },
                (Ordering::Equal, true) if begun => l.pick(crate::texts::common::UNFINISHED).into(),
                (Ordering::Greater, false) => l.pick(WAITING_TURN).into(),
                (Ordering::Equal | Ordering::Greater, true) => l.pick(crate::texts::wipe::NOT_WIPED).into(),
            })
            .collect();
        let rows: Vec<Row> = e
            .parts
            .iter()
            .zip(&states)
            .map(|(&part, state)| Row::new(fmt::part(l, part)).trail(Trail::Text(state)))
            .collect();
        y = widgets::list(&mut c, t, f, y, &rows).bottom;
    }
    let notes: Vec<String> = match (&e.outcome, e.asked, e.reset) {
        (Some(Err(stop)), _, _) => {
            let (what, detail) = fmt::erase_stop(l, stop);
            std::iter::once(what).chain(detail).collect()
        }
        (_, true, true) => {
            let leaving = match s.asked.shutdown_after {
                true => crate::texts::erase::ALL_THEN_POWER_OFF,
                false => crate::texts::erase::ALL_THEN_REBOOT,
            };
            vec![l.pick(leaving).into()]
        }
        (_, true, false) => vec![
            l.pick(if s.asked.shutdown_after {
                POWERS_OFF_WHEN_DONE
            } else {
                REBOOTS_WHEN_DONE
            })
            .into(),
        ],
        (None, false, _) => vec![l.pick(KEEP_POWER).into()],
        (Some(Ok(())), false, true) => {
            vec![
                l.pick([
                    "下次开机时将重新创建 data 分区，启动时间较长。",
                    "下次開機時將重新建立 data 分割區，啟動時間較長。",
                    "The data partition is made anew at the next start, which then takes longer.",
                ])
                .into(),
            ]
        }
        (Some(Ok(())), false, false) => Vec::new(),
    };
    for note in &notes {
        y = widgets::footnote(&mut c, t, f, y, note);
    }
    if let (true, Some(reason)) = (e.asked, &s.asked.reason) {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(REASON), reason.clone())]);
    }
    if failed || (!e.asked && e.outcome.is_some()) {
        let below = widgets::section(&mut c, t, f, y, l.pick(ACTIONS));
        y = afterwards(&mut c, t, f, l, below, e.asked, &mut out);
    }
    let title = l.pick(match (e.reset, e.asked) {
        (true, _) => FACTORY_RESET,
        (false, true) => crate::texts::erase::WIPE_CACHE,
        (false, false) => crate::texts::page::WIPE,
    });
    Frame {
        back: false,
        ..Frame::new(t, fit(c, t, y), out, title, top)
    }
}

const FACTORY_RESET: Text = crate::texts::wipe::RESET;

/// The system could not start Android and asks whether to erase all user
/// data: start it again, look around the recovery first, or erase.
pub fn cant_start(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let what = l.pick(crate::texts::cant_start::WHAT);
    let mut y = widgets::paragraph(&mut c, t, f, top, what);
    if let Some(reason) = &s.asked.reason {
        y = widgets::info_card(&mut c, t, f, y + t.m.dp(12.0), &[(l.pick(REASON), reason.clone())]);
    }
    y = widgets::section(&mut c, t, f, y, l.pick(ACTIONS));
    let rows = [
        Row::new(l.pick(crate::texts::common::TRY_AGAIN))
            .subtitle(l.pick(crate::texts::cant_start::STARTS_AGAIN))
            .trail(Trail::None),
        Row::new(l.pick(crate::texts::afterwards::ENTER_RECOVERY))
            .subtitle(l.pick(crate::texts::cant_start::WIPES_NOTHING))
            .trail(Trail::None),
    ];
    let choices = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&choices.rows, &[Action::RebootTo(Target::System), Action::Dismiss]));
    let y = widgets::footnote(
        &mut c,
        t,
        f,
        choices.bottom,
        l.pick(crate::texts::cant_start::CANT_UNDO),
    );
    let (label, danger) = confirm_label(Confirm::FactoryReset, cx.state);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(24.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::FactoryReset),
    });
    Frame {
        back: false,
        ..Frame::new(t, c, out, l.pick(crate::texts::cant_start::TITLE), top)
    }
}

/// Something the system asked of the recovery that it does not do: what
/// was asked, and where to go instead.
pub fn unhandled(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut items = vec![(l.pick(["请求", "要求", "Request"]), s.asked.unhandled.clone().unwrap_or_default())];
    if let Some(reason) = &s.asked.reason {
        items.push((l.pick(REASON), reason.clone()));
    }
    let y = widgets::info_card(&mut c, t, f, top, &items);
    let y = widgets::footnote(
        &mut c,
        t,
        f,
        y,
        l.pick(crate::texts::unhandled::NOTE),
    );
    let y = widgets::section(&mut c, t, f, y, l.pick(ACTIONS));
    afterwards(&mut c, t, f, l, y, true, &mut out);
    Frame {
        back: false,
        ..Frame::new(
            t,
            c,
            out,
            l.pick(crate::texts::unhandled::TITLE),
            top,
        )
    }
}

/// Turning the phone off: what it means, and the swipe that does it. Its
/// own page rather than one line of the reboot page's list, because a phone
/// put off is not coming back until someone presses the key again, and
/// because what happens to work under way deserves saying.
pub fn power_off(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let x = t.m.w as f32 / 2.0;
    let sign_cy = t.m.dp(120.0);
    c.fill_circle(x, sign_cy, t.m.dp(30.0), t.pal.card);
    icons::glyph(&mut c, Icon::Power, x, sign_cy, t.m.dp(40.0), t.pal.accent);
    c.text_center(&f.heading, l.pick(POWER_OFF), x, sign_cy + t.m.dp(64.0), t.m.dp(24.0), t.pal.text);
    c.text_center(
        &f.body,
        l.pick(crate::lang::legacy([
            "手机将完全关闭，按电源键可再次开机。",
            "手機將完全關閉，按電源鍵可再次開機。",
            "The phone turns off completely. Press the power key to turn it on again.",
        ])),
        x,
        sign_cy + t.m.dp(96.0),
        t.m.sub_px,
        t.pal.subtitle,
    );
    let mut y = sign_cy + t.m.dp(150.0);
    // Work that would be cut short is named before the swipe, rather than
    // discovered afterwards.
    let running = l.pick(crate::lang::legacy([
        "正在进行的安装、备份或写入会被中断，并可能留下未完成的分区。",
        "正在進行的安裝、備份或寫入會被中斷，並可能留下未完成的分割區。",
        "An install, backup or write under way is cut off, and may leave a partition half-written.",
    ]));
    y = widgets::footnote(&mut c, t, f, y, running);
    let (label, danger) = confirm_label(Confirm::PowerOff, s);
    let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
    out.push(Hit {
        rect: track,
        action: Action::Confirm(Confirm::PowerOff),
    });
    Frame::new(t, fit(c, t, track.y + track.h), out, l.pick(POWER_OFF), top)
}

const POWER_OFF: Text = crate::texts::reboot::target::POWER_OFF;

/// The slot's vbmeta image: what it is, the copies kept of it, making one,
/// and putting one back. A copy older than the phone's rollback index is
/// refused rather than written, since writing it is what stops a phone
/// starting.
pub fn vbmeta(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = t.m.content_top;
    let mut y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["当前 vbmeta", "目前 vbmeta", "This slot's vbmeta"])));
    match &s.avb {
        Some(Ok(state)) => {
            let target = if state.suffix.is_empty() {
                "vbmeta".to_string()
            } else {
                format!("vbmeta{}", state.suffix)
            };
            let flags = state.flags;
            y = widgets::info_card(
                &mut c,
                t,
                f,
                y,
                &[
                    (l.pick(crate::lang::legacy(["目标分区", "目標分割區", "Target partition"])), target),
                    (l.pick(crate::lang::legacy(["标志", "旗標", "Flags"])), format!("0x{flags:08x}")),
                ],
            );
        }
        Some(Err(why)) => y = widgets::footnote(&mut c, t, f, y, why),
        None => y = widgets::footnote(&mut c, t, f, y, l.pick(crate::lang::legacy(["正在读取 vbmeta", "正在讀取 vbmeta", "Reading vbmeta"]))),
    }
    // A copy is the only way back from a vbmeta write, so it comes first.
    y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["备份", "備份", "Copies kept"])));
    let copies = match &s.vbmeta_copies {
        Some(Ok(copies)) => Some(copies),
        Some(Err(why)) => {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y,
                &tr!(
                    l,
                    "无法读取备份目录：{why}",
                    "無法讀取備份目錄：{why}",
                    "The copies can't be read: {why}"
                ),
            );
            None
        }
        None => None,
    };
    if let Some(copies) = copies {
        if copies.is_empty() {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y,
                l.pick(crate::lang::legacy([
                    "还没有留过副本，没有可写回的。",
                    "還沒有留過副本，沒有可寫回的。",
                    "No copy has been kept yet, so there is nothing to write back.",
                ])),
            );
        } else {
            let chosen = s.vbmeta_pending.as_ref();
            // `Row` borrows its words, so each copy's are built first and the
            // rows borrow from them.
            let said: Vec<(String, String)> = copies
                .iter()
                .map(|copy| {
                    let when = fmt::datetime(l, copy.made, s.zone());
                    let slot = match copy.suffix.as_str() {
                        "" => l.pick(crate::lang::legacy(["单槽位", "單槽位", "one slot"])).to_string(),
                        suffix => {
                            let slot = suffix.trim_start_matches('_').to_uppercase();
                            tr!(l, "{slot} 槽", "{slot} 槽", "slot {slot}")
                        }
                    };
                    let title = tr!(l, "{when} · {slot}", "{when} · {slot}", "{when} · {slot}");
                    // The number the guard compares, or why it cannot be read.
                    // A copy whose header will not read is shown rather than
                    // hidden: a person looking for one should see that it is
                    // there, and the write refuses on it anyway.
                    let sub = match &copy.rollback {
                        Ok(rollback) => {
                            let index = rollback.highest();
                            match rollback.chained() {
                                true => tr!(
                                    l,
                                    "回滚索引 {index}，另有链式分区",
                                    "回滾索引 {index}，另有鏈式分割區",
                                    "Rollback index {index}, with chained partitions"
                                ),
                                false => tr!(l, "回滚索引 {index}", "回滾索引 {index}", "Rollback index {index}"),
                            }
                        }
                        Err(why) => tr!(l, "无法读取：{why}", "無法讀取：{why}", "Can't be read: {why}"),
                    };
                    (title, sub)
                })
                .collect();
            let rows: Vec<Row> = said
                .iter()
                .zip(copies)
                .map(|((title, sub), copy)| {
                    let picked = chosen.is_some_and(|p| p.name == copy.name && p.place == copy.place);
                    Row::new(title).icon(GREEN, Icon::Backup).subtitle(sub).trail(Trail::Check(picked))
                })
                .collect();
            let list = widgets::list(&mut c, t, f, y, &rows);
            let picks: Vec<Action> = (0..copies.len()).map(Action::VbmetaRestore).collect();
            out.extend(hits(&list.rows, &picks));
            y = list.bottom;
        }
    }
    // Keeping a copy is offered for each place the phone has open; a place that
    // is not there has no row, rather than a row that could only fail.
    y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["留一份副本", "留一份副本", "Keep a copy"])));
    let mut keeping: Vec<Row> = Vec::new();
    let mut keeping_hits: Vec<Action> = Vec::new();
    if s.data.storage_open() {
        keeping.push(Row::new(place_name(l, Place::Storage)).trail(Trail::None));
        keeping_hits.push(Action::VbmetaBackup(Place::Storage));
    }
    if s.usb_drive.is_some() {
        keeping.push(Row::new(place_name(l, Place::UsbDrive)).trail(Trail::None));
        keeping_hits.push(Action::VbmetaBackup(Place::UsbDrive));
    }
    if keeping.is_empty() {
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(crate::lang::legacy([
                "内部存储尚未打开，无法留副本；请先在「解密」中打开 data 分区。",
                "內部儲存空間尚未開啟，無法留副本；請先在「解密」中開啟 data 分割區。",
                "Internal storage is not open, so a copy can't be kept. Open the data partition in Decrypt first.",
            ])),
        );
    } else {
        let save = widgets::list(&mut c, t, f, y, &keeping);
        out.extend(hits(&save.rows, &keeping_hits));
        y = save.bottom;
    }
    let find = widgets::list(
        &mut c,
        t,
        f,
        y + t.m.card_gap,
        &[Row::new(l.pick(crate::texts::wlan::REFRESH)).trail(Trail::None)],
    );
    out.extend(hits(&find.rows, &[Action::FindVbmeta]));
    y = find.bottom;
    // The rule that keeps a restore from bricking the phone, said where the
    // restore is offered and before the swipe that does it.
    let guard = l.pick(crate::lang::legacy([
        "写回前会比对回滚索引：副本的索引低于手机现在的值时会拒绝写回，因为写回它正是让手机无法开机的原因。相同或更高则写回。",
        "寫回前會比對回滾索引：副本的索引低於手機目前的值時會拒絕寫回，因為寫回它正是讓手機無法開機的原因。相同或更高則寫回。",
        "A copy is written back only if its rollback index is not below the one the phone holds now: writing an older one back is what stops a phone starting. An equal or higher index is written.",
    ]));
    y = widgets::footnote(&mut c, t, f, y, guard);
    if let Some(copy) = &s.vbmeta_pending {
        let name = copy.name.as_str();
        let what = match &copy.rollback {
            Ok(rollback) => {
                let index = rollback.highest();
                tr!(
                    l,
                    "将把 {name} 写回当前槽位的 vbmeta（索引 {index}）。写回前会重读分区现在的索引再判断一次。",
                    "將把 {name} 寫回目前槽位的 vbmeta（索引 {index}）。寫回前會重讀分割區目前的索引再判斷一次。",
                    "{name} will be written back into this slot's vbmeta (index {index}). The slot's own index is read again before it is judged."
                )
            }
            Err(why) => tr!(
                l,
                "{name} 无法读取，不能写回：{why}",
                "{name} 無法讀取，不能寫回：{why}",
                "{name} can't be read, so it can't be written back: {why}"
            ),
        };
        y = widgets::footnote(&mut c, t, f, y, &what);
        // Only a copy that reads is offered the swipe at all: one that does not
        // is refused where the write is made, so offering the gesture would be
        // offering something that cannot happen.
        if copy.rollback.is_ok() {
            let (label, danger) = confirm_label(Confirm::VbmetaRestore, s);
            let track = widgets::confirm(&mut c, t, f, y + t.m.dp(28.0), label, danger, s.confirm);
            out.push(Hit {
                rect: track,
                action: Action::Confirm(Confirm::VbmetaRestore),
            });
            y = track.y + track.h;
        }
    }
    if let Some(notice) = &s.vbmeta_notice {
        y = widgets::footnote(&mut c, t, f, y, notice);
    }
    Frame::new(t, fit(c, t, y), out, l.pick(VBMETA), top)
}

const VBMETA: Text = crate::lang::legacy(["vbmeta 备份", "vbmeta 備份", "vbmeta copies"]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BackupRefusal;

    fn job(restore: bool, checks: bool) -> BackupJob {
        let plan = vec![("boot".into(), 200), ("system".into(), 800)];
        BackupJob {
            restore,
            place: Place::Storage,
            checks,
            plan,
            at: None,
            outcome: None,
            stopping: false,
        }
    }

    fn at(item: usize, done: u64, of: u64, checking: bool) -> BackupAt {
        BackupAt { item, done, of, checking }
    }

    #[test]
    fn a_script_shows_its_last_line_until_skyward_goes_back() {
        let said = ["- Flashing boot".to_string(), "  ".to_string()];
        let l = Lang::Hans;
        assert_eq!(script_stage(l, None, &[]), ("正在运行安装脚本", None));
        assert_eq!(script_stage(Lang::En, None, &[]), ("Running the install script", None));
        assert_eq!(
            script_stage(l, Some(&Stage::Script { thousandths: 250 }), &said),
            ("- Flashing boot", Some(0.25)),
            "past a blank line"
        );
        assert_eq!(
            script_stage(l, Some(&Stage::Reflashing { done: 1, total: 4 }), &said),
            ("正在重新刷入 Skyward", Some(0.25))
        );
        assert_eq!(
            update_stage(Lang::En, Some(&Stage::Reflashing { done: 3, total: 4 })),
            ("Flashing Skyward again".to_string(), Some(0.75))
        );
        assert_eq!(
            update_stage(Lang::Hant, Some(&Stage::Writing { done: 1, total: 4 })).0,
            "第 1 步，共 3 步：寫入新系統 25%"
        );
    }

    #[test]
    fn a_backup_checks_its_parts_last_and_a_restore_first() {
        // Halfway through writing system, 600 of the 1000 bytes: the first
        // of two passes, then the second.
        assert_eq!(job_progress(&job(false, true), &at(1, 400, 800, false)), (0.3, 600));
        assert_eq!(job_progress(&job(false, true), &at(1, 400, 800, true)), (0.8, 600));
        assert_eq!(
            job_progress(&job(false, false), &at(1, 400, 800, false)),
            (0.6, 600),
            "without a check, writing is all"
        );
        assert_eq!(job_progress(&job(true, true), &at(0, 100, 200, true)), (0.05, 100));
        assert_eq!(job_progress(&job(true, true), &at(0, 100, 200, false)), (0.55, 100));
    }

    #[test]
    fn each_part_says_where_it_stands() {
        let states = |j: &BackupJob| (0..2).map(|k| part_state(Lang::Hans, j, k)).collect::<Vec<_>>();
        let mut b = job(false, true);
        assert_eq!(states(&b), ["等待中", "等待中"]);
        b.at = Some(at(1, 400, 800, false));
        assert_eq!(states(&b), ["已备份", "正在备份 50%"]);
        b.at = Some(at(0, 50, 200, true));
        assert_eq!(states(&b), ["正在校验 25%", "已备份"]);
        b.outcome = Some(BackupEnd::Failed("the backup does not hold what was stored".into()));
        assert_eq!(states(&b), ["未完成", "未校验"]);
        b.outcome = Some(BackupEnd::Saved("20261021-141639".into()));
        assert_eq!(states(&b), ["已校验", "已校验"]);
        let mut r = job(true, true);
        r.at = Some(at(1, 0, 800, true));
        assert_eq!(states(&r), ["已校验", "正在校验 0%"]);
        r.at = Some(at(0, 200, 400, false));
        assert_eq!(states(&r), ["正在恢复 50%", "已校验"]);
        r.outcome = Some(BackupEnd::Failed("write /dev/block/by-name/boot_b".into()));
        assert_eq!(states(&r), ["未完成", "未恢复"]);
        r.at = None;
        r.outcome = Some(BackupEnd::Refused(BackupRefusal::DataReset));
        assert_eq!(states(&r), ["未开始", "未开始"]);
    }

    #[test]
    fn each_part_says_where_it_stands_in_the_language() {
        let mut b = job(false, true);
        b.at = Some(at(1, 400, 800, false));
        assert_eq!(
            (0..2).map(|k| part_state(Lang::En, &b, k)).collect::<Vec<_>>(),
            ["Backed up", "Backing up 50%"]
        );
        let mut r = job(true, true);
        r.at = Some(at(0, 200, 400, false));
        assert_eq!(
            (0..2).map(|k| part_state(Lang::Hant, &r, k)).collect::<Vec<_>>(),
            ["正在還原 50%", "已驗證"]
        );
    }

    #[test]
    fn a_phrase_that_opens_a_line_starts_with_a_capital_in_english() {
        assert_eq!(capitalized(Lang::En, "empties the data partition"), "Empties the data partition");
        assert_eq!(capitalized(Lang::Hans, "清空 data 分区"), "清空 data 分区");
    }

    #[test]
    fn only_a_super_image_on_its_own_has_no_slots_to_pick() {
        assert!(super_image(&["super"]), "the whole of the dynamic partitions, in one partition");
        assert!(
            !super_image(&["super", "boot"]),
            "an image that also names another partition is not that"
        );
        assert!(!super_image(&["boot"]));
        assert!(
            !super_image(&[]),
            "an image with nowhere to go asks for nothing, rather than for a slot"
        );
    }

    #[test]
    fn a_flash_with_no_slot_is_said_without_one() {
        assert_eq!(flashed_detail(Lang::Hans, "super", &[]), "super 已刷入。");
        assert_eq!(flashed_detail(Lang::Hant, "super", &[]), "super 已刷入。");
        assert_eq!(flashed_detail(Lang::En, "super", &[]), "Flashed to super.");
        // An image that does go to slots reads as it did.
        assert_eq!(flashed_detail(Lang::Hans, "boot", &[Slot::A, Slot::B]), "boot 已刷入槽位 A 和 B。");
        assert_eq!(flashed_detail(Lang::Hant, "recovery", &[Slot::B]), "recovery 已刷入槽位 B。");
        assert_eq!(flashed_detail(Lang::En, "boot", &[Slot::A]), "Flashed to boot in slot A.");
        assert_eq!(
            flashed_detail(Lang::En, "boot", &[Slot::A, Slot::B]),
            "Flashed to boot in slots A and B."
        );
    }
}

/// The time zones of one part of the world, from the one furthest behind
/// UTC now to the one furthest ahead, each with how far that is and the
/// cities it is known by.
pub fn time_zones(cx: &Ctx, region: Region) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let mut c = long_page(cx);
    let top = t.m.content_top;
    let mut zones: Vec<usize> = (0..TIME_ZONES.len()).filter(|&at| TIME_ZONES[at].region == Some(region)).collect();
    zones.sort_by_key(|&at| fmt::utc_offset_seconds(s.now, fmt::rules(at)));
    let lines: Vec<String> = zones.iter().map(|&at| format!("{} · {}", fmt::utc_offset(s.now, fmt::rules(at)), l.pick(TIME_ZONES[at].cities))).collect();
    let kept = s.zone_listed();
    let rows: Vec<Row> = zones.iter().zip(&lines).map(|(&at, line)| Row::new(l.pick(TIME_ZONES[at].name)).subtitle(line).trail(Trail::Check(kept == Some(at)))).collect();
    let list = widgets::list(&mut c, t, f, top, &rows);
    let picks: Vec<Action> = zones.iter().map(|&at| Action::TimeZone(at)).collect();
    Frame::new(t, fit(c, t, list.bottom), hits(&list.rows, &picks), l.pick(region.name()), top)
}

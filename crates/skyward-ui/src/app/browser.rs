//! The browser's state, actions and effects, kept apart from the other pages.

use super::*;
use crate::icons::Icon;
use skyward_net::web::{FileOffer, Page as WebPage};

/// A shortcut on the browser's start page.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickLink {
    pub title: String,
    pub url: String,
    pub color: Rgba,
    pub icon: Icon,
}

/// The page the browser shows.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserView {
    Home,
    Loading(String),
    Page(WebPage),
    File(FileOffer),
    Full,
    Failed { url: String, why: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserSave {
    Downloading { done: u64, total: Option<u64> },
    Done,
    Stopped,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDownload {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size: Option<u64>,
    pub done: u64,
    pub state: BrowserSave,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub url: String,
    pub address: String,
    pub view: BrowserView,
    pub history: Vec<String>,
    pub at: usize,
    pub form: Vec<Vec<String>>,
    pub field: Option<(usize, usize)>,
}

impl BrowserTab {
    pub fn title(&self, _l: Lang) -> String {
        match &self.view {
            BrowserView::Page(page) if !page.title.is_empty() => page.title.clone(),
            BrowserView::File(file) => file.name.clone(),
            BrowserView::Loading(url) => url.clone(),
            BrowserView::Failed { url, .. } => url.clone(),
            _ => self.url.clone(),
        }
    }

    pub fn home() -> BrowserTab {
        BrowserTab {
            url: String::new(),
            address: String::new(),
            view: BrowserView::Home,
            history: Vec::new(),
            at: 0,
            form: Vec::new(),
            field: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserMark {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserFull {
    Unknown,
    Absent(String),
    Ready(String),
    Installing { done: u64, total: Option<u64> },
    Running(String),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginDownload {
    pub done: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub error: Option<String>,
    pub finished: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Browser {
    pub ime: crate::pinyin::Composition,
    pub picture: Option<BrowserPicture>,
    pub address: String,
    pub address_cursor: usize,
    pub address_start: usize,
    pub url: String,
    pub view: BrowserView,
    pub history: Vec<String>,
    pub at: usize,
    pub downloads: Vec<BrowserDownload>,
    pub saving: Option<usize>,
    pub remove_download: Option<usize>,
    pub removing_download: Option<String>,
    pub form: Vec<Vec<String>>,
    pub field: Option<(usize, usize)>,
    pub cookies: Vec<Cookie>,
    pub full: BrowserFull,
    pub full_shot: Option<Vec<u8>>,
    pub full_active: bool,
    pub prefer_full: bool,
    pub full_failures: u8,
    pub media_playing: bool,
    pub media_session: bool,
    pub desktop_mode: bool,
    pub landscape_mode: bool,
    pub immersive: bool,
    pub full_zoom: f32,
    pub full_text: bool,
    pub full_touch: (u16, u16),
    pub full_progress: Option<f32>,
    pub tabs: Vec<BrowserTab>,
    pub active: usize,
    pub quick: Vec<QuickLink>,
    pub recent: Vec<BrowserMark>,
    pub bookmarks: Vec<BrowserMark>,
    pub plugin_download: Option<PluginDownload>,
}

impl Default for Browser {
    fn default() -> Self {
        Browser {
            ime: crate::pinyin::Composition::default(),
            picture: None,
            address: String::new(),
            address_cursor: usize::MAX,
            address_start: 0,
            url: String::new(),
            view: BrowserView::Home,
            history: Vec::new(),
            at: 0,
            downloads: Vec::new(),
            saving: None,
            remove_download: None,
            removing_download: None,
            form: Vec::new(),
            field: None,
            cookies: Vec::new(),
            full: BrowserFull::Unknown,
            full_shot: None,
            full_active: false,
            prefer_full: false,
            full_failures: 0,
            media_playing: false,
            media_session: false,
            desktop_mode: false,
            landscape_mode: false,
            immersive: false,
            full_zoom: 1.0,
            full_text: false,
            full_touch: (500, 500),
            full_progress: None,
            tabs: vec![BrowserTab::home()],
            active: 0,
            quick: vec![
                QuickLink {
                    title: "Bing".into(),
                    url: "https://cn.bing.com".into(),
                    color: Rgba::rgb(0, 160, 160),
                    icon: Icon::Globe,
                },
                QuickLink {
                    title: "GitHub".into(),
                    url: "https://github.com/qingzhengdoufu/skyward".into(),
                    color: Rgba::rgb(52, 130, 255),
                    icon: Icon::Layers,
                },
            ],
            recent: Vec::new(),
            bookmarks: Vec::new(),
            plugin_download: None,
        }
    }
}

impl App {
    pub(super) fn set_browser_immersive(&mut self, enabled: bool) -> Vec<Effect> {
        if !self.state.browser.full_active || self.state.browser.immersive == enabled {
            return Vec::new();
        }
        let mut effects = self.browser_cancel_touch();
        self.immersive_pull = None;
        self.state.browser.immersive = enabled;
        self.state.browser.picture = None;
        self.state.browser.full_shot = None;
        self.keys = false;
        self.adjusting = false;
        self.drop_button();
        self.end_turn();
        self.settle_page();
        self.refresh();
        if !enabled {
            effects.push(Effect::BrowserPluginExitFullscreen);
        }
        effects
    }

    pub fn browser_plugin_fullscreen(&mut self, enabled: bool) -> Vec<Effect> {
        if enabled && !matches!(self.screen(), Screen::Browser | Screen::BrowserAddress) {
            return vec![Effect::BrowserPluginExitFullscreen];
        }
        self.set_browser_immersive(enabled)
    }

    pub(super) fn immersive_touch(&mut self, event: Touch, now: u64) -> Option<Vec<Effect>> {
        if !self.browser_immersive() {
            self.immersive_pull = None;
            return None;
        }
        if let Touch::Down(x, y) = event
            && y < self.theme().m.dp(12.0)
        {
            self.immersive_pull = Some((x, y));
            self.stir(now);
            return Some(Vec::new());
        }
        let (start_x, start_y) = self.immersive_pull?;
        match event {
            Touch::Move(x, y) if y - start_y > self.theme().m.dp(40.0) && y - start_y > (x - start_x).abs() => {
                let effects = self.set_browser_immersive(false);
                self.open_shade(now, true);
                self.drag = Drag::Held;
                self.stir(now);
                Some(effects)
            }
            Touch::Up => {
                self.immersive_pull = None;
                Some(Vec::new())
            }
            _ => Some(Vec::new()),
        }
    }

    pub fn browser_pinch(&mut self, scale: f32, now: u64) -> Vec<Effect> {
        if !self.browser_accepts_pinch() || !scale.is_finite() || scale <= 0.0 {
            return Vec::new();
        }
        self.drag = Drag::Idle;
        self.stir(now);
        vec![Effect::BrowserPluginPinch {
            scale: f64::from(scale.clamp(0.5, 2.0)),
        }]
    }

    pub fn browser_accepts_pinch(&self) -> bool {
        self.display_on
            && !self.charge_shown()
            && self.shade.is_none()
            && self.screen() == Screen::Browser
            && self.state.browser.full_active
    }

    pub fn browser_cancel_touch(&mut self) -> Vec<Effect> {
        self.nav_press = None;
        if matches!(self.drag, Drag::BrowserTouch { .. }) {
            self.drag = Drag::Idle;
            return vec![Effect::BrowserPluginCancelTouch];
        }
        self.drag = Drag::Idle;
        Vec::new()
    }

    pub(super) fn browser_commit_ime(&mut self, index: usize) -> Vec<Effect> {
        if self.state.browser.ime.spelling.is_empty() {
            return Vec::new();
        }
        let text = self.state.browser.ime.choose(index).unwrap_or_else(|| {
            let raw = self.state.browser.ime.spelling.clone();
            self.state.browser.ime.clear();
            raw
        });
        self.browser_insert_text(&text)
    }

    fn browser_insert_text(&mut self, text: &str) -> Vec<Effect> {
        if self.state.browser.full_active && self.state.browser.full_text {
            self.refresh();
            return vec![Effect::BrowserPluginText(text.to_string())];
        }
        let b = &mut self.state.browser;
        let at = widgets::text_boundary(&b.address, b.address_cursor);
        b.address.insert_str(at, text);
        b.address_cursor = at + text.len();
        b.address_start = b.address_start.min(at);
        self.edits += 1;
        self.refresh();
        Vec::new()
    }

    pub(super) fn browser_edit_key(&mut self, cap: Cap) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.state.browser.ime.enabled {
            match cap {
                Cap::Char(ch) if ch.is_ascii_alphabetic() || ch == '\'' => {
                    if self.state.browser.ime.spelling.len() < 32 {
                        self.state.browser.ime.spelling.push(ch.to_ascii_lowercase());
                        self.state.browser.ime.update();
                    }
                    self.refresh();
                    return effects;
                }
                Cap::Backspace if !self.state.browser.ime.spelling.is_empty() => {
                    self.state.browser.ime.spelling.pop();
                    self.state.browser.ime.update();
                    self.refresh();
                    return effects;
                }
                Cap::Space if !self.state.browser.ime.spelling.is_empty() => {
                    return self.browser_commit_ime(0);
                }
                _ => {
                    effects.extend(self.browser_commit_ime(0));
                }
            }
        }
        let shifted = self.state.layer == Layer::Shifted;
        if self.state.browser.full_active && self.state.browser.full_text {
            match cap {
                Cap::Char(ch) => {
                    let ch = if shifted { ch.to_ascii_uppercase() } else { ch };
                    effects.extend(self.browser_insert_text(&ch.to_string()));
                    if shifted {
                        self.state.layer = Layer::Letters;
                    }
                }
                Cap::Space => effects.extend(self.browser_insert_text(" ")),
                Cap::Backspace => effects.push(Effect::BrowserPluginKey(8)),
                _ => {}
            }
            self.refresh();
            return effects;
        }
        let b = &mut self.state.browser;
        let at = widgets::text_boundary(&b.address, b.address_cursor);
        match cap {
            Cap::Char(ch) => {
                let ch = if shifted { ch.to_ascii_uppercase() } else { ch };
                b.address.insert(at, ch);
                b.address_cursor = at + ch.len_utf8();
                if shifted {
                    self.state.layer = Layer::Letters;
                }
            }
            Cap::Space => {
                b.address.insert(at, ' ');
                b.address_cursor = at + 1;
            }
            Cap::Backspace if at > 0 => {
                let previous = b.address[..at].char_indices().last().map_or(0, |(i, _)| i);
                b.address.replace_range(previous..at, "");
                b.address_cursor = previous;
            }
            _ => {}
        }
        b.address_start = widgets::text_boundary(&b.address, b.address_start).min(b.address_cursor);
        self.edits += 1;
        self.refresh();
        effects
    }

    pub(super) fn browser_address_touch(&mut self, ev: Touch, f: &Fonts, now: u64) -> bool {
        if self.screen() != Screen::BrowserAddress || self.state.browser.full_text {
            return false;
        }
        let t = self.theme();
        let top = if self.state.browser.full_active {
            crate::browser::full_input_viewport(&t).y + crate::browser::full_input_viewport(&t).h
        } else {
            t.m.content_top + t.m.dp(36.0)
        };
        // Read the native field's exact top from its hit rectangle, avoiding
        // a duplicate section-height assumption.
        let top = if !self.state.browser.full_active {
            self.frame
                .as_ref()
                .and_then(|fr| fr.hits.iter().find(|hit| hit.action == Action::BrowserAddress))
                .map_or(top, |hit| hit.rect.y)
        } else {
            top
        };
        let area = Rect::new(t.m.margin, top, t.m.w as f32 - 2.0 * t.m.margin, t.m.dp(56.0));
        let x = match ev {
            Touch::Down(x, y) if area.contains(x, y + self.top().1) => {
                self.drop_button();
                self.end_turn();
                self.drag = Drag::BrowserCaret;
                x
            }
            Touch::Move(x, _) if matches!(self.drag, Drag::BrowserCaret) => x,
            Touch::Up if matches!(self.drag, Drag::BrowserCaret) => {
                self.drag = Drag::Idle;
                return true;
            }
            _ => return false,
        };
        let b = &mut self.state.browser;
        let (position, start) = widgets::text_field_position(&t, f, &b.address, b.address_cursor, b.address_start, x);
        b.address_cursor = position;
        b.address_start = start;
        self.edits += 1;
        self.stir(now);
        self.refresh();
        true
    }

    pub(super) fn browser_act(&mut self, action: Action, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        match action {
            Action::BrowserImeToggle => {
                let effects = self.browser_commit_ime(0);
                self.state.browser.ime.enabled = !self.state.browser.ime.enabled;
                self.refresh();
                return effects;
            }
            Action::BrowserImePick(index) => {
                if index < self.state.browser.ime.candidates.len() {
                    return self.browser_commit_ime(index);
                }
            }
            Action::BrowserImePage(next) => {
                let ime = &mut self.state.browser.ime;
                let pages = ime.candidates.len().div_ceil(3).max(1);
                ime.page = if next {
                    (ime.page + 1) % pages
                } else {
                    (ime.page + pages - 1) % pages
                };
                self.refresh();
            }
            Action::BrowserAddress => {
                self.state.browser.ime.clear();
                self.state.browser.full_text = false;
                self.state.browser.address = self.state.browser.url.clone();
                self.state.browser.address_cursor = self.state.browser.address.len();
                self.state.browser.address_start = 0;
                self.state.layer = Layer::Letters;
                self.open(Screen::BrowserAddress);
            }
            Action::BrowserBack => return self.browser_step(-1, now),
            Action::BrowserForward => return self.browser_step(1, now),
            Action::BrowserReload => {
                if self.state.browser.full_active {
                    return vec![Effect::BrowserPluginReload];
                }
                let url = self.state.browser.url.clone();
                if !url.is_empty() {
                    if cfg!(feature = "aera-browser") && matches!(self.state.browser.full, BrowserFull::Failed(_)) {
                        return self.browser_plugin_start_at(url);
                    }
                    return self.browser_read(url);
                }
            }
            Action::BrowserHome => {
                if self.state.browser.prefer_full {
                    return self.browser_plugin_start_at(String::new());
                }
                let full = self.state.browser.full_active;
                self.state.browser.full_active = false;
                self.state.browser.url.clear();
                self.state.browser.address.clear();
                self.state.browser.view = BrowserView::Home;
                self.state.browser.history.clear();
                self.state.browser.at = 0;
                self.open(Screen::Browser);
                if full {
                    return vec![Effect::BrowserPluginStop];
                }
            }
            Action::BrowserTabs => self.open(Screen::BrowserTabs),
            Action::BrowserNewTab => {
                self.state.browser.new_tab();
                self.open(Screen::Browser);
            }
            Action::BrowserPickTab(i) => {
                self.state.browser.pick_tab(i);
                self.open(Screen::Browser);
            }
            Action::BrowserCloseTab(i) => {
                self.state.browser.close_tab(i);
                self.refresh();
            }
            Action::BrowserDownloads => self.open(Screen::BrowserDownloads),
            Action::BrowserLink(i) => {
                if let Some(url) = self.state.browser.page().and_then(|p| p.blocks.get(i)).and_then(|b| match b {
                    skyward_net::web::Block::Link { url, .. } => Some(url.clone()),
                    _ => None,
                }) {
                    return self.browser_read(url);
                }
            }
            Action::BrowserFormField(form, field) => {
                let b = &mut self.state.browser;
                if b.form.len() <= form {
                    b.form.resize(form + 1, Vec::new());
                }
                if b.form[form].len() <= field {
                    b.form[form].resize(field + 1, String::new());
                }
                b.field = Some((form, field));
                self.state.layer = Layer::Letters;
                self.open(Screen::BrowserForm);
            }
            Action::BrowserFormSubmit(form) => return self.browser_submit(form),
            Action::BrowserFormClose => {
                self.state.browser.field = None;
                self.back(now);
            }
            Action::BrowserStop => {
                let url = self.state.browser.url.clone();
                self.state.browser.view = BrowserView::Failed {
                    url,
                    why: l.pick(["已停止", "已停止", "Stopped"]).into(),
                };
                self.refresh();
                return vec![Effect::StopBrowser];
            }
            Action::BrowserStopDownload => return vec![Effect::StopBrowser],
            Action::BrowserAskRemoveDownload(index) => {
                if self.state.browser.downloads.get(index).is_some() && self.state.browser.removing_download.is_none() {
                    self.state.browser.remove_download = Some(index);
                    self.refresh();
                }
            }
            Action::BrowserCancelRemoveDownload => {
                self.state.browser.remove_download = None;
                self.refresh();
            }
            Action::BrowserConfirmRemoveDownload => {
                if let Some(index) = self.state.browser.remove_download.take()
                    && let Some(download) = self.state.browser.downloads.get(index)
                {
                    let path = download.path.clone();
                    self.state.browser.removing_download = Some(path.clone());
                    self.refresh();
                    return vec![Effect::RemoveBrowserDownload { path }];
                }
            }
            Action::BrowserOpenDownload(i) => {
                let download = self.state.browser.downloads.get(i).cloned();
                if let Some(d) = download {
                    if matches!(d.state, BrowserSave::Stopped | BrowserSave::Failed(_)) {
                        return self.browser_download(d.url, d.name, now);
                    }
                    if d.state != BrowserSave::Done {
                        return Vec::new();
                    }
                    if [".zip", ".img", ".img.zst"]
                        .iter()
                        .any(|suffix| d.name.to_ascii_lowercase().ends_with(suffix))
                    {
                        return self.install_opened(d.path, d.done, now);
                    }
                    self.say(&d.path, now);
                }
            }
            Action::BrowserSaveFile => {
                let offer = match &self.state.browser.view {
                    BrowserView::File(file) => Some((file.url.clone(), file.name.clone())),
                    BrowserView::Page(page) => Some((page.url.clone(), skyward_net::web::file_name(&page.url))),
                    _ => None,
                };
                if let Some((url, name)) = offer {
                    return self.browser_download(url, name, now);
                }
            }
            Action::BrowserClearAddress => {
                self.state.browser.ime.clear();
                self.state.browser.address.clear();
                self.state.browser.address_cursor = 0;
                self.state.browser.address_start = 0;
                self.refresh();
            }
            Action::BrowserMenu => self.open(Screen::BrowserMenu),
            Action::BrowserImmersiveToggle if self.state.browser.full_active => {
                let enabled = !self.state.browser.immersive;
                self.open(Screen::Browser);
                return self.set_browser_immersive(enabled);
            }
            Action::BrowserDesktopToggle if self.state.browser.full_active => {
                self.state.browser.desktop_mode = !self.state.browser.desktop_mode;
                self.open(Screen::Browser);
                return vec![
                    Effect::BrowserPluginDesktop {
                        enabled: self.state.browser.desktop_mode,
                    },
                    Effect::BrowserPluginReload,
                ];
            }
            Action::BrowserLandscapeToggle if self.state.browser.full_active => {
                self.state.browser.landscape_mode = !self.state.browser.landscape_mode;
                self.open(Screen::Browser);
                return vec![Effect::BrowserPluginLandscape {
                    enabled: self.state.browser.landscape_mode,
                }];
            }
            Action::BrowserHistoryList => self.open(Screen::BrowserHistory),
            Action::BrowserBookmarksList => self.open(Screen::BrowserBookmarks),
            Action::BrowserBookmark => {
                if !self.state.browser.url.is_empty() {
                    let mark = BrowserMark {
                        title: self
                            .state
                            .browser
                            .page()
                            .map_or_else(|| self.state.browser.url.clone(), |page| page.title.clone()),
                        url: self.state.browser.url.clone(),
                    };
                    if let Some(at) = self.state.browser.bookmarks.iter().position(|b| b.url == mark.url) {
                        self.state.browser.bookmarks.remove(at);
                    } else {
                        self.state.browser.bookmarks.push(mark);
                    }
                    self.refresh();
                }
            }
            Action::BrowserMarkOpen(i) => {
                let mark = match self.screen() {
                    Screen::BrowserHistory => self.state.browser.recent.get(i).cloned(),
                    Screen::BrowserBookmarks => self.state.browser.bookmarks.get(i).cloned(),
                    _ => None,
                };
                if let Some(mark) = mark {
                    self.open(Screen::Browser);
                    return self.browser_read(mark.url);
                }
            }
            Action::BrowserMarkRemove(i) => {
                let list = if self.screen() == Screen::BrowserHistory {
                    &mut self.state.browser.recent
                } else {
                    &mut self.state.browser.bookmarks
                };
                if i < list.len() {
                    list.remove(i);
                    self.refresh();
                }
            }
            Action::BrowserQuick => self.open(Screen::BrowserQuick),
            Action::BrowserQuickOpen(i) => {
                if let Some(q) = self.state.browser.quick.get(i).cloned() {
                    self.open(Screen::Browser);
                    return self.browser_read(q.url);
                }
            }
            Action::BrowserQuickAdd => {
                if let Some(page) = self.state.browser.page()
                    && !self.state.browser.quick.iter().any(|q| q.url == page.url)
                {
                    self.state.browser.quick.push(QuickLink {
                        title: page.title.clone(),
                        url: page.url.clone(),
                        color: Rgba::rgb(0x34, 0x82, 0xff),
                        icon: Icon::Globe,
                    });
                    self.refresh();
                }
            }
            Action::BrowserQuickRemove(i) => {
                if i < self.state.browser.quick.len() {
                    self.state.browser.quick.remove(i);
                    self.refresh();
                }
            }
            Action::BrowserQuickReset => {
                self.state.browser.quick.clear();
                self.refresh();
            }
            Action::BrowserFull => {
                self.browser_check_full();
                self.open(Screen::BrowserFull);
            }
            Action::BrowserFullLoad => return self.browser_plugin_start_at(self.state.browser.url.clone()),
            Action::BrowserBasic => {
                self.state.browser.immersive = false;
                self.state.browser.media_playing = false;
                self.state.browser.prefer_full = false;
                self.state.browser.full_active = false;
                let url = self.state.browser.url.clone();
                self.open(Screen::Browser);
                if !url.is_empty() {
                    let mut effects = vec![Effect::BrowserPluginStop];
                    effects.extend(self.browser_read(url));
                    return effects;
                }
                return vec![Effect::BrowserPluginStop];
            }
            Action::BrowserPluginStart => return self.browser_plugin_start_at(self.state.browser.url.clone()),
            Action::BrowserPluginStop => {
                self.state.browser.immersive = false;
                self.state.browser.media_playing = false;
                self.state.browser.full_active = false;
                self.state.browser.prefer_full = false;
                self.state.browser.full_progress = None;
                self.state.browser.full = BrowserFull::Unknown;
                self.state.browser.view = BrowserView::Home;
                self.refresh();
                return vec![Effect::BrowserPluginStop];
            }
            Action::BrowserPluginInstall => return self.browser_plugin_install(now),
            Action::BrowserPluginRemove => {
                if self.state.browser.full_active {
                    return Vec::new();
                }
                let dir = "/data/skyward-browser/chromium".to_string();
                return vec![Effect::BrowserPluginRemove { dir }];
            }
            Action::BrowserPluginDownload => {
                if self.state.browser.downloading() {
                    return Vec::new();
                }
                if !self.state.data.storage_open() {
                    self.say(l.pick(["请先解密 data", "請先解密 data", "Decrypt data first"]), now);
                    return Vec::new();
                }
                self.state.browser.plugin_download = Some(PluginDownload {
                    done: 0,
                    total: None,
                    speed: 0,
                    error: None,
                    finished: false,
                });
                return vec![Effect::BrowserPluginDownload {
                    url: "https://github.com/Jamvii/skyward-for-aurora/releases/download/browser-v0.3.4-aurora/chromium.zip".into(),
                    path: "/data/media/0/Download/Skyward/chromium.zip".into(),
                }];
            }
            Action::BrowserPluginReload => return vec![Effect::BrowserPluginReload],
            Action::BrowserPluginScroll(dy) if self.state.browser.full_active => {
                return vec![Effect::BrowserPluginScroll {
                    dy: dy.clamp(-2000.0, 2000.0) as i64,
                }];
            }
            Action::BrowserZoomIn | Action::BrowserZoomOut if self.state.browser.full_active => {
                if self.screen() == Screen::BrowserMenu {
                    self.back(now);
                }
                return vec![Effect::BrowserPluginPinch {
                    scale: if action == Action::BrowserZoomIn { 1.2 } else { 0.8 },
                }];
            }
            Action::BrowserScrollUp | Action::BrowserScrollDown if self.state.browser.full_active => {
                if self.screen() == Screen::BrowserMenu {
                    self.back(now);
                }
                return vec![Effect::BrowserPluginScroll {
                    dy: if action == Action::BrowserScrollUp { 550 } else { -550 },
                }];
            }
            Action::BrowserKeyboard => {
                self.state.browser.ime.clear();
                self.state.layer = Layer::Letters;
                self.state.browser.full_text = true;
                self.state.browser.address.clear();
                self.state.browser.address_cursor = 0;
                self.state.browser.address_start = 0;
                self.open(Screen::BrowserAddress);
            }
            Action::BrowserPluginTap(x, y) => return vec![Effect::BrowserPluginTap { x, y }],
            Action::BrowserPluginPinch(scale) => return vec![Effect::BrowserPluginPinch { scale: f64::from(scale) }],
            Action::BrowserPluginText => {}
            _ => {}
        }
        Vec::new()
    }

    pub(super) fn is_browser_action(action: Action) -> bool {
        is_browser_action(action)
    }

    pub(super) fn browser_go(&mut self, input: &str, _now: u64) -> Vec<Effect> {
        let url = if self.state.browser.full_active {
            skyward_net::web::full_address(input)
        } else {
            skyward_net::web::address(input)
        };
        if url.is_empty() {
            return Vec::new();
        }
        self.browser_read(url)
    }

    pub(super) fn browser_read(&mut self, url: String) -> Vec<Effect> {
        self.state.browser.media_playing = false;
        self.state.browser.media_session = false;
        if self.state.browser.prefer_full && !self.state.browser.full_active {
            return self.browser_plugin_start_at(url);
        }
        if self.state.browser.url != url {
            self.state.browser.full_failures = 0;
        }
        let url = if self.state.browser.full_active {
            skyward_net::web::full_url(&url)
        } else {
            url
        };
        let b = &mut self.state.browser;
        if b.history.is_empty() {
            b.history.push(url.clone());
            b.at = 0;
        } else {
            b.history.truncate(b.at + 1);
            if b.history.last() != Some(&url) {
                b.history.push(url.clone());
                b.at = b.history.len() - 1;
            }
        }
        b.url = url.clone();
        b.address = url.clone();
        b.view = BrowserView::Loading(url.clone());
        b.form.clear();
        b.field = None;
        if b.full_active {
            b.view = BrowserView::Full;
            b.full_progress = Some(0.1);
            self.refresh();
            return vec![Effect::BrowserPluginStart {
                dir: "/data/skyward-browser/chromium".into(),
                url,
                out: "/tmp/skyward-full.jpg".into(),
            }];
        }
        let cookie = cookie_header(&b.cookies, &url);
        self.refresh();
        vec![Effect::BrowserOpen { url, cookie }]
    }

    fn browser_cookie(&self, url: &str) -> Option<String> {
        cookie_header(&self.state.browser.cookies, url)
    }

    fn browser_step(&mut self, by: isize, _now: u64) -> Vec<Effect> {
        let b = &mut self.state.browser;
        let at = b.at as isize + by;
        if at < 0 || at as usize >= b.history.len() {
            return Vec::new();
        }
        b.at = at as usize;
        let url = if b.full_active {
            skyward_net::web::full_url(&b.history[b.at])
        } else {
            b.history[b.at].clone()
        };
        b.history[b.at] = url.clone();
        b.url = url.clone();
        b.address = url.clone();
        b.view = BrowserView::Loading(url.clone());
        if b.full_active {
            b.view = BrowserView::Full;
            self.refresh();
            return vec![Effect::BrowserPluginStart {
                dir: "/data/skyward-browser/chromium".into(),
                url,
                out: "/tmp/skyward-full.jpg".into(),
            }];
        }
        let cookie = cookie_header(&b.cookies, &url);
        self.refresh();
        vec![Effect::BrowserOpen { url, cookie }]
    }

    fn browser_download(&mut self, url: String, name: String, _now: u64) -> Vec<Effect> {
        if !self.state.data.storage_open() || self.state.browser.downloading() || self.state.browser.removing_download.is_some() {
            return Vec::new();
        }
        let name = std::path::Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty() && *n != "." && *n != "..")
            .unwrap_or("download.bin")
            .to_string();
        let path = format!("/data/media/0/Download/Skyward/{name}");
        let index = self.state.browser.downloads.iter().position(|d| d.path == path);
        let download = BrowserDownload {
            name: name.clone(),
            url: url.clone(),
            path: path.clone(),
            size: None,
            done: 0,
            state: BrowserSave::Downloading { done: 0, total: None },
        };
        let index = match index {
            Some(index) => {
                self.state.browser.downloads[index] = download;
                index
            }
            None => {
                self.state.browser.downloads.push(download);
                self.state.browser.downloads.len() - 1
            }
        };
        self.state.browser.saving = Some(index);
        self.open(Screen::BrowserDownloads);
        self.refresh();
        vec![Effect::BrowserDownload { url, name, path }]
    }

    pub fn browser_opened(&mut self, opened: Result<skyward_net::web::Opened, String>) {
        let b = &mut self.state.browser;
        match opened {
            Ok(skyward_net::web::Opened::Page(page)) => {
                for cookie in &page.cookies {
                    b.cookies
                        .retain(|kept| !(kept.name == cookie.name && kept.domain == cookie.domain && kept.path == cookie.path));
                    b.cookies.push(Cookie {
                        name: cookie.name.clone(),
                        value: cookie.value.clone(),
                        domain: cookie.domain.clone(),
                        path: cookie.path.clone(),
                        expires: None,
                    });
                }
                b.url = page.url.clone();
                b.address = page.url.clone();
                b.recent.retain(|m| m.url != page.url);
                b.recent.insert(
                    0,
                    BrowserMark {
                        title: page.title.clone(),
                        url: page.url.clone(),
                    },
                );
                b.recent.truncate(60);
                b.view = BrowserView::Page(page);
            }
            Ok(skyward_net::web::Opened::File(file)) => {
                b.url = file.url.clone();
                b.address = file.url.clone();
                b.view = BrowserView::File(file);
            }
            Err(why) => {
                let url = b.url.clone();
                b.view = BrowserView::Failed { url, why };
            }
        }
        self.refresh();
    }

    pub fn browser_saved(&mut self, done: u64, total: Option<u64>) {
        let Some(i) = self.state.browser.saving else { return };
        let Some(d) = self.state.browser.downloads.get_mut(i) else { return };
        d.done = done;
        if total.is_some() {
            d.size = total;
        }
        d.state = BrowserSave::Downloading { done, total };
        self.refresh();
    }

    pub fn browser_save_ended(&mut self, ended: Result<(), String>, _now: u64) {
        let Some(i) = self.state.browser.saving.take() else { return };
        let Some(d) = self.state.browser.downloads.get_mut(i) else { return };
        d.state = match ended {
            Ok(()) => BrowserSave::Done,
            Err(why) if why == "stopped as asked" => BrowserSave::Stopped,
            Err(why) => BrowserSave::Failed(why),
        };
        self.refresh();
    }

    pub fn browser_download_removed(&mut self, path: &str, result: Result<(), String>, now: u64) {
        if self.state.browser.removing_download.as_deref() != Some(path) {
            return;
        }
        self.state.browser.removing_download = None;
        if let Err(error) = result {
            self.say(&error, now);
            self.refresh();
            return;
        }
        let b = &mut self.state.browser;
        if let Some(index) = b.downloads.iter().position(|download| download.path == path) {
            b.downloads.remove(index);
            b.saving = b.saving.and_then(|saving| {
                if saving == index {
                    None
                } else {
                    Some(saving - usize::from(saving > index))
                }
            });
        }
        self.refresh();
    }

    fn browser_submit(&mut self, form: usize) -> Vec<Effect> {
        let Some(page) = self.state.browser.page() else { return Vec::new() };
        let Some(f) = page.forms.get(form) else { return Vec::new() };
        let mut fields = Vec::new();
        for (i, field) in f.fields.iter().enumerate() {
            let value = self
                .state
                .browser
                .form
                .get(form)
                .and_then(|v| v.get(i))
                .cloned()
                .unwrap_or_else(|| field.value.clone());
            if !field.name.is_empty() {
                fields.push((field.name.clone(), value));
            }
        }
        let url = f.action.clone();
        let post = f.post;
        self.state.browser.field = None;
        self.state.browser.view = BrowserView::Loading(url.clone());
        let cookie = self.browser_cookie(&url);
        self.refresh();
        vec![Effect::BrowserSubmit { url, fields, post, cookie }]
    }

    fn browser_check_full(&mut self) {
        if self.state.browser.full_active {
            return;
        }
        let path = if cfg!(feature = "aera-browser") {
            "/skyward/browser/runtime.xz"
        } else {
            "/data/skyward-browser/chromium"
        };
        self.state.browser.full = if std::path::Path::new(path).exists() {
            BrowserFull::Ready(path.into())
        } else {
            BrowserFull::Absent("插件未安装".into())
        };
    }

    fn browser_plugin_install(&mut self, _now: u64) -> Vec<Effect> {
        if !self.state.data.storage_open() || self.state.browser.full_active {
            return Vec::new();
        }
        let dir = "/data/skyward-browser/chromium".to_string();
        for archive in [
            "/data/media/0/Download/Skyward/chromium.zip",
            "/sdcard/Download/Skyward/chromium.zip",
            "/sdcard/Skyward/plugins/chromium.zip",
        ] {
            if std::path::Path::new(archive).is_file() {
                self.state.browser.full = BrowserFull::Installing { done: 0, total: None };
                return vec![Effect::BrowserPluginInstall {
                    archive: archive.into(),
                    dir,
                }];
            }
        }
        Vec::new()
    }

    pub(super) fn browser_enter(&mut self) -> Vec<Effect> {
        if self.state.browser.full_active {
            self.open(Screen::Browser);
            return Vec::new();
        }
        self.browser_plugin_start_at(self.state.browser.url.clone())
    }

    fn browser_plugin_start_at(&mut self, url: String) -> Vec<Effect> {
        self.state.browser.prefer_full = true;
        self.state.browser.full_failures = 0;
        if !self.state.data.storage_open() {
            self.state.browser.full = BrowserFull::Failed(
                self.state
                    .lang
                    .pick([
                        "请先解密 data，再启动完整模式",
                        "請先解密 data，再啟動完整模式",
                        "Decrypt data before starting full mode",
                    ])
                    .into(),
            );
            self.open(Screen::BrowserFull);
            return Vec::new();
        }
        let url = if url.is_empty() {
            "https://cn.bing.com".to_string()
        } else {
            skyward_net::web::full_url(&url)
        };
        let dir = "/data/skyward-browser/chromium".to_string();
        let installed = if cfg!(feature = "aera-browser") {
            // Runtime identity and presence are checked by the background loader.
            true
        } else {
            std::path::Path::new(&dir).is_dir()
        };
        if !installed {
            self.state.browser.full = BrowserFull::Absent("插件未安装".into());
            self.open(Screen::BrowserFull);
            return Vec::new();
        }
        self.state.browser.full_active = true;
        self.state.browser.media_playing = false;
        self.state.browser.full_text = false;
        self.state.browser.ime.clear();
        self.state.browser.picture = None;
        self.state.browser.full_shot = None;
        self.state.browser.address = url.clone();
        self.state.browser.full = BrowserFull::Running(
            if cfg!(feature = "aera-browser") {
                "WPE WebKit"
            } else {
                "localhost:9222"
            }
            .into(),
        );
        self.state.browser.full_progress = Some(0.1);
        self.state.browser.view = BrowserView::Full;
        self.open(Screen::Browser);
        self.browser_read(url)
    }

    pub fn browser_plugin_failed(&mut self, why: String, now: u64) -> Vec<Effect> {
        if !self.state.browser.full_active {
            return Vec::new();
        }
        let b = &mut self.state.browser;
        let was_playing = b.media_playing;
        b.media_playing = false;
        b.full_failures = b.full_failures.saturating_add(1);
        b.full_text = false;
        b.ime.clear();
        b.picture = None;
        b.full_shot = None;
        b.full = BrowserFull::Failed(why.clone());
        let url = b.url.clone();
        let visible = matches!(self.screen(), Screen::Browser | Screen::BrowserAddress | Screen::BrowserFull);
        if visible && self.state.data.storage_open() && self.state.browser.full_failures <= 2 {
            self.state.browser.full_progress = Some(0.0);
            self.state.browser.view = BrowserView::Full;
            self.open(Screen::Browser);
            self.say(
                self.state.lang.pick([
                    "完整模式已中断，正在重试",
                    "完整模式已中斷，正在重試",
                    "Full mode stopped; retrying",
                ]),
                now,
            );
            return vec![Effect::BrowserPluginStart {
                dir: "/data/skyward-browser/chromium".into(),
                url,
                out: "/tmp/skyward-full.jpg".into(),
            }];
        }
        self.state.browser.full_active = false;
        self.state.browser.full_progress = None;
        if was_playing || self.state.browser.media_session {
            self.state.browser.prefer_full = true;
            self.state.browser.view = BrowserView::Failed { url, why };
            if visible {
                self.refresh();
            }
            return Vec::new();
        }
        if visible && self.state.data.storage_open() {
            self.state.browser.prefer_full = false;
            self.open(Screen::Browser);
            self.say(
                self.state.lang.pick([
                    "完整模式连续失败，已切换轻量模式",
                    "完整模式連續失敗，已切換輕量模式",
                    "Full mode failed repeatedly; using basic mode",
                ]),
                now,
            );
            return self.browser_read(url);
        }
        self.state.browser.view = BrowserView::Failed { url, why };
        self.refresh();
        Vec::new()
    }

    pub fn browser_plugin_shot(&mut self, shot: Result<Vec<u8>, String>) {
        if !self.state.browser.full_active {
            return;
        }
        match shot {
            Ok(shot) => {
                self.state.browser.picture = crate::browser::picture(&shot);
                self.state.browser.full_shot = Some(shot);
                self.state.browser.full_progress = Some(1.0);
                self.state.browser.view = BrowserView::Full;
            }
            Err(why) => {
                self.state.browser.full_active = false;
                self.state.browser.full_progress = None;
                let url = self.state.browser.url.clone();
                self.state.browser.view = BrowserView::Failed { url, why: why.clone() };
                self.state.browser.full = BrowserFull::Failed(why);
            }
        }
        self.refresh();
    }

    #[cfg(feature = "aera-browser")]
    pub fn browser_plugin_pixels(&mut self, width: u32, height: u32, rgba: Vec<u8>) {
        if !self.state.browser.full_active
            || width == 0
            || height == 0
            || u64::from(width) * u64::from(height) > 16_000_000
            || rgba.len() as u64 != u64::from(width) * u64::from(height) * 4
        {
            return;
        }
        if matches!(self.screen(), Screen::Browser | Screen::BrowserAddress) {
            let area = self.browser_viewport();
            if (width, height) != (area.w as u32, area.h as u32) {
                return;
            }
        }
        self.state.browser.picture = Some(BrowserPicture {
            width,
            height,
            pixels: std::sync::Arc::new(rgba),
        });
        self.state.browser.full_progress = Some(1.0);
        self.state.browser.view = BrowserView::Full;
        if self.browser_fullscreen() {
            self.refresh();
        }
    }

    pub fn browser_plugin_focus(&mut self, focus: bool) {
        if !self.state.browser.full_active || (self.screen() == Screen::BrowserAddress && !self.state.browser.full_text) {
            return;
        }
        if self.state.browser.full_text == focus {
            return;
        }
        self.state.browser.full_text = focus;
        if focus {
            self.state.browser.ime.clear();
            self.state.browser.address.clear();
            self.state.browser.address_cursor = 0;
            self.state.browser.address_start = 0;
            self.state.layer = Layer::Letters;
            if self.screen() == Screen::Browser {
                self.open(Screen::BrowserAddress);
            }
        } else if self.screen() == Screen::BrowserAddress {
            self.state.browser.ime.clear();
            self.open(Screen::Browser);
        }
    }

    pub fn browser_plugin_media_playing(&mut self, playing: bool, now: u64) {
        if !self.state.browser.full_active || self.state.browser.media_playing == playing {
            return;
        }
        self.state.browser.media_playing = playing;
        self.state.browser.media_session |= playing;
        self.stir(now);
    }

    pub fn browser_offered_download(&mut self, url: String, name: String, now: u64) -> Vec<Effect> {
        self.browser_download(url, name, now)
    }

    pub fn browser_navigated(&mut self, url: String) {
        if !skyward_net::web::readable_url(&url) {
            return;
        }
        let browser = &mut self.state.browser;
        if browser.url != url {
            browser.history.truncate(browser.at + 1);
            browser.history.push(url.clone());
            browser.at = browser.history.len() - 1;
            browser.url = url.clone();
            if self.screen() != Screen::BrowserAddress {
                self.state.browser.address = url;
            }
            if matches!(self.screen(), Screen::Browser | Screen::BrowserAddress | Screen::BrowserMenu) {
                self.refresh();
            }
        }
    }

    pub fn browser_plugin_installed(&mut self, ended: Result<(), String>) {
        self.state.browser.full = match ended {
            Ok(()) => BrowserFull::Ready("/data/skyward-browser/chromium".into()),
            Err(why) => BrowserFull::Failed(why),
        };
        self.refresh();
    }

    pub fn plugin_saved(&mut self, done: u64, total: Option<u64>, speed: u64) {
        if let Some(d) = self.state.browser.plugin_download.as_mut() {
            d.done = done;
            d.total = total;
            d.speed = speed;
            d.error = None;
            self.refresh();
        }
    }

    pub fn plugin_downloaded(&mut self, ended: Result<(), String>) {
        if let Some(d) = self.state.browser.plugin_download.as_mut() {
            d.finished = ended.is_ok();
            d.error = ended.err();
            self.refresh();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserPicture {
    pub width: u32,
    pub height: u32,
    pub pixels: std::sync::Arc<Vec<u8>>,
}

fn cookie_header(cookies: &[Cookie], url: &str) -> Option<String> {
    let pairs: Vec<(String, String, String, String)> = cookies
        .iter()
        .map(|c| (c.name.clone(), c.value.clone(), c.domain.clone(), c.path.clone()))
        .collect();
    skyward_net::web::cookie_header(&pairs, url)
}

fn is_browser_action(action: Action) -> bool {
    matches!(
        action,
        Action::BrowserPluginScroll(_)
            | Action::BrowserAddress
            | Action::BrowserImeToggle
            | Action::BrowserImePick(_)
            | Action::BrowserImePage(_)
            | Action::BrowserSaveFile
            | Action::BrowserQuickOpen(_)
            | Action::BrowserQuick
            | Action::BrowserQuickAdd
            | Action::BrowserQuickRemove(_)
            | Action::BrowserQuickReset
            | Action::BrowserHistory(_)
            | Action::BrowserDownloads
            | Action::BrowserFull
            | Action::BrowserBasic
            | Action::BrowserPluginTap(_, _)
            | Action::BrowserPluginPinch(_)
            | Action::BrowserPluginText
            | Action::BrowserPluginReload
            | Action::BrowserPluginStop
            | Action::BrowserPluginStart
            | Action::BrowserPluginInstall
            | Action::BrowserPluginRemove
            | Action::BrowserPluginDownload
            | Action::BrowserStop
            | Action::BrowserStopDownload
            | Action::BrowserOpenDownload(_)
            | Action::BrowserAskRemoveDownload(_)
            | Action::BrowserConfirmRemoveDownload
            | Action::BrowserCancelRemoveDownload
            | Action::BrowserClearAddress
            | Action::BrowserTabs
            | Action::BrowserNewTab
            | Action::BrowserPickTab(_)
            | Action::BrowserCloseTab(_)
            | Action::BrowserMenu
            | Action::BrowserDesktopToggle
            | Action::BrowserLandscapeToggle
            | Action::BrowserImmersiveToggle
            | Action::BrowserBookmark
            | Action::BrowserHistoryList
            | Action::BrowserBookmarksList
            | Action::BrowserMarkOpen(_)
            | Action::BrowserMarkRemove(_)
            | Action::BrowserLink(_)
            | Action::BrowserBack
            | Action::BrowserForward
            | Action::BrowserReload
            | Action::BrowserHome
            | Action::BrowserFormField(_, _)
            | Action::BrowserFormSubmit(_)
            | Action::BrowserFormClose
            | Action::BrowserFullLoad
            | Action::BrowserKeyboard
            | Action::BrowserScrollUp
            | Action::BrowserScrollDown
            | Action::BrowserZoomIn
            | Action::BrowserZoomOut
    )
}

impl Browser {
    pub fn page(&self) -> Option<&WebPage> {
        match &self.view {
            BrowserView::Page(page) => Some(page),
            _ => None,
        }
    }

    pub fn downloading(&self) -> bool {
        self.saving.is_some_and(|i| {
            self.downloads
                .get(i)
                .is_some_and(|d| matches!(d.state, BrowserSave::Downloading { .. }))
        })
    }

    pub fn new_tab(&mut self) {
        self.store_active();
        self.tabs.push(BrowserTab::home());
        self.active = self.tabs.len() - 1;
        self.load_active();
    }

    pub fn close_tab(&mut self, i: usize) {
        if self.tabs.len() <= 1 || i >= self.tabs.len() {
            return;
        }
        self.store_active();
        self.tabs.remove(i);
        if i < self.active {
            self.active -= 1;
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        self.load_active();
    }

    pub fn pick_tab(&mut self, i: usize) {
        if i < self.tabs.len() {
            self.store_active();
            self.active = i;
            self.load_active();
        }
    }

    fn load_active(&mut self) {
        let tab = &self.tabs[self.active];
        self.url = tab.url.clone();
        self.address = tab.address.clone();
        self.view = tab.view.clone();
        self.history = tab.history.clone();
        self.at = tab.at;
        self.form = tab.form.clone();
        self.field = tab.field;
    }

    pub fn store_active(&mut self) {
        let tab = &mut self.tabs[self.active];
        tab.url = self.url.clone();
        tab.address = self.address.clone();
        tab.view = self.view.clone();
        tab.history = self.history.clone();
        tab.at = self.at;
        tab.form = self.form.clone();
        tab.field = self.field;
    }
}

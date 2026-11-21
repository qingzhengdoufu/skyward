//! The browser's pages: the start page, a page read as text and links, the
//! address field, and the files saved.

use crate::app::{Action, BrowserFull, BrowserSave, BrowserView, Dock, Frame, Hit, Rows, Screen};
use crate::canvas::Canvas;
use crate::color::Rgba;
use crate::fmt;
use crate::font::Fonts;
use crate::geom::Rect;
use crate::icons::{self, Icon};
use crate::lang::{Lang, Text};
use crate::pages::{Band, Ctx};
use crate::theme::Theme;
use crate::widgets::{self, Row, Trail};
use skyward_net::web::{self, Block as WebBlock, Page as WebPage};

const BROWSER: Text = crate::lang::legacy(["浏览器", "瀏覽器", "Browser"]);
const OPEN_ADDRESS: Text = crate::lang::legacy(["打开网址", "開啟網址", "Open address"]);
const HINT: Text = crate::lang::legacy(["输入网址或搜索词", "輸入網址或搜尋詞", "Enter an address or search for something"]);
const GO: Text = crate::lang::legacy(["访问", "前往", "Go"]);
const CLEAR: Text = crate::lang::legacy(["清空", "清除", "Clear"]);
const DOWNLOADS: Text = crate::lang::legacy(["下载的文件", "下載的檔案", "Downloads"]);
const DOWNLOAD: Text = crate::texts::update::DOWNLOAD;
const RETRY: Text = crate::texts::common::TRY_AGAIN;
const NOT_NOW: Text = crate::lang::legacy(["输入其他网址", "輸入其他網址", "Open another address"]);
const NO_DOWNLOADS: Text = crate::lang::legacy(["还没有下载的文件", "還沒有下載的檔案", "Nothing has been downloaded yet"]);

const BLUE: Rgba = Rgba::rgb(0x34, 0x82, 0xff);
const GREEN: Rgba = Rgba::rgb(0x1d, 0xcd, 0x3a);
const PURPLE: Rgba = Rgba::rgb(0x77, 0x67, 0xf9);
const ORANGE: Rgba = Rgba::rgb(0xff, 0x69, 0x00);
const RED: Rgba = Rgba::rgb(0xfa, 0x38, 0x2e);

fn sheet(cx: &Ctx, h: u32) -> Canvas {
    let w = cx.t.m.w;
    let Band { rows, cols } = cx.band.unwrap_or(Band {
        rows: (0, h),
        cols: (0, w),
    });
    Canvas::band(w, h, rows.0..rows.1, cols.0..cols.1, cx.t.pal.bg)
}

fn page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h)
}

fn long_page(cx: &Ctx) -> Canvas {
    sheet(cx, cx.t.m.h * 4)
}

fn fit(c: Canvas, t: &Theme, bottom: f32) -> Canvas {
    c.crop_height(((bottom + t.m.margin * 2.0).ceil() as u32).max(t.m.h))
}

fn hits(rects: &[Rect], actions: &[Action]) -> Vec<Hit> {
    rects.iter().zip(actions).map(|(&rect, &action)| Hit { rect, action }).collect()
}

fn download_progress(c: &mut Canvas, t: &Theme, f: &Fonts, y: f32, title: &str, detail: &str, done: Option<f32>) -> f32 {
    let green = Theme {
        pal: crate::theme::Palette { accent: GREEN, ..t.pal },
        ..*t
    };
    widgets::progress_card(c, &green, f, y, title, detail, done)
}

/// The browser's page, in whatever state it stands.
pub fn screen(cx: &Ctx) -> Frame {
    match &cx.state.browser.view {
        BrowserView::Home => home(cx),
        BrowserView::Loading(url) => loading(cx, url),
        BrowserView::Full => full_page(cx),
        BrowserView::Failed { url, why } => failed(cx, url, why),
        BrowserView::Page(article) => read(cx, article),
        BrowserView::File(file) => {
            let mut c = page(cx);
            let top = cx.t.m.content_top;
            let mut out = Vec::new();
            let y = address_bar(&mut c, cx, top, &file.name);
            out.push(Hit {
                rect: rect_bar(cx, top),
                action: Action::BrowserAddress,
            });
            let mut y = y + cx.t.m.card_gap;
            y = widgets::section(
                &mut c,
                cx.t,
                cx.f,
                y - cx.t.m.card_gap,
                cx.state.lang.pick(crate::lang::legacy(["文件", "檔案", "File"])),
            );
            let size = file
                .size
                .map(fmt::file_size)
                .unwrap_or_else(|| cx.state.lang.pick(crate::lang::legacy(["大小未知", "大小未知", "Size unknown"])).to_string());
            let kind = &file.kind;
            let rows = [
                Row::new(&file.name).icon(GREEN, Icon::Install).trail(Trail::None),
                Row::new(cx.state.lang.pick(crate::texts::common::SIZE)).trail(Trail::Text(&size)),
                Row::new(cx.state.lang.pick(crate::lang::legacy(["类型", "類型", "Type"]))).trail(Trail::Text(kind)),
            ];
            let list = widgets::list(&mut c, cx.t, cx.f, y, &rows);
            y = list.bottom;
            let note = cx.state.lang.pick(crate::lang::legacy([
                "下载后可从文件管理打开或安装。",
                "下載後可從檔案管理開啟或安裝。",
                "Open or install the downloaded file from Files.",
            ]));
            y = widgets::footnote(&mut c, cx.t, cx.f, y, note);
            let row = Row::new(cx.state.lang.pick(DOWNLOAD)).icon(BLUE, Icon::Install).trail(Trail::Arrow);
            let download = widgets::list(&mut c, cx.t, cx.f, y + cx.t.m.card_gap, &[row]);
            out.push(Hit {
                rect: download.rows[0],
                action: Action::BrowserSaveFile,
            });
            y = download.bottom;
            let mut frame = Frame::new(cx.t, fit(c, cx.t, y), out, cx.state.lang.pick(BROWSER), top);
            frame.dock = Some(dock(cx));
            frame
        }
    }
}

fn rect_bar(cx: &Ctx, top: f32) -> Rect {
    let m = &cx.t.m;
    Rect::new(m.margin, top, m.w as f32 - m.margin * 2.0, m.dp(56.0))
}

/// An address bar at `top`, saying `text`, or the hint where it is empty.
fn address_bar(c: &mut Canvas, cx: &Ctx, top: f32, text: &str) -> f32 {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let r = rect_bar(cx, top);
    c.fill_round_rect(r.x, r.y, r.w, r.h, m.dp(28.0), t.pal.card);
    let glyph = m.dp(23.0);
    icons::glyph(c, Icon::Globe, r.x + m.inner + glyph / 2.0, r.y + r.h / 2.0, glyph, t.pal.accent);
    let x = r.x + m.inner + glyph + m.dp(12.0);
    let max = r.w - (x - r.x) - m.inner - m.dp(10.0);
    let (shown, color) = if text.is_empty() {
        (l.pick(HINT), t.pal.placeholder)
    } else {
        (text, t.pal.text)
    };
    c.text_vc(&f.body, shown, x, r.y + r.h / 2.0, m.row_title_px, color, Some(max));
    r.y + r.h
}

/// Shortcuts and current tasks; the address stays in the bottom toolbar.
fn home(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let w = m.w as f32 - m.margin * 2.0;
    let mut y = top;

    y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["快捷访问", "快速存取", "Shortcuts"])));
    let gap = m.dp(10.0);
    let cols = 4;
    let tile_w = (w - gap * (cols - 1) as f32) / cols as f32;
    let tile_h = m.dp(106.0);
    for (k, q) in s.browser.quick.iter().enumerate() {
        let row = k / cols;
        let col = k % cols;
        let tx = m.margin + col as f32 * (tile_w + gap);
        let ty = y + row as f32 * (tile_h + gap);
        c.fill_round_rect(tx, ty, tile_w, tile_h, m.dp(8.0), t.pal.card);
        let ccx = tx + tile_w / 2.0;
        let ccy = ty + m.dp(36.0);
        c.fill_circle(ccx, ccy, m.dp(22.0), q.color);
        icons::glyph(&mut c, q.icon, ccx, ccy, m.dp(26.0), Rgba::rgb(255, 255, 255));
        c.text_center(&f.body, &q.title, ccx, ty + m.dp(82.0), m.dp(13.0), t.pal.text);
        out.push(Hit {
            rect: Rect::new(tx, ty, tile_w, tile_h),
            action: Action::BrowserQuickOpen(k),
        });
    }
    let rows = s.browser.quick.len().div_ceil(cols);
    y += rows.max(1) as f32 * (tile_h + gap);
    let edit = widgets::list(
        &mut c,
        t,
        f,
        y,
        &[Row::new(l.pick(crate::lang::legacy(["编辑快捷访问", "編輯快速存取", "Edit shortcuts"])))
            .icon(BLUE, Icon::More)
            .trail(Trail::Arrow)],
    );
    out.extend(hits(&edit.rows, &[Action::BrowserQuick]));
    y = edit.bottom;

    // The addresses read before, newest first and once each.
    let mut seen: Vec<&str> = Vec::new();
    let mut recent: Vec<(usize, &String)> = Vec::new();
    for (i, url) in s.browser.history.iter().enumerate().rev() {
        if seen.contains(&url.as_str()) {
            continue;
        }
        seen.push(url);
        recent.push((i, url));
        if recent.len() == 4 {
            break;
        }
    }
    if !recent.is_empty() {
        y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["最近访问", "最近造訪", "Recent"])));
        let subs: Vec<String> = recent.iter().map(|(_, u)| u.to_string()).collect();
        let titles: Vec<String> = recent.iter().map(|(_, u)| web::host_of(u)).collect();
        let rows: Vec<Row> = recent
            .iter()
            .zip(&titles)
            .zip(&subs)
            .map(|((_, title), sub)| Row::new(title).subtitle(sub).icon(BLUE, Icon::Globe))
            .collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        let actions: Vec<Action> = recent.iter().map(|(i, _)| Action::BrowserHistory(*i)).collect();
        out.extend(hits(&list.rows, &actions));
        y = list.bottom;
    }

    if let Some(i) = s.browser.saving
        && let Some(d) = s.browser.downloads.get(i)
        && let BrowserSave::Downloading { done, total } = d.state
    {
        y = widgets::section(&mut c, t, f, y, l.pick(DOWNLOADS));
        let (head, detail, share) = saving_line(l, &d.name, done, total);
        y = download_progress(&mut c, t, f, y, head, &detail, share);
    }

    y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["更多", "更多", "More"])));
    let full_title = if cfg!(feature = "aera-browser") {
        l.pick(crate::lang::legacy(["网页引擎", "網頁引擎", "Web engine"]))
    } else {
        l.pick(crate::lang::legacy([
            "完整模式（Linux Chromium）",
            "完整模式（Linux Chromium）",
            "Full mode (Linux Chromium)",
        ]))
    };
    let full = Row::new(full_title)
        .icon(PURPLE, Icon::Layers)
        .subtitle(if cfg!(feature = "aera-browser") {
            "WPE WebKit"
        } else {
            l.pick(crate::lang::legacy([
                "外置插件，解密后可安装",
                "外置外掛，解密後可安裝",
                "An external plugin, installed after decryption",
            ]))
        });
    let list = widgets::list(&mut c, t, f, y, &[full]);
    out.extend(hits(&list.rows, &[Action::BrowserFull]));
    y = list.bottom;

    if !s.online() {
        y += m.card_gap * 2.0;
        y = widgets::section(&mut c, t, f, y, l.pick(crate::lang::legacy(["网络", "網路", "Network"])));
        let (title, sub) = match s.wlan.is_some() {
            true => (
                l.pick(crate::lang::legacy(["未连接 WLAN", "未連接 Wi-Fi", "Not on Wi-Fi"])).to_string(),
                l.pick(crate::lang::legacy(["点按连接网络", "點按連線網路", "Tap to join a network"])).to_string(),
            ),
            false => (
                l.pick(crate::lang::legacy(["本机未配置 WLAN", "本機未設定 Wi-Fi", "No Wi-Fi on this phone"]))
                    .to_string(),
                String::new(),
            ),
        };
        let row = Row::new(&title)
            .icon(RED, Icon::Wlan)
            .subtitle(&sub)
            .trail(if s.wlan.is_some() { Trail::Arrow } else { Trail::None });
        let list = widgets::list(&mut c, t, f, y, &[row]);
        if s.wlan.is_some() {
            out.extend(hits(&list.rows, &[Action::Open(Screen::Wlan)]));
        }
        y = list.bottom;
    }

    let mut frame = Frame::new(t, fit(c, t, y), out, l.pick(BROWSER), top);
    frame.dock = Some(dock(cx));
    frame
}

/// The engine owns the page; recovery owns the bottom address and tools.
fn full_page(cx: &Ctx) -> Frame {
    let (t, l) = (cx.t, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let area = engine_viewport(t, cx.state.browser.immersive);
    let top = area.y;
    let mut out = Vec::new();
    let y = top;
    if cx.state.browser.picture.is_some() || cx.state.browser.full_shot.is_some() {
        if let Some(picture) = cx
            .state
            .browser
            .picture
            .clone()
            .or_else(|| cx.state.browser.full_shot.as_deref().and_then(picture))
        {
            let (w, h) = (picture.width, picture.height);
            let x = area.x + (area.w - w as f32) / 2.0;
            c.blit_rgba(&picture.pixels, w, h, x.round() as i32, y.round() as i32);
            if let Some(progress) = cx.state.browser.full_progress.filter(|p| *p < 1.0) {
                let progress = progress.clamp(0.0, 1.0);
                c.fill_round_rect(0.0, top, m.w as f32 * progress, m.dp(3.0), m.dp(1.5), t.pal.accent);
            }
        } else {
            widgets::footnote(
                &mut c,
                t,
                cx.f,
                y,
                l.pick(crate::lang::legacy([
                    "完整模式返回的图片无法解码",
                    "完整模式回傳的圖片無法解碼",
                    "Full mode returned a picture that can't be decoded",
                ])),
            );
        }
    } else {
        let progress = cx.state.browser.full_progress.unwrap_or(0.0);
        widgets::progress_card(
            &mut c,
            t,
            cx.f,
            m.dp(60.0),
            l.pick(crate::lang::legacy(["正在渲染页面", "正在繪製頁面", "Drawing the page"])),
            &cx.state.browser.url,
            Some(progress),
        );
        out.push(Hit {
            rect: Rect::new(m.margin, top, m.w as f32 - m.margin * 2.0, m.dp(56.0)),
            action: Action::BrowserAddress,
        });
    }
    let mut frame = Frame::new(t, c.crop_height(m.h), out, l.pick(BROWSER), top);
    frame.fixed = true;
    frame.header = false;
    frame.back = false;
    frame.browser_area = Some(area);
    if !cx.state.browser.immersive {
        frame.dock = Some(full_dock(cx));
    }
    frame
}

pub fn full_viewport(t: &Theme) -> Rect {
    let top = (t.m.dp(48.0) / 4.0).ceil() * 4.0;
    let bottom = t.m.h as f32 - t.bottom_inset - t.m.dp(BROWSER_DOCK_DP);
    let height = ((bottom - top).max(400.0) / 4.0).floor() * 4.0;
    Rect::new(0.0, top, (t.m.w / 4 * 4) as f32, height)
}

pub fn engine_viewport(t: &Theme, immersive: bool) -> Rect {
    if immersive {
        Rect::new(0.0, 0.0, (t.m.w / 4 * 4) as f32, (t.m.h / 4 * 4) as f32)
    } else {
        full_viewport(t)
    }
}

pub fn full_input_viewport(t: &Theme) -> Rect {
    viewport_above(
        t,
        t.m.h as f32 - t.bottom_inset - widgets::keyboard_height(t) - t.m.dp(input_field_dp()),
    )
}

fn input_field_dp() -> f32 {
    if cfg!(feature = "pinyin-input") { 108.0 } else { 64.0 }
}

fn viewport_above(t: &Theme, bottom: f32) -> Rect {
    let m = &t.m;
    let y = full_viewport(t).y;
    let height = (bottom - y).max(m.dp(100.0));
    Rect::new(0.0, y, m.w as f32, height)
}

pub(crate) fn picture(bytes: &[u8]) -> Option<crate::app::BrowserPicture> {
    let (width, height, pixels) = decode_png(bytes).or_else(|| decode_jpeg(bytes))?;
    Some(crate::app::BrowserPicture {
        width,
        height,
        pixels: std::sync::Arc::new(pixels),
    })
}

fn loading(cx: &Ctx, url: &str) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let bar = address_bar(&mut c, cx, top, url);
    out.push(Hit {
        rect: rect_bar(cx, top),
        action: Action::BrowserAddress,
    });
    let y = bar + m.card_gap;
    let head = l.pick(crate::lang::legacy(["正在打开", "正在開啟", "Opening"]));
    let y = widgets::progress_card(&mut c, t, f, y, head, url, None);
    let note = l.pick(crate::lang::legacy([
        "这可能需要几秒。按返回可离开。",
        "這可能需要幾秒。按返回可離開。",
        "This may take a few seconds. Go back to leave.",
    ]));
    let y = widgets::footnote(&mut c, t, f, y, note);
    let stop = widgets::button(&mut c, t, f, y + m.card_gap, l.pick(crate::texts::common::STOP));
    out.push(Hit {
        rect: stop,
        action: Action::BrowserStop,
    });
    let mut frame = Frame::new(t, fit(c, t, y + m.card_gap + stop.h), out, l.pick(BROWSER), top);
    frame.dock = Some(dock(cx));
    frame
}

fn failed(cx: &Ctx, url: &str, why: &str) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let bar = address_bar(&mut c, cx, top, url);
    out.push(Hit {
        rect: rect_bar(cx, top),
        action: Action::BrowserAddress,
    });
    let y = widgets::section(&mut c, t, f, bar + m.card_gap, l.pick(crate::lang::legacy(["无法打开", "無法開啟", "Couldn't open it"])));
    let url_text = url.to_string();
    let row = Row::new(&url_text).icon(RED, Icon::Globe).subtitle(why).trail(Trail::None);
    let list = widgets::list(&mut c, t, f, y, &[row]);
    let mut y = list.bottom;
    let rows = [
        Row::new(l.pick(RETRY)).icon(BLUE, Icon::Restore).trail(Trail::None),
        Row::new(l.pick(NOT_NOW)).icon(ORANGE, Icon::Globe),
    ];
    let again = widgets::list(&mut c, t, f, y + m.card_gap, &rows);
    out.extend(hits(&again.rows, &[Action::BrowserReload, Action::BrowserAddress]));
    y = again.bottom;
    if !cx.state.online() {
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(crate::lang::legacy([
                "未连接 WLAN。先去 WLAN 页面连接网络。",
                "未連接 Wi-Fi。請先到 Wi-Fi 頁面連線。",
                "Not on Wi-Fi. Join a network on the Wi-Fi page first.",
            ])),
        );
    }
    let mut frame = Frame::new(t, fit(c, t, y), out, l.pick(BROWSER), top);
    frame.dock = Some(dock(cx));
    frame
}

fn saving_line(l: Lang, name: &str, done: u64, total: Option<u64>) -> (&'static str, String, Option<f32>) {
    let head = l.pick(crate::texts::update::DOWNLOADING);
    let detail = match total {
        Some(total) if total > 0 => format!("{} / {}", fmt::file_size(done), fmt::file_size(total)),
        _ => fmt::file_size(done),
    };
    let share = total.filter(|&t| t > 0).map(|t| (done as f32 / t as f32).clamp(0.0, 1.0));
    let _ = name;
    (head, detail, share)
}

fn dock(cx: &Ctx) -> Dock {
    dock_with(
        cx,
        &[
            Action::BrowserBack,
            Action::BrowserForward,
            Action::BrowserReload,
            Action::BrowserHistoryList,
            Action::BrowserBookmarksList,
            Action::BrowserDownloads,
        ],
    )
}

fn full_dock(cx: &Ctx) -> Dock {
    dock(cx)
}

const BROWSER_DOCK_DP: f32 = 122.0;

fn dock_with(cx: &Ctx, actions: &[Action]) -> Dock {
    let t = cx.t;
    let m = &t.m;
    let (h, pad, gap) = (m.dp(44.0), m.dp(64.0), m.dp(4.0));
    let overlay = cx.state.browser.full_active;
    let mut c = Canvas::new(m.w, (m.dp(BROWSER_DOCK_DP) + t.bottom_inset).ceil() as u32);
    c.clear(if overlay { Rgba::rgb(0, 0, 0).with_alpha(175) } else { t.pal.bg });
    let menu = Rect::new(m.w as f32 - m.margin - m.dp(40.0), m.dp(8.0), m.dp(40.0), m.dp(44.0));
    let bar = Rect::new(m.margin, m.dp(8.0), menu.x - m.margin - m.dp(6.0), m.dp(44.0));
    let url = &cx.state.browser.url;
    let address = if url.is_empty() {
        cx.state.lang.pick(OPEN_ADDRESS)
    } else {
        url.as_str()
    };
    widgets::search_capsule(&mut c, t, cx.f, bar, address);
    for k in 0..3 {
        c.fill_circle(
            menu.x + menu.w / 2.0,
            menu.y + menu.h / 2.0 + (k as f32 - 1.0) * m.dp(5.0),
            m.dp(1.6),
            t.pal.text,
        );
    }
    let n = actions.len() as f32;
    let w = (m.w as f32 - m.margin * 2.0 - gap * (n - 1.0)) / n;
    let mut rects = Vec::new();
    for (k, action) in actions.iter().enumerate() {
        let x = m.margin + k as f32 * (w + gap);
        let (cx0, cy0) = (x + w / 2.0, pad + h / 2.0);
        let ink = t.pal.text;
        match action {
            Action::BrowserBack => {
                c.line(cx0 + m.dp(4.0), cy0 - m.dp(8.0), cx0 - m.dp(4.0), cy0, m.dp(2.6), ink);
                c.line(cx0 - m.dp(4.0), cy0, cx0 + m.dp(4.0), cy0 + m.dp(8.0), m.dp(2.6), ink);
            }
            Action::BrowserForward => {
                c.line(cx0 - m.dp(4.0), cy0 - m.dp(8.0), cx0 + m.dp(4.0), cy0, m.dp(2.6), ink);
                c.line(cx0 + m.dp(4.0), cy0, cx0 - m.dp(4.0), cy0 + m.dp(8.0), m.dp(2.6), ink);
            }
            Action::BrowserReload => {
                c.stroke_arc(cx0, cy0, m.dp(8.0), -0.6, 4.8, m.dp(2.2), ink);
                c.line(cx0 + m.dp(6.0), cy0 - m.dp(7.0), cx0 + m.dp(9.0), cy0 - m.dp(2.0), m.dp(2.2), ink);
            }
            Action::BrowserHome => {
                c.line(cx0 - m.dp(8.0), cy0, cx0, cy0 - m.dp(8.0), m.dp(2.4), ink);
                c.line(cx0, cy0 - m.dp(8.0), cx0 + m.dp(8.0), cy0, m.dp(2.4), ink);
                c.stroke_round_rect(cx0 - m.dp(5.5), cy0 - m.dp(1.0), m.dp(11.0), m.dp(10.0), m.dp(1.5), m.dp(2.0), ink);
            }
            Action::BrowserTabs => {
                c.stroke_round_rect(cx0 - m.dp(8.0), cy0 - m.dp(6.0), m.dp(12.0), m.dp(12.0), m.dp(2.0), m.dp(2.0), ink);
                c.stroke_round_rect(cx0 - m.dp(1.0), cy0 - m.dp(9.0), m.dp(10.0), m.dp(10.0), m.dp(2.0), m.dp(2.0), ink);
            }
            Action::BrowserKeyboard => {
                c.stroke_round_rect(cx0 - m.dp(9.0), cy0 - m.dp(6.0), m.dp(18.0), m.dp(12.0), m.dp(2.0), m.dp(1.8), ink);
                for k in 0..3 {
                    let x = cx0 - m.dp(5.0) + k as f32 * m.dp(5.0);
                    c.line(x, cy0 - m.dp(2.0), x, cy0 + m.dp(1.0), m.dp(1.3), ink);
                }
                c.line(cx0 - m.dp(5.0), cy0 + m.dp(4.0), cx0 + m.dp(5.0), cy0 + m.dp(4.0), m.dp(1.5), ink);
            }
            Action::BrowserScrollUp | Action::BrowserScrollDown => {
                let up = action == &Action::BrowserScrollUp;
                let (y0, y1) = if up { (m.dp(3.0), m.dp(-7.0)) } else { (m.dp(-3.0), m.dp(7.0)) };
                c.line(cx0, cy0 + y0, cx0, cy0 + y1, m.dp(2.2), ink);
                c.line(
                    cx0 - m.dp(5.0),
                    cy0 + y1 - if up { -m.dp(4.0) } else { m.dp(4.0) },
                    cx0,
                    cy0 + y1,
                    m.dp(2.2),
                    ink,
                );
                c.line(
                    cx0 + m.dp(5.0),
                    cy0 + y1 - if up { -m.dp(4.0) } else { m.dp(4.0) },
                    cx0,
                    cy0 + y1,
                    m.dp(2.2),
                    ink,
                );
            }
            Action::BrowserZoomIn | Action::BrowserZoomOut => {
                c.stroke_round_rect(cx0 - m.dp(9.0), cy0 - m.dp(9.0), m.dp(18.0), m.dp(18.0), m.dp(9.0), m.dp(2.0), ink);
                c.line(cx0 - m.dp(5.0), cy0, cx0 + m.dp(5.0), cy0, m.dp(2.0), ink);
                if action == &Action::BrowserZoomIn {
                    c.line(cx0, cy0 - m.dp(5.0), cx0, cy0 + m.dp(5.0), m.dp(2.0), ink);
                }
            }
            Action::BrowserMenu => {
                for k in 0..3 {
                    c.fill_circle(cx0 + (k as f32 - 1.0) * m.dp(5.0), cy0, m.dp(1.8), ink);
                }
            }
            Action::BrowserHistoryList => crate::icons::glyph(&mut c, Icon::Clock, cx0, cy0, m.dp(23.0), ink),
            Action::BrowserBookmarksList => {
                let (dx, dy) = (m.dp(7.0), m.dp(9.0));
                c.line(cx0 - dx, cy0 - dy, cx0 + dx, cy0 - dy, m.dp(2.0), ink);
                c.line(cx0 - dx, cy0 - dy, cx0 - dx, cy0 + dy, m.dp(2.0), ink);
                c.line(cx0 + dx, cy0 - dy, cx0 + dx, cy0 + dy, m.dp(2.0), ink);
                c.line(cx0 - dx, cy0 + dy, cx0, cy0 + m.dp(3.0), m.dp(2.0), ink);
                c.line(cx0 + dx, cy0 + dy, cx0, cy0 + m.dp(3.0), m.dp(2.0), ink);
            }
            Action::BrowserFullLoad => {
                c.stroke_round_rect(cx0 - m.dp(8.0), cy0 - m.dp(8.0), m.dp(16.0), m.dp(16.0), m.dp(8.0), m.dp(1.8), ink);
                c.line(cx0 - m.dp(8.0), cy0, cx0 + m.dp(8.0), cy0, m.dp(1.5), ink);
                c.stroke_arc(cx0, cy0, m.dp(5.0), 0.0, std::f32::consts::TAU, m.dp(1.4), ink);
            }
            _ => {
                c.line(cx0, cy0 - m.dp(9.0), cx0, cy0 + m.dp(3.0), m.dp(2.4), ink);
                c.line(cx0 - m.dp(4.0), cy0 - m.dp(1.0), cx0, cy0 + m.dp(3.0), m.dp(2.4), ink);
                c.line(cx0 + m.dp(4.0), cy0 - m.dp(1.0), cx0, cy0 + m.dp(3.0), m.dp(2.4), ink);
                c.line(cx0 - m.dp(8.0), cy0 + m.dp(7.0), cx0 + m.dp(8.0), cy0 + m.dp(7.0), m.dp(2.0), ink);
            }
        }
        rects.push(Rect::new(x, pad, w, h));
    }
    let mut controls = hits(&rects, actions);
    controls.push(Hit {
        rect: bar,
        action: Action::BrowserAddress,
    });
    controls.push(Hit {
        rect: menu,
        action: Action::BrowserMenu,
    });
    Dock { canvas: c, hits: controls }
}

/// A page read as text and links: its title and address in a card, then a
/// row for each part, drawn as it scrolls in.
fn read(cx: &Ctx, article: &WebPage) -> Frame {
    let (t, l) = (cx.t, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let top = widgets::bar_bottom(t) + m.dp(8.0);
    let mut out = Vec::new();
    let header = article_head(&mut c, cx, article, top);
    let header = if article.scripts && article.blocks.is_empty() {
        let row = Row::new(l.pick(crate::lang::legacy(["此页面含 JavaScript", "此頁面含 JavaScript", "This page uses JavaScript"])))
            .subtitle(l.pick(crate::lang::legacy(["使用完整模式加载脚本页面", "使用完整模式載入指令碼頁面", "Open it in full mode"])))
            .icon(PURPLE, Icon::Layers)
            .trail(Trail::Arrow);
        let list = widgets::list(&mut c, t, cx.f, header, &[row]);
        out.extend(hits(&list.rows, &[Action::BrowserFullLoad]));
        list.bottom
    } else {
        header
    };
    let mut edges = vec![header.round()];
    for block in &article.blocks {
        let h = block_height(&c, cx, block);
        edges.push((*edges.last().unwrap() + h).round());
    }
    let mut rows = Vec::new();
    for (i, block) in article.blocks.iter().enumerate() {
        if let WebBlock::Link { .. } = block {
            rows.push(Hit {
                rect: Rect::new(0.0, edges[i], m.w as f32, edges[i + 1] - edges[i]),
                action: Action::BrowserLink(i),
            });
        }
        if let WebBlock::Form(form) = block
            && let Some(fields) = article.forms.get(*form).map(|f| &f.fields)
        {
            let mut y = edges[i] + m.dp(38.0);
            for (j, field) in fields.iter().enumerate() {
                if field.kind == web::FieldKind::Hidden {
                    continue;
                }
                let action = if field.kind == web::FieldKind::Submit {
                    Action::BrowserFormSubmit(*form)
                } else {
                    Action::BrowserFormField(*form, j)
                };
                rows.push(Hit {
                    rect: Rect::new(0.0, y, m.w as f32, m.dp(48.0)),
                    action,
                });
                y += m.dp(48.0);
            }
        }
    }
    out.extend(rows);
    let mut frame = Frame::new(t, c, out, l.pick(BROWSER), top);
    frame.header = false;
    frame.back = false;
    frame.rows = Some(Rows { edges, action: None });
    if cx.state.browser.field.is_some() {
        let (sheet, caret) = form_sheet(cx);
        frame.dock = Some(sheet);
        frame.caret = Some(caret);
    } else {
        frame.dock = Some(dock(cx));
    }
    frame
}

fn article_head(c: &mut Canvas, cx: &Ctx, page: &WebPage, top: f32) -> f32 {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let x = m.title_x;
    let max = m.w as f32 - x * 2.0;
    let title_lines = widgets::wrap(c, &f.heading, &page.title, m.dp(20.0), max);
    let desc = page.description.as_deref().filter(|d| !d.trim().is_empty());
    let desc_lines = desc.map(|d| widgets::wrap(c, &f.body, d, m.sub_px, max)).unwrap_or_default();
    let note = page.truncated.then(|| {
        l.pick(crate::lang::legacy([
            "页面较长，只显示了前面一部分。",
            "頁面較長，只顯示了前面一部分。",
            "The page is long, so only its start shows.",
        ]))
    });
    let mut y = top + m.dp(12.0);
    for line in &title_lines {
        c.text_top(&f.heading, line, x, y, m.dp(20.0), t.pal.text, Some(max));
        y += m.dp(27.0);
    }
    if !desc_lines.is_empty() {
        y += m.dp(8.0);
        for line in &desc_lines {
            c.text_top(&f.body, line, x, y, m.sub_px, t.pal.subtitle, Some(max));
            y += m.dp(19.0);
        }
    }
    if let Some(note) = note {
        y = widgets::footnote(c, t, f, y, note);
    }
    y + m.dp(12.0)
}

fn text_style<'a>(cx: &'a Ctx, style: &web::Style, heading: Option<u8>) -> (&'a crate::font::Face, f32, f32) {
    let m = &cx.t.m;
    let font = if style.bold {
        &cx.f.strong
    } else if heading.is_some() {
        &cx.f.heading
    } else {
        &cx.f.body
    };
    let px = style
        .size
        .map(|s| m.dp(s as f32).clamp(m.dp(12.0), m.dp(32.0)))
        .unwrap_or_else(|| heading.map_or(m.dp(15.0), |level| heading_px(level, cx.t)));
    let pitch = if let Some(level) = heading {
        (px * 1.3).max(heading_pitch(level, cx.t))
    } else {
        (px * 1.45).max(m.dp(20.0))
    };
    (font, px, pitch)
}

fn block_height(c: &Canvas, cx: &Ctx, block: &WebBlock) -> f32 {
    let (t, f) = (cx.t, cx.f);
    let m = &t.m;
    let x = m.title_x;
    let w = m.w as f32 - x * 2.0;
    match block {
        WebBlock::Heading { level, text, style } => {
            let (font, px, pitch) = text_style(cx, style, Some(*level));
            let lines = widgets::wrap(c, font, text, px, w);
            m.dp(20.0) + lines.len() as f32 * pitch + m.dp(8.0)
        }
        WebBlock::Paragraph { text, style } => {
            let (font, px, pitch) = text_style(cx, style, None);
            let lines = widgets::wrap(c, font, text, px, w);
            m.dp(10.0) + lines.len() as f32 * pitch + m.dp(10.0)
        }
        WebBlock::Bullet { text, .. } => {
            let lines = widgets::wrap(c, &f.body, text, m.dp(15.0), w - m.dp(22.0));
            m.dp(6.0) + lines.len() as f32 * m.dp(23.0) + m.dp(6.0)
        }
        WebBlock::Quote { text, .. } => {
            let lines = widgets::wrap(c, &f.body, text, m.dp(15.0), w - m.dp(20.0));
            m.dp(8.0) + lines.len() as f32 * m.dp(23.0) + m.dp(10.0)
        }
        WebBlock::Code { text, .. } => {
            let lines: usize = text
                .lines()
                .map(|line| widgets::wrap(c, &f.body, line, m.dp(13.0), w - m.dp(24.0)).len())
                .sum();
            m.dp(10.0) + lines.max(1) as f32 * m.dp(20.0) + m.dp(16.0)
        }
        WebBlock::Link { label, url, download, .. } => link_height(c, t, f, cx.state.lang, label, url, *download),
        WebBlock::Image { alt, data, .. } => {
            let _ = alt;
            if data.is_some() { m.dp(190.0) } else { m.dp(78.0) }
        }
        WebBlock::Form(i) => form_height(c, t, f, cx, *i),
        WebBlock::Rule => m.dp(22.0),
    }
}

fn heading_px(level: u8, t: &Theme) -> f32 {
    t.m.dp(match level {
        1 => 26.0,
        2 => 21.0,
        _ => 17.5,
    })
}

fn heading_pitch(level: u8, t: &Theme) -> f32 {
    t.m.dp(match level {
        1 => 34.0,
        2 => 28.0,
        _ => 24.0,
    })
}

fn link_height(c: &Canvas, t: &Theme, f: &Fonts, l: Lang, label: &str, url: &str, download: bool) -> f32 {
    let m = &t.m;
    let card_w = m.w as f32 - m.margin * 2.0;
    let text_x = m.inner + m.icon + m.icon_gap;
    let trail_w = match download {
        true => m.chevron_inset + m.chevron_w + m.value_gap * 2.0 + c.text_width(&f.value, l.pick(DOWNLOAD), m.sub_px),
        false => m.chevron_inset + m.chevron_w + m.value_gap,
    };
    let text_max = (card_w - text_x - trail_w).max(m.dp(40.0));
    let sub = web::host_of(url);
    let sub_lines = widgets::wrap(c, &f.body, &sub, m.sub_px, text_max);
    let titles = widgets::wrap(c, &f.heading, label, m.row_title_px, text_max);
    let (tt, tb) = f.heading.ink(m.row_title_px);
    let (st, sb) = f.body.ink(m.sub_px);
    let block = (tb - tt)
        + (titles.len().saturating_sub(1) as f32) * m.dp(24.0)
        + m.sub_gap
        + (sub_lines.len() as f32 - 1.0) * m.sub_pitch
        + (sb - st);
    let h = (block + m.row_pad_v * 2.0).max(m.row_h) + m.dp(10.0);
    h.ceil()
}

fn draw_block(c: &mut Canvas, cx: &Ctx, block: &WebBlock) {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    c.clear(t.pal.bg);
    let x = m.title_x;
    let w = m.w as f32 - x * 2.0;
    match block {
        WebBlock::Heading { level, text, style } => {
            let (font, px, pitch) = text_style(cx, style, Some(*level));
            let color = style.color.map(rgba).unwrap_or(t.pal.text);
            let mut y = m.dp(20.0);
            for line in widgets::wrap(c, font, text, px, w) {
                let lx = aligned_x(c, font, &line, px, x, w, style.align);
                c.text_top(font, &line, lx, y, px, color, None);
                if style.underline {
                    c.fill_rect(lx, y + px + m.dp(2.0), c.text_width(font, &line, px), m.dp(1.0), color);
                }
                y += pitch;
            }
        }
        WebBlock::Paragraph { text, style } => {
            let (font, px, pitch) = text_style(cx, style, None);
            let color = style.color.map(rgba).unwrap_or(t.pal.text);
            let mut y = m.dp(10.0);
            if let Some(bg) = style.background {
                c.fill_round_rect(
                    m.margin,
                    m.dp(4.0),
                    m.w as f32 - m.margin * 2.0,
                    block_height(c, cx, block) - m.dp(8.0),
                    m.dp(8.0),
                    rgba(bg),
                );
            }
            for line in widgets::wrap(c, font, text, px, w) {
                let lx = aligned_x(c, font, &line, px, x, w, style.align);
                c.text_top(font, &line, lx, y, px, color, None);
                if style.underline {
                    c.fill_rect(lx, y + px + m.dp(2.0), c.text_width(font, &line, px), m.dp(1.0), color);
                }
                y += pitch;
            }
        }
        WebBlock::Bullet { text, .. } => {
            let mut y = m.dp(6.0);
            c.fill_circle(x + m.dp(4.0), y + m.dp(10.0), m.dp(2.2), t.pal.accent);
            for line in widgets::wrap(c, &f.body, text, m.dp(15.0), w - m.dp(22.0)) {
                c.text_top(&f.body, &line, x + m.dp(22.0), y, m.dp(15.0), t.pal.text, None);
                y += m.dp(23.0);
            }
        }
        WebBlock::Quote { text, .. } => {
            let mut y = m.dp(8.0);
            c.fill_rect(
                x - m.dp(6.0),
                y,
                m.dp(3.0),
                (block_height(c, cx, block) - m.dp(18.0)).max(m.dp(10.0)),
                t.pal.accent.with_alpha(120),
            );
            for line in widgets::wrap(c, &f.body, text, m.dp(15.0), w - m.dp(20.0)) {
                c.text_top(&f.body, &line, x + m.dp(12.0), y, m.dp(15.0), t.pal.subtitle, None);
                y += m.dp(23.0);
            }
        }
        WebBlock::Code { text, .. } => {
            c.fill_round_rect(
                m.margin,
                m.dp(6.0),
                m.w as f32 - m.margin * 2.0,
                c.h as f32 - m.dp(12.0),
                m.dp(12.0),
                t.pal.card,
            );
            let mut y = m.dp(16.0);
            for line in text.lines() {
                for wrapped in widgets::wrap(c, &f.body, line, m.dp(13.0), w - m.dp(24.0)) {
                    c.text_top(&f.body, &wrapped, x + m.dp(12.0), y, m.dp(13.0), t.pal.info, None);
                    y += m.dp(20.0);
                }
            }
        }
        WebBlock::Link { label, url, download, .. } => {
            let trail_label = l.pick(DOWNLOAD);
            let trail = match download {
                true => Trail::Value(trail_label),
                false => Trail::Arrow,
            };
            let (color, icon) = match download {
                true => (ORANGE, Icon::Install),
                false => (BLUE, Icon::Globe),
            };
            let host = web::host_of(url);
            let text_x = m.margin + m.inner + m.icon + m.icon_gap;
            let trailing = m.chevron_inset
                + m.chevron_w
                + m.value_gap * 2.0
                + if *download {
                    c.text_width(&f.value, trail_label, m.sub_px)
                } else {
                    0.0
                };
            let width = (m.w as f32 - m.margin - text_x - trailing).max(m.dp(40.0));
            let lines = widgets::wrap(c, &f.heading, label, m.row_title_px, width);
            let title = lines.first().cloned().unwrap_or_default();
            let row = Row::new(&title).icon(color, icon).subtitle(&host).trail(trail);
            let _ = widgets::list(c, t, f, m.dp(5.0), &[row]);
            if lines.len() > 1 {
                c.fill_rect(text_x, m.dp(10.0), width, c.h as f32 - m.dp(14.0), t.pal.card);
                let mut y = m.dp(16.0);
                for line in lines {
                    c.text_top(&f.heading, &line, text_x, y, m.row_title_px, t.pal.text, Some(width));
                    y += m.dp(24.0);
                }
                c.text_top(&f.body, &host, text_x, y + m.dp(4.0), m.sub_px, t.pal.subtitle, Some(width));
            }
        }
        WebBlock::Image { alt, data, .. } => {
            if data.is_some() {
                draw_picture(c, cx, block);
            } else {
                let row = Row::new(l.pick(crate::lang::legacy(["图片", "圖片", "Picture"])))
                    .icon(PURPLE, Icon::Layers)
                    .subtitle(alt)
                    .trail(Trail::None);
                let _ = widgets::list(c, t, f, m.dp(6.0), &[row]);
            }
        }
        WebBlock::Form(i) => draw_form(c, cx, *i),
        WebBlock::Rule => {
            c.fill_rect(m.margin, m.dp(10.0), m.w as f32 - m.margin * 2.0, m.dp(1.0), t.pal.track);
        }
    }
}

/// A row of a page read: the part at `i`, drawn on `c` alone.
pub fn row(cx: &Ctx, i: usize, c: &mut Canvas) {
    let page = cx.state.browser.page().expect("the page read draws its rows");
    if let Some(block) = page.blocks.get(i) {
        draw_block(c, cx, block);
    }
}

/// The address field, the sites offered, and the keyboard.
pub fn address(cx: &Ctx) -> Frame {
    if cx.state.browser.full_active {
        return full_input(cx);
    }
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["网址", "網址", "Address"])));
    let (below, caret) = widgets::text_field_at(
        &mut c,
        t,
        f,
        y,
        &s.browser.address,
        s.browser.address_cursor,
        s.browser.address_start,
    );
    if s.browser.address.trim().is_empty() {
        let hint = l.pick(HINT);
        c.text_top(
            &f.body,
            hint,
            m.margin + m.inner,
            y + m.dp(16.0),
            m.row_title_px,
            t.pal.placeholder,
            Some(m.w as f32 - m.margin * 2.0 - m.inner * 2.0),
        );
    }
    out.push(Hit {
        rect: Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0)),
        action: Action::BrowserAddress,
    });
    let clear = widgets::list(&mut c, t, f, below + m.card_gap, &[Row::new(l.pick(CLEAR)).trail(Trail::None)]);
    out.extend(hits(&clear.rows, &[Action::BrowserClearAddress]));
    let y = widgets::section(&mut c, t, f, clear.bottom, l.pick(crate::lang::legacy(["快捷访问", "快速存取", "Shortcuts"])));
    let rows: Vec<Row> = s
        .browser
        .quick
        .iter()
        .map(|q| Row::new(&q.title).icon(q.color, q.icon).subtitle(&q.url))
        .collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    let actions: Vec<Action> = (0..s.browser.quick.len()).map(Action::BrowserQuickOpen).collect();
    out.extend(hits(&list.rows, &actions));
    let enter = l.pick(GO);
    let kb_top = t.m.h as f32 - widgets::keyboard_height(t);
    let keys = widgets::keyboard(&mut c, t, f, l, kb_top, s.layer, s.key_held(), enter);
    if cfg!(feature = "pinyin-input") {
        out.extend(ime_bar(&mut c, cx, kb_top - m.dp(44.0)));
    }
    out.extend(keys.into_iter().map(|(rect, cap)| Hit {
        rect,
        action: Action::Type(cap),
    }));
    Frame {
        fixed: true,
        caret: Some(caret),
        ..Frame::new(t, c, out, l.pick(OPEN_ADDRESS), top)
    }
}

fn full_input(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut frame = full_page(cx);
    let keys_top = m.h as f32 - t.bottom_inset - widgets::keyboard_height(t);
    let direct = s.browser.full_text;
    let field_top = keys_top
        - m.dp(if direct {
            if cfg!(feature = "pinyin-input") { 44.0 } else { 0.0 }
        } else {
            input_field_dp()
        });
    frame.canvas.fill_rect(0.0, field_top, m.w as f32, m.h as f32 - field_top, t.pal.bg);
    frame.hits.retain(|hit| hit.rect.y + hit.rect.h <= field_top);
    let caret = if direct {
        None
    } else {
        Some(
            widgets::text_field_at(
                &mut frame.canvas,
                t,
                f,
                field_top,
                &s.browser.address,
                s.browser.address_cursor,
                s.browser.address_start,
            )
            .1,
        )
    };
    let keys = widgets::keyboard(&mut frame.canvas, t, f, l, keys_top, s.layer, s.key_held(), l.pick(GO));
    if cfg!(feature = "pinyin-input") {
        frame.hits.extend(ime_bar(&mut frame.canvas, cx, keys_top - m.dp(44.0)));
    }
    frame.hits.extend(keys.into_iter().map(|(rect, cap)| Hit {
        rect,
        action: Action::Type(cap),
    }));
    frame.canvas = frame.canvas.crop_height(m.h);
    let area = engine_viewport(t, s.browser.immersive);
    frame.browser_area = Some(Rect::new(area.x, area.y, area.w, (field_top - area.y).max(0.0)));
    frame.dock = None;
    frame.caret = caret;
    frame
}

fn ime_bar(canvas: &mut Canvas, cx: &Ctx, top: f32) -> Vec<Hit> {
    crate::ime::bar(canvas, cx.t, cx.f, &cx.state.browser.ime, top, true)
}

/// The files saved and being saved.
pub fn downloads(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let mut out = Vec::new();
    let top = m.content_top;
    let mut y = top;
    if let Some(index) = s.browser.remove_download
        && let Some(download) = s.browser.downloads.get(index)
    {
        y = widgets::footnote(&mut c, t, f, y, &download.name);
        let confirm = widgets::list(
            &mut c,
            t,
            f,
            y,
            &[
                Row::new(l.pick(crate::lang::legacy(["删除任务和未完成文件", "刪除工作與未完成檔案", "Delete task and partial file"])))
                    .icon(RED, Icon::Wipe)
                    .trail(Trail::None),
                Row::new(l.pick(crate::texts::common::CANCEL)).trail(Trail::None),
            ],
        );
        out.extend(hits(
            &confirm.rows,
            &[Action::BrowserConfirmRemoveDownload, Action::BrowserCancelRemoveDownload],
        ));
        y = confirm.bottom + m.card_gap;
    }
    if s.browser.removing_download.is_some() {
        y = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(crate::lang::legacy(["正在停止并删除任务", "正在停止並刪除工作", "Stopping and deleting task"])),
        );
    }
    if let Some(i) = s.browser.saving
        && let Some(d) = s.browser.downloads.get(i)
        && let BrowserSave::Downloading { done, total } = d.state
    {
        let (head, detail, share) = saving_line(l, &d.name, done, total);
        y = download_progress(&mut c, t, f, y, head, &detail, share);
        let stop = widgets::list(
            &mut c,
            t,
            f,
            y + m.card_gap,
            &[Row::new(l.pick(crate::lang::legacy(["暂停下载", "暫停下載", "Pause download"])))
                .icon(RED, Icon::Wipe)
                .trail(Trail::None)],
        );
        out.extend(hits(&stop.rows, &[Action::BrowserStopDownload]));
        y = stop.bottom + m.card_gap;
    }
    if s.browser.downloads.is_empty() {
        let list = widgets::list(&mut c, t, f, y, &[Row::new(l.pick(NO_DOWNLOADS)).trail(Trail::None)]);
        y = list.bottom;
    } else {
        let names: Vec<String> = s.browser.downloads.iter().map(|d| d.name.clone()).collect();
        let subs: Vec<String> = s
            .browser
            .downloads
            .iter()
            .map(|d| {
                let state = match &d.state {
                    BrowserSave::Downloading { done, total } => match total {
                        Some(total) if *total > 0 => format!("{} / {}", fmt::file_size(*done), fmt::file_size(*total)),
                        _ => fmt::file_size(*done),
                    },
                    BrowserSave::Done => fmt::file_size(d.size.unwrap_or(d.done)),
                    BrowserSave::Stopped => format!("{} · {}", l.pick(["已暂停", "已暫停", "Paused"]), fmt::file_size(d.done)),
                    BrowserSave::Failed(why) => why.clone(),
                };
                format!("{state} · {}", d.path)
            })
            .collect();
        let rows: Vec<Row> = s
            .browser
            .downloads
            .iter()
            .zip(&names)
            .zip(&subs)
            .map(|((d, name), sub)| {
                let install = skyward_net::web::installable(name) && matches!(d.state, BrowserSave::Done);
                Row::new(name)
                    .icon(GREEN, Icon::Install)
                    .subtitle(sub)
                    .trail(Trail::DeleteTask(if install {
                        l.pick(crate::texts::page::INSTALL)
                    } else if matches!(d.state, BrowserSave::Stopped) {
                        l.pick(crate::lang::legacy(["继续", "繼續", "Resume"]))
                    } else if matches!(d.state, BrowserSave::Failed(_)) {
                        l.pick(crate::lang::legacy(["重试", "重試", "Retry"]))
                    } else {
                        ""
                    }))
            })
            .collect();
        let list = widgets::list(&mut c, t, f, y, &rows);
        for (index, rect) in list.rows.iter().enumerate() {
            let width = m.dp(44.0);
            out.push(Hit {
                rect: Rect {
                    w: rect.w - width,
                    ..*rect
                },
                action: Action::BrowserOpenDownload(index),
            });
            if s.browser.removing_download.is_none() {
                out.push(Hit {
                    rect: Rect::new(rect.x + rect.w - width, rect.y, width, rect.h),
                    action: Action::BrowserAskRemoveDownload(index),
                });
            }
        }
        y = list.bottom;
    }
    let note = l.pick(["Download/Skyward", "Download/Skyward", "Download/Skyward"]);
    y = widgets::footnote(&mut c, t, f, y + m.card_gap, note);
    Frame::new(t, fit(c, t, y), out, l.pick(DOWNLOADS), top)
}

/// The field and keyboard that rise over the page, rather than a page of
/// their own.
fn form_sheet(cx: &Ctx) -> (Dock, widgets::Caret) {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let (form, field) = s.browser.field.expect("the sheet has a field");
    let label = s
        .browser
        .page()
        .and_then(|p| p.forms.get(form))
        .and_then(|f| f.fields.get(field))
        .map(|f| if f.label.is_empty() { f.name.clone() } else { f.label.clone() })
        .unwrap_or_default();
    let value = s.browser.form.get(form).and_then(|v| v.get(field)).cloned().unwrap_or_default();
    let keys = widgets::keyboard_height(t);
    let h = keys + m.dp(92.0) + crate::ime::height(t);
    let mut c = Canvas::new(m.w, h.ceil() as u32);
    c.clear(t.pal.bg);
    c.fill_round_rect(m.margin, m.dp(8.0), m.w as f32 - m.margin * 2.0, m.dp(70.0), m.dp(18.0), t.pal.card);
    c.text_top(
        &f.label,
        &label,
        m.margin + m.dp(18.0),
        m.dp(18.0),
        m.dp(13.0),
        t.pal.subtitle,
        None,
    );
    let close = Rect::new(m.w as f32 - m.margin - m.dp(36.0), m.dp(18.0), m.dp(28.0), m.dp(28.0));
    c.line(
        close.x + m.dp(6.0),
        close.y + m.dp(6.0),
        close.x + close.w - m.dp(6.0),
        close.y + close.h - m.dp(6.0),
        m.dp(2.0),
        t.pal.subtitle,
    );
    c.line(
        close.x + close.w - m.dp(6.0),
        close.y + m.dp(6.0),
        close.x + m.dp(6.0),
        close.y + close.h - m.dp(6.0),
        m.dp(2.0),
        t.pal.subtitle,
    );
    let (_, caret) = widgets::text_field(&mut c, t, f, m.dp(40.0), &value);
    let mut out = vec![
        Hit {
            rect: Rect::new(m.margin, m.dp(8.0), m.w as f32 - m.margin * 2.0, m.dp(70.0)),
            action: Action::BrowserFormField(form, field),
        },
        Hit {
            rect: Rect::new(m.w as f32 - m.margin - m.dp(36.0), m.dp(18.0), m.dp(28.0), m.dp(28.0)),
            action: Action::BrowserFormClose,
        },
    ];
    if cfg!(feature = "pinyin-input") {
        out.extend(crate::ime::bar(&mut c, t, f, &s.ime, m.dp(92.0), false));
    }
    let keys = widgets::keyboard(
        &mut c,
        t,
        f,
        l,
        m.dp(92.0) + crate::ime::height(t),
        s.layer,
        s.key_held(),
        l.pick(GO),
    );
    out.extend(keys.into_iter().map(|(rect, cap)| Hit {
        rect,
        action: Action::Type(cap),
    }));
    (Dock { canvas: c, hits: out }, caret)
}

/// One field of a page's form, typed on the keyboard.
pub fn form(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let top = m.content_top;
    let (form, field) = s.browser.field.expect("the form page has a field");
    let label = s
        .browser
        .page()
        .and_then(|p| p.forms.get(form))
        .and_then(|f| f.fields.get(field))
        .map(|f| if f.label.is_empty() { f.name.clone() } else { f.label.clone() })
        .unwrap_or_default();
    let value = s.browser.form.get(form).and_then(|v| v.get(field)).cloned().unwrap_or_default();
    let y = widgets::section_first(&mut c, t, f, top, &label);
    let (below, caret) = widgets::text_field(&mut c, t, f, y, &value);
    let mut out = vec![Hit {
        rect: Rect::new(m.margin, y, m.w as f32 - m.margin * 2.0, m.dp(56.0)),
        action: Action::BrowserFormField(form, field),
    }];
    let kb_top = t.m.h as f32 - widgets::keyboard_height(t);
    if cfg!(feature = "pinyin-input") {
        out.extend(crate::ime::bar(&mut c, t, f, &s.ime, kb_top - crate::ime::height(t), false));
    }
    let keys = widgets::keyboard(&mut c, t, f, l, kb_top, s.layer, s.key_held(), l.pick(GO));
    out.extend(keys.into_iter().map(|(rect, cap)| Hit {
        rect,
        action: Action::Type(cap),
    }));
    let _ = below;
    Frame {
        caret: Some(caret),
        ..Frame::new(t, c, out, l.pick(crate::lang::legacy(["填写", "填寫", "Fill in"])), top)
    }
}

/// The optional full browser plugin.
pub fn full(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let top = m.content_top;
    let mut out = Vec::new();
    let mut y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["完整模式", "完整模式", "Full mode"])));
    let status = match &s.browser.full {
        BrowserFull::Unknown if cfg!(feature = "aera-browser") => l.pick(crate::lang::legacy(["正在检查引擎", "正在檢查引擎", "Checking engine"])),
        BrowserFull::Ready(_) if cfg!(feature = "aera-browser") => l.pick(crate::lang::legacy(["引擎就绪", "引擎就緒", "Engine ready"])),
        BrowserFull::Installing { .. } if cfg!(feature = "aera-browser") => l.pick(crate::lang::legacy(["正在准备引擎", "正在準備引擎", "Preparing engine"])),
        BrowserFull::Unknown => l.pick(crate::lang::legacy(["尚未检查插件", "尚未檢查外掛", "The plugin hasn't been checked"])),
        BrowserFull::Absent(why) => why.as_str(),
        BrowserFull::Ready(_) => l.pick(crate::lang::legacy(["插件已安装", "外掛已安裝", "The plugin is installed"])),
        BrowserFull::Installing { .. } => l.pick(crate::lang::legacy(["正在安装插件", "正在安裝外掛", "Installing the plugin"])),
        BrowserFull::Running(_) => l.pick(crate::lang::legacy(["完整模式正在运行", "完整模式正在執行", "Full mode is running"])),
        BrowserFull::Failed(why) => why.as_str(),
    };
    y = widgets::footnote(&mut c, t, f, y + m.dp(4.0), status);
    let mut rows = Vec::new();
    let mut actions = Vec::new();
    if !cfg!(feature = "aera-browser") {
        rows.push(
            Row::new(l.pick(crate::lang::legacy(["在线下载插件", "線上取得外掛", "Download the plugin"])))
                .icon(BLUE, Icon::Install)
                .subtitle("GitHub Releases · chromium.zip")
                .trail(Trail::Arrow),
        );
        actions.push(Action::BrowserPluginDownload);
    }
    match &s.browser.full {
        BrowserFull::Ready(_) | BrowserFull::Failed(_) => {
            rows.push(
                Row::new(l.pick(crate::lang::legacy(["启动完整模式", "啟動完整模式", "Start full mode"])))
                    .icon(GREEN, Icon::Install)
                    .trail(Trail::Arrow),
            );
            actions.push(Action::BrowserPluginStart);
            if !cfg!(feature = "aera-browser") {
                rows.push(
                    Row::new(l.pick(crate::lang::legacy(["移除插件", "移除外掛", "Remove the plugin"])))
                        .icon(ORANGE, Icon::Wipe)
                        .trail(Trail::None),
                );
                actions.push(Action::BrowserPluginRemove);
            }
        }
        BrowserFull::Running(_) => {
            rows.push(
                Row::new(l.pick(crate::lang::legacy(["停止完整模式", "停止完整模式", "Stop full mode"])))
                    .icon(ORANGE, Icon::Power)
                    .trail(Trail::None),
            );
            actions.push(Action::BrowserPluginStop);
        }
        BrowserFull::Installing { .. } => {}
        _ if !cfg!(feature = "aera-browser") => {
            rows.push(
                Row::new(l.pick(crate::lang::legacy(["安装或更新", "安裝或更新", "Install or update"])))
                    .icon(GREEN, Icon::Install)
                    .trail(Trail::None),
            );
            actions.push(Action::BrowserPluginInstall);
        }
        _ => {}
    }
    let list = widgets::list(&mut c, t, f, y + m.card_gap, &rows);
    out.extend(hits(&list.rows, &actions));
    y = list.bottom;
    if let Some(d) = &s.browser.plugin_download {
        if let Some(why) = &d.error {
            y = widgets::footnote(&mut c, t, f, y + m.card_gap, why);
        } else if d.finished {
            y = widgets::footnote(
                &mut c,
                t,
                f,
                y + m.card_gap,
                l.pick(crate::lang::legacy([
                    "插件已下载，点上面的「安装或更新」安装",
                    "外掛已下載，點上面的「安裝或更新」安裝",
                    "Plugin downloaded. Tap Install or update above.",
                ])),
            );
        } else {
            let total = d.total;
            let detail = match total {
                Some(total) if total > 0 => format!(
                    "{} / {} · {}/s",
                    fmt::file_size(d.done),
                    fmt::file_size(total),
                    fmt::file_size(d.speed)
                ),
                _ => format!("{} · {}/s", fmt::file_size(d.done), fmt::file_size(d.speed)),
            };
            let share = total.filter(|&t| t > 0).map(|t| (d.done as f32 / t as f32).clamp(0.0, 1.0));
            y = download_progress(
                &mut c,
                t,
                f,
                y + m.card_gap,
                l.pick(crate::lang::legacy(["正在下载插件", "正在下載外掛", "Downloading the plugin"])),
                &detail,
                share,
            );
        }
    }
    if s.browser.full_shot.is_none()
        && let Some(progress) = s.browser.full_progress
    {
        y = widgets::progress_card(
            &mut c,
            t,
            f,
            y + m.card_gap,
            l.pick(crate::lang::legacy(["正在启动完整模式", "正在啟動完整模式", "Starting full mode"])),
            l.pick(crate::lang::legacy(["正在渲染页面", "正在繪製頁面", "Drawing the page"])),
            Some(progress),
        );
    }
    if let Some(shot) = &s.browser.full_shot {
        let max_w = m.w as f32 - m.margin * 2.0;
        if let Some((w, h, rgba)) = decode_picture(shot, max_w as u32) {
            let x = (m.w as f32 - w as f32) / 2.0;
            y += m.card_gap;
            let picture = {
                let mut cv = Canvas::new(w, h);
                cv.data = rgba;
                cv
            };
            c.lay(&picture, x.round() as i32, y.round() as i32, 1.0);
            let (cols, rows) = (6, 12);
            for gy in 0..rows {
                for gx in 0..cols {
                    let rect = Rect::new(
                        x + gx as f32 * w as f32 / cols as f32,
                        y + gy as f32 * h as f32 / rows as f32,
                        w as f32 / cols as f32,
                        h as f32 / rows as f32,
                    );
                    let px = ((gx * 1000 + 500) / cols) as u16;
                    let py = ((gy * 1000 + 500) / rows) as u16;
                    out.push(Hit {
                        rect,
                        action: Action::BrowserPluginTap(px, py),
                    });
                }
            }
            y += h as f32;
        }
    }
    Frame::new(t, fit(c, t, y), out, l.pick(BROWSER), top)
}

/// The open pages, one row each, and a way to add or close one.
pub fn tabs(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let top = m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["标签页", "分頁", "Tabs"])));
    let titles: Vec<String> = s
        .browser
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let active = if i == s.browser.active { " ·" } else { "" };
            format!("{}{active}", tab.title(l))
        })
        .collect();
    let rows: Vec<Row> = s
        .browser
        .tabs
        .iter()
        .zip(&titles)
        .enumerate()
        .map(|(i, (tab, title))| {
            Row::new(title)
                .subtitle(if tab.url.is_empty() {
                    l.pick(crate::lang::legacy(["新标签页", "新分頁", "New tab"]))
                } else {
                    tab.url.as_str()
                })
                .icon(if i == s.browser.active { BLUE } else { t.pal.subtitle }, Icon::Layers)
        })
        .collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    let actions: Vec<Action> = (0..s.browser.tabs.len()).map(Action::BrowserPickTab).collect();
    let mut out = hits(&list.rows, &actions);
    let close = widgets::list(
        &mut c,
        t,
        f,
        list.bottom + m.card_gap,
        &[Row::new(l.pick(crate::lang::legacy(["关闭当前标签", "關閉目前分頁", "Close this tab"])))
            .icon(RED, Icon::Wipe)
            .trail(Trail::None)],
    );
    out.extend(hits(&close.rows, &[Action::BrowserCloseTab(s.browser.active)]));
    let new = widgets::list(
        &mut c,
        t,
        f,
        close.bottom + m.card_gap,
        &[Row::new(l.pick(crate::lang::legacy(["新建标签页", "新增分頁", "New tab"])))
            .icon(GREEN, Icon::Install)
            .trail(Trail::None)],
    );
    out.extend(hits(&new.rows, &[Action::BrowserNewTab]));
    Frame::new(t, fit(c, t, new.bottom), out, l.pick(crate::lang::legacy(["标签页", "分頁", "Tabs"])), top)
}

/// The start page's shortcuts, to remove one, add the page shown, or put
/// them back as they started.
pub fn quick(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let top = m.content_top;
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["快捷访问", "快速存取", "Shortcuts"])));
    let rows: Vec<Row> = s
        .browser
        .quick
        .iter()
        .map(|q| {
            Row::new(&q.title)
                .icon(q.color, q.icon)
                .subtitle(&q.url)
                .trail(Trail::Value(l.pick(crate::lang::legacy(["删除", "刪除", "Remove"]))))
                .to_owned()
        })
        .collect();
    let list = widgets::list(&mut c, t, f, y, &rows);
    let actions: Vec<Action> = (0..s.browser.quick.len()).map(Action::BrowserQuickRemove).collect();
    let mut out = hits(&list.rows, &actions);
    let add = widgets::list(
        &mut c,
        t,
        f,
        list.bottom + m.card_gap,
        &[Row::new(l.pick(crate::lang::legacy(["添加当前页面", "加入目前頁面", "Add the page shown"])))
            .icon(GREEN, Icon::Install)
            .trail(Trail::None)],
    );
    out.extend(hits(&add.rows, &[Action::BrowserQuickAdd]));
    let reset = widgets::list(
        &mut c,
        t,
        f,
        add.bottom + m.card_gap,
        &[Row::new(l.pick(crate::lang::legacy(["恢复默认", "回復預設", "Restore the defaults"])))
            .icon(BLUE, Icon::Restore)
            .trail(Trail::None)],
    );
    out.extend(hits(&reset.rows, &[Action::BrowserQuickReset]));
    Frame::new(t, fit(c, t, reset.bottom), out, l.pick(crate::lang::legacy(["快捷访问", "快速存取", "Shortcuts"])), top)
}

pub fn menu(cx: &Ctx) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = long_page(cx);
    let top = m.content_top;
    let mut out = Vec::new();
    let y = widgets::section_first(&mut c, t, f, top, l.pick(crate::lang::legacy(["浏览器菜单", "瀏覽器選單", "Browser menu"])));
    let bookmarked = cx.state.browser.bookmarks.iter().any(|b| b.url == cx.state.browser.url);
    let mut rows = vec![
        Row::new(if bookmarked {
            l.pick(["取消书签", "取消書籤", "Remove bookmark"])
        } else {
            l.pick(["添加书签", "加入書籤", "Add bookmark"])
        })
        .icon(ORANGE, Icon::Backup)
        .trail(Trail::None),
        Row::new(l.pick(["标签页", "分頁", "Tabs"])).icon(BLUE, Icon::Layers),
        Row::new(l.pick(["主页", "首頁", "Home"])).icon(BLUE, Icon::Globe),
    ];
    let mut actions = vec![Action::BrowserBookmark, Action::BrowserTabs, Action::BrowserHome];
    if cx.state.browser.full_active {
        rows.extend([
            Row::new(l.pick(crate::lang::legacy(["沉浸全屏", "沉浸全螢幕", "Immersive fullscreen"])))
                .icon(BLUE, Icon::Layers)
                .trail(Trail::Switch(cx.state.browser.immersive)),
            Row::new(if cx.state.browser.desktop_mode {
                l.pick(crate::lang::legacy(["手机模式", "手機模式", "Mobile mode"]))
            } else {
                l.pick(crate::lang::legacy(["电脑模式", "電腦模式", "Desktop mode"]))
            })
            .icon(BLUE, Icon::Globe),
            Row::new(if cx.state.browser.landscape_mode {
                l.pick(crate::lang::legacy(["退出横屏", "退出橫屏", "Leave landscape"]))
            } else {
                l.pick(crate::lang::legacy(["横屏播放", "橫屏播放", "Landscape video"]))
            })
            .icon(BLUE, Icon::Globe),
            Row::new(l.pick(crate::lang::legacy(["输入网页文字", "輸入網頁文字", "Type on page"]))).icon(BLUE, Icon::Log),
            Row::new(l.pick(crate::lang::legacy(["放大", "放大", "Zoom in"]))).icon(BLUE, Icon::Globe),
            Row::new(l.pick(crate::lang::legacy(["缩小", "縮小", "Zoom out"]))).icon(BLUE, Icon::Globe),
            Row::new(l.pick(crate::lang::legacy(["向上滚动", "向上捲動", "Scroll up"]))).icon(BLUE, Icon::Restore),
            Row::new(l.pick(crate::lang::legacy(["向下滚动", "向下捲動", "Scroll down"]))).icon(BLUE, Icon::Restore),
            Row::new(l.pick(crate::lang::legacy(["轻量模式", "輕量模式", "Basic mode"]))).icon(BLUE, Icon::Globe),
        ]);
        actions.extend([
            Action::BrowserImmersiveToggle,
            Action::BrowserDesktopToggle,
            Action::BrowserLandscapeToggle,
            Action::BrowserKeyboard,
            Action::BrowserZoomIn,
            Action::BrowserZoomOut,
            Action::BrowserScrollUp,
            Action::BrowserScrollDown,
            Action::BrowserBasic,
        ]);
    } else {
        rows.push(Row::new(l.pick(crate::lang::legacy(["在完整模式中打开", "在完整模式中開啟", "Open in full mode"]))).icon(PURPLE, Icon::Layers));
        actions.push(Action::BrowserFullLoad);
    }
    let list = widgets::list(&mut c, t, f, y, &rows);
    out.extend(hits(&list.rows, &actions));
    Frame::new(t, fit(c, t, list.bottom), out, l.pick(BROWSER), top)
}

pub fn marks(cx: &Ctx, history: bool) -> Frame {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let mut c = page(cx);
    let top = m.content_top;
    let mut out = Vec::new();
    let list = if history {
        &cx.state.browser.recent
    } else {
        &cx.state.browser.bookmarks
    };
    let title = if history {
        l.pick(crate::lang::legacy(["历史记录", "歷史記錄", "History"]))
    } else {
        l.pick(crate::lang::legacy(["书签", "書籤", "Bookmarks"]))
    };
    let mut y = widgets::section_first(&mut c, t, f, top, title);
    if list.is_empty() {
        let _ = widgets::footnote(
            &mut c,
            t,
            f,
            y,
            if history {
                l.pick(crate::lang::legacy([
                    "本次会话还没有访问记录",
                    "本次工作階段還沒有造訪記錄",
                    "Nothing has been read this session",
                ]))
            } else {
                l.pick(crate::lang::legacy(["还没有书签", "還沒有書籤", "No bookmarks yet"]))
            },
        );
    } else {
        for (i, mark) in list.iter().enumerate() {
            let row = Row::new(&mark.title)
                .subtitle(&mark.url)
                .icon(BLUE, Icon::Globe)
                .trail(Trail::Arrow);
            let rows = widgets::list(&mut c, t, f, y, &[row]);
            out.extend(hits(&rows.rows, &[Action::BrowserMarkOpen(i)]));
            y = rows.bottom;
            let row = Row::new(l.pick(crate::lang::legacy(["移除", "移除", "Remove"])))
                .icon(RED, Icon::Wipe)
                .trail(Trail::None);
            let rows = widgets::list(&mut c, t, f, y, &[row]);
            out.extend(hits(&rows.rows, &[Action::BrowserMarkRemove(i)]));
            y = rows.bottom;
        }
    }
    Frame::new(t, c, out, l.pick(BROWSER), top)
}

fn rgba(hex: u32) -> Rgba {
    Rgba::rgb(((hex >> 16) & 0xff) as u8, ((hex >> 8) & 0xff) as u8, (hex & 0xff) as u8)
}

fn aligned_x(c: &Canvas, font: &crate::font::Face, line: &str, px: f32, x: f32, w: f32, align: web::Align) -> f32 {
    let width = c.text_width(font, line, px);
    match align {
        web::Align::Center => x + (w - width).max(0.0) / 2.0,
        web::Align::Right => x + (w - width).max(0.0),
        web::Align::Left => x,
    }
}

fn form_height(c: &Canvas, t: &Theme, f: &Fonts, cx: &Ctx, i: usize) -> f32 {
    let m = &t.m;
    let Some(form) = cx.state.browser.page().and_then(|p| p.forms.get(i)) else {
        return m.dp(40.0);
    };
    let mut h = m.dp(30.0);
    for field in &form.fields {
        let _ = (c, f);
        h += match field.kind {
            web::FieldKind::Hidden => 0.0,
            _ => m.dp(48.0),
        };
    }
    h + m.dp(10.0)
}

fn draw_form(c: &mut Canvas, cx: &Ctx, i: usize) {
    let (t, f, l) = (cx.t, cx.f, cx.state.lang);
    let m = &t.m;
    let Some(form) = cx.state.browser.page().and_then(|p| p.forms.get(i)) else {
        return;
    };
    let mut y = m.dp(6.0);
    c.fill_round_rect(
        m.margin,
        y,
        m.w as f32 - m.margin * 2.0,
        form_height(c, t, f, cx, i),
        m.card_radius,
        t.pal.card,
    );
    y += m.dp(12.0);
    c.text_top(
        &f.label,
        l.pick(crate::lang::legacy(["表单", "表單", "Form"])),
        m.title_x,
        y,
        m.sub_px,
        t.pal.accent,
        None,
    );
    y += m.dp(26.0);
    for (j, field) in form.fields.iter().enumerate() {
        if field.kind == web::FieldKind::Hidden {
            continue;
        }
        let value = cx
            .state
            .browser
            .form
            .get(i)
            .and_then(|v| v.get(j))
            .cloned()
            .unwrap_or_else(|| field.value.clone());
        let label = if field.label.is_empty() { &field.name } else { &field.label };
        let (title, trail_value): (String, Option<String>) = match field.kind {
            web::FieldKind::Submit => (
                if label.is_empty() {
                    l.pick(crate::lang::legacy(["提交", "提交", "Submit"])).to_string()
                } else {
                    label.clone()
                },
                None,
            ),
            web::FieldKind::Checkbox | web::FieldKind::Radio => (
                label.clone(),
                Some(if value.is_empty() { "[ ]".to_string() } else { "[x]".to_string() }),
            ),
            _ => (
                label.clone(),
                Some(if value.is_empty() {
                    l.pick(crate::lang::legacy(["点按填写", "點按填寫", "Tap to fill"])).to_string()
                } else {
                    value
                }),
            ),
        };
        let trail = match &trail_value {
            Some(value) => Trail::Value(value.as_str()),
            None => Trail::Arrow,
        };
        let row = Row::new(&title).icon(BLUE, Icon::Log).trail(trail);
        let list = widgets::list(c, t, f, y, &[row]);
        y = list.bottom + m.dp(4.0);
    }
}

fn draw_picture(c: &mut Canvas, cx: &Ctx, block: &WebBlock) {
    let (t, l) = (cx.t, cx.state.lang);
    let m = &t.m;
    let WebBlock::Image {
        alt, data: Some(bytes), ..
    } = block
    else {
        return;
    };
    let max_w = (m.w as f32 - m.margin * 2.0).max(m.dp(80.0)) as u32;
    match decode_picture(bytes, max_w) {
        Some((w, h, rgba)) => {
            let x = (m.w as f32 - w as f32) / 2.0;
            let y = m.dp(8.0);
            let picture = {
                let mut cv = Canvas::new(w, h);
                cv.data = rgba;
                cv
            };
            c.lay(&picture, x.round() as i32, y.round() as i32, 1.0);
        }
        None => {
            let row = Row::new(l.pick(crate::lang::legacy(["图片", "圖片", "Picture"])))
                .icon(PURPLE, Icon::Layers)
                .subtitle(if alt.is_empty() {
                    l.pick(crate::lang::legacy(["无法显示这张图片", "無法顯示這張圖片", "This picture can't be shown"]))
                } else {
                    alt
                })
                .trail(Trail::None);
            let _ = widgets::list(c, t, cx.f, m.dp(6.0), &[row]);
        }
    }
}

/// Decode a picture's bytes into RGBA, no wider than `max_w`.
fn decode_picture(bytes: &[u8], max_w: u32) -> Option<(u32, u32, Vec<u8>)> {
    if let Some(picture) = decode_png(bytes) {
        return scale_picture(picture, max_w);
    }
    decode_jpeg(bytes).and_then(|picture| scale_picture(picture, max_w))
}

fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 8_000_000 {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    if w == 0 || h == 0 || w as u64 * h as u64 > 16_000_000 {
        return None;
    }
    let bpp = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for pixel in buf.chunks_exact(bpp) {
        match bpp {
            4 => rgba.extend_from_slice(pixel),
            3 => {
                rgba.extend_from_slice(pixel);
                rgba.push(255);
            }
            2 => {
                let g = pixel[0];
                rgba.extend_from_slice(&[g, g, g, pixel[1]]);
            }
            _ => {
                let g = pixel[0];
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
    }
    Some((w, h, rgba))
}

fn decode_jpeg(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    if w == 0 || h == 0 || w as u64 * h as u64 > 8_000_000 {
        return None;
    }
    let pixels = decoder.decode().ok()?;
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            for g in pixels {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
        jpeg_decoder::PixelFormat::RGB24 => {
            for p in pixels.as_chunks::<3>().0 {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            for p in pixels.as_chunks::<4>().0 {
                let k = u16::from(p[3]);
                let c = (u16::from(p[0]) * k) / 255;
                let m = (u16::from(p[1]) * k) / 255;
                let y = (u16::from(p[2]) * k) / 255;
                rgba.extend_from_slice(&[c as u8, m as u8, y as u8, 255]);
            }
        }
        _ => return None,
    }
    Some((w, h, rgba))
}

fn scale_picture((w, h, rgba): (u32, u32, Vec<u8>), max_w: u32) -> Option<(u32, u32, Vec<u8>)> {
    if w <= max_w {
        return Some((w, h, rgba));
    }
    resize_picture((w, h, rgba), max_w)
}

fn resize_picture((w, h, rgba): (u32, u32, Vec<u8>), max_w: u32) -> Option<(u32, u32, Vec<u8>)> {
    let nw = max_w.max(1);
    let nh = ((h as u64 * nw as u64) / w as u64).max(1) as u32;
    let mut scaled = Vec::with_capacity(nw as usize * nh as usize * 4);
    for y in 0..nh {
        let sy = (y as u64 * h as u64 / nh as u64) as u32;
        for x in 0..nw {
            let sx = (x as u64 * w as u64 / nw as u64) as u32;
            let at = ((sy as usize * w as usize) + sx as usize) * 4;
            scaled.extend_from_slice(&rgba[at..at + 4]);
        }
    }
    Some((nw, nh, scaled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paragraph_is_wrapped_to_the_content_width() {
        let t = Theme::light(1200, 3200, 3.0);
        let f = crate::font::bundled();
        let c = Canvas::new(1200, 3200);
        let w = t.m.w as f32 - t.m.title_x * 2.0;
        let text = "完整镜像，解压后按下面的命令刷入 recovery 分区。";
        let lines = widgets::wrap(&c, &f.body, text, t.m.dp(15.0), w);
        for line in &lines {
            assert!(
                c.text_width(&f.body, line, t.m.dp(15.0)) <= w + t.m.dp(15.0),
                "{line:?} is wider than the page and a hanging punctuation mark: {lines:?}"
            );
        }
        assert!(!lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn a_link_row_is_as_tall_as_the_recovery_draws_it() {
        let t = Theme::light(1440, 3200, 3.5);
        let f = crate::font::bundled();
        let c = Canvas::new(1440, 3200);
        let h = link_height(
            &c,
            &t,
            &f,
            Lang::Hans,
            "下载",
            "https://example.com/a/very/long/address/for/a/file.zip",
            true,
        );
        assert!(h > t.m.row_h, "a link with its host under it is taller than a row: {h}");
    }
}

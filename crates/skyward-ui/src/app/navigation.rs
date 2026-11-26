use super::*;

pub const NAV_DP: f32 = 24.0;
const HOLD_MS: u64 = 300;
const MAX_TASKS: usize = 12;

#[derive(Clone, Copy)]
pub(super) struct NavGesture {
    start: (f32, f32),
    last: (f32, f32),
    still: u64,
    overview: bool,
}

#[derive(Clone, Copy)]
pub(super) struct BrowserEdge {
    x: f32,
    y: f32,
}

pub(super) struct ParkedTask {
    kind: Screen,
    stack: Vec<(Screen, f32, Focus)>,
    picker: Option<(bool, bool)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskCard {
    pub screen: Screen,
    pub title: &'static str,
    pub preview: Option<BrowserPicture>,
}

pub fn task_kind(screen: Screen) -> Screen {
    match screen {
        Screen::Browser
        | Screen::BrowserAddress
        | Screen::BrowserForm
        | Screen::BrowserFull
        | Screen::BrowserTabs
        | Screen::BrowserQuick
        | Screen::BrowserMenu
        | Screen::BrowserHistory
        | Screen::BrowserBookmarks => Screen::Browser,
        Screen::Files
        | Screen::FileInfo
        | Screen::Rename
        | Screen::NewFolder
        | Screen::Permissions
        | Screen::Editor
        | Screen::FileJob
        | Screen::Clash => Screen::Files,
        Screen::Install | Screen::Installing | Screen::Choose(_) => Screen::Install,
        Screen::Backup | Screen::ImageBackup | Screen::BackupJob => Screen::Backup,
        Screen::Restore | Screen::RestoreBackup => Screen::Restore,
        Screen::RootModules | Screen::KernelSu | Screen::RootModuleInstall => Screen::RootModules,
        Screen::Settings
        | Screen::About
        | Screen::Specs
        | Screen::Language
        | Screen::ScreenOff
        | Screen::TimeZone
        | Screen::TimeZones(_)
        | Screen::License
        | Screen::FontLicense => Screen::Settings,
        other => other,
    }
}

pub fn task_tiles(t: &Theme, count: usize) -> Vec<Rect> {
    let gap = t.m.dp(18.0);
    let margin = t.m.dp(18.0);
    let width = (t.m.w as f32 - margin * 2.0 - gap) / 2.0;
    let height = width * t.m.h as f32 / t.m.w.max(1) as f32 + t.m.dp(40.0);
    (0..count)
        .map(|i| {
            Rect::new(
                margin + (i % 2) as f32 * (width + gap),
                t.m.dp(96.0) + (i / 2) as f32 * (height + t.m.dp(30.0)),
                width,
                height,
            )
        })
        .collect()
}

fn thumbnail(source: &Canvas) -> BrowserPicture {
    let width = 360u32;
    let height = ((source.h as u64 * u64::from(width)) / u64::from(source.w.max(1))) as u32;
    let mut out = Canvas::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let from = (((y * source.h / height.max(1)) * source.w + x * source.w / width) * 4) as usize;
            let to = ((y * width + x) * 4) as usize;
            if let Some(pixel) = source.data.get(from..from + 4) {
                out.data[to..to + 4].copy_from_slice(pixel);
            }
        }
    }
    picture(out)
}

fn picture(mut canvas: Canvas) -> BrowserPicture {
    BrowserPicture {
        width: canvas.w,
        height: canvas.h,
        pixels: std::sync::Arc::new(std::mem::take(&mut canvas.data)),
    }
}

pub fn task_preview_rect(t: &Theme, tile: Rect) -> Rect {
    Rect::new(tile.x, tile.y + t.m.dp(40.0), tile.w, tile.h - t.m.dp(40.0))
}

impl App {
    pub(super) fn nav_height(&self) -> u32 {
        if !self.navigation || self.screen() == Screen::Fastboot || self.browser_immersive() {
            0
        } else {
            (self.px_per_dp * NAV_DP).round() as u32
        }
    }

    pub fn set_navigation(&mut self, enabled: bool) {
        self.navigation = enabled;
        self.nav_gesture = None;
        self.settle_page();
        self.refresh();
    }

    pub(super) fn park_task(&mut self) {
        if matches!(
            self.screen(),
            Screen::Home
                | Screen::Tasks
                | Screen::ControlCenter
                | Screen::Fastboot
                | Screen::Decrypt
                | Screen::WlanPassword
                | Screen::WlanAdd
        ) {
            return;
        }
        let kind = task_kind(self.screen());
        let title = self.frame.as_ref().map_or("Skyward", |frame| frame.title);
        let title = if title.is_empty() {
            self.state.lang.pick(["文件管理", "檔案管理", "Files"])
        } else {
            title
        };
        self.leave(false);
        let preview = self.turn.take().map(|turn| thumbnail(&turn.left));
        if let Some(at) = self.tasks.iter().position(|task| task.kind == kind) {
            self.tasks.remove(at);
            self.state.task_cards.remove(at);
        }
        let mut stack = self.stack.clone();
        if stack.first().is_none_or(|(screen, ..)| *screen != Screen::Home) {
            stack.insert(0, (Screen::Home, 0.0, Focus::Unset));
        }
        let picker = matches!(self.screen(), Screen::Choose(_)).then_some((self.state.image_only, self.state.module_picker));
        self.tasks.insert(0, ParkedTask { kind, stack, picker });
        self.state.task_cards.insert(
            0,
            TaskCard {
                screen: kind,
                title,
                preview,
            },
        );
        self.tasks.truncate(MAX_TASKS);
        self.state.task_cards.truncate(MAX_TASKS);
    }

    fn navigation_page(&mut self, screen: Screen) {
        self.state.ime.clear();
        self.state.ime.enabled = false;
        self.state.browser.ime.clear();
        self.state.browser.full_text = false;
        self.end_turn();
        self.morph = None;
        self.row_press = None;
        self.row_art = None;
        self.stack = vec![(Screen::Home, 0.0, Focus::Unset)];
        if screen != Screen::Home {
            self.stack.push((screen, 0.0, Focus::Unset));
        }
        self.settle_page();
        self.refresh();
    }

    pub(super) fn home(&mut self) {
        if self.shade.is_some() {
            self.dismiss_shade();
        }
        self.park_task();
        self.navigation_page(Screen::Home);
    }

    pub(super) fn overview(&mut self) {
        if self.shade.is_some() {
            self.dismiss_shade();
        }
        self.park_task();
        if let Some(preview) = self.state.task_cards.first().and_then(|task| task.preview.as_ref()) {
            let mut source = Canvas::new(preview.width, preview.height);
            source.blit_rgba(&preview.pixels, preview.width, preview.height, 0, 0);
            self.state.task_background = Some(picture(shade::frosted(&source)));
        }
        self.navigation_page(Screen::Tasks);
    }

    pub(super) fn resume(&mut self) {
        self.resume_task(0);
    }

    pub(super) fn close_task(&mut self, kind: Screen) {
        if self.screen() != Screen::Tasks {
            return;
        }
        let Some(at) = self.tasks.iter().position(|task| task.kind == kind) else {
            return;
        };
        self.tasks.remove(at);
        self.state.task_cards.retain(|task| task.screen != kind);
        self.end_turn();
        if self.tasks.is_empty() {
            self.state.task_background = None;
            self.navigation_page(Screen::Home);
        } else {
            self.settle_page();
            self.refresh();
        }
    }

    pub(super) fn resume_task(&mut self, at: usize) {
        let Some(task) = self.tasks.get(at) else { return };
        if let Some((image_only, module_picker)) = task.picker {
            self.state.image_only = image_only;
            self.state.module_picker = module_picker;
        }
        let mut stack = task.stack.clone();
        if let Some(invalid) = stack.iter().position(|(screen, ..)| !self.task_page_valid(*screen)) {
            stack.truncate(invalid);
        }
        if stack.is_empty() {
            stack.push((Screen::Home, 0.0, Focus::Unset));
        }
        self.end_turn();
        self.stack = stack;
        self.morph = None;
        self.settle_page();
        self.refresh();
    }

    fn task_page_valid(&self, screen: Screen) -> bool {
        let s = &self.state;
        match screen {
            Screen::FileInfo | Screen::Rename | Screen::Permissions => s.file.is_some(),
            Screen::Editor => s.editor.is_some(),
            Screen::Clash => s.clash.is_some(),
            Screen::FileJob => s.job.is_some(),
            Screen::RestoreBackup => s.restore.is_some(),
            Screen::BackupJob => s.backup_job.is_some(),
            Screen::PartWork => s.part_work.is_some(),
            Screen::Relabel => s.relabel.is_some(),
            Screen::Wlan => s.wlan.is_some(),
            Screen::WlanNetwork => s.details.is_some(),
            Screen::WlanPassword | Screen::WlanAdd | Screen::Decrypt => false,
            Screen::BrowserForm => s
                .browser
                .field
                .is_some_and(|(form, field)| s.browser.form.get(form).is_some_and(|values| field < values.len())),
            _ => true,
        }
    }

    pub(super) fn navigation_deadline(&self) -> Option<u64> {
        self.nav_gesture
            .filter(|g| {
                !g.overview && g.start.1 - g.last.1 >= self.px_per_dp * 52.0 && (g.last.0 - g.start.0).abs() < self.px_per_dp * 48.0
            })
            .map(|g| g.still + HOLD_MS)
    }

    pub(super) fn tick_navigation(&mut self, now: u64) -> Vec<Effect> {
        if self.navigation_deadline().is_some_and(|at| now >= at) {
            if let Some(gesture) = &mut self.nav_gesture {
                gesture.overview = true;
            }
            self.overview();
            if self.state.vibrate {
                return vec![Effect::Haptic];
            }
        }
        Vec::new()
    }

    pub(super) fn navigation_touch(&mut self, event: Touch, now: u64) -> Option<Vec<Effect>> {
        if !self.navigation || self.screen() == Screen::Fastboot || self.browser_immersive() {
            return None;
        }
        if let Touch::Down(x, y) = event
            && y >= self.h as f32 - self.px_per_dp * NAV_DP
        {
            self.browser_edge = None;
            self.nav_gesture = Some(NavGesture {
                start: (x, y),
                last: (x, y),
                still: now,
                overview: false,
            });
            self.drag = Drag::Idle;
            self.stir(now);
            self.dirty = true;
            return Some(Vec::new());
        }
        let mut gesture = self.nav_gesture?;
        self.stir(now);
        match event {
            Touch::Move(x, y) => {
                if (x - gesture.last.0).abs().max((y - gesture.last.1).abs()) >= self.px_per_dp * 0.5 {
                    gesture.still = now;
                }
                gesture.last = (x, y);
                self.nav_gesture = Some(gesture);
                self.dirty = true;
                Some(self.tick_navigation(now))
            }
            Touch::Up => {
                let mut effects = self.tick_navigation(now);
                let overview = self.nav_gesture.is_some_and(|g| g.overview);
                self.nav_gesture = None;
                self.drag = Drag::Idle;
                self.dirty = true;
                if !overview
                    && gesture.start.1 - gesture.last.1 >= self.px_per_dp * 52.0
                    && (gesture.last.0 - gesture.start.0).abs() < self.px_per_dp * 48.0
                {
                    self.home();
                    if self.state.vibrate {
                        effects.push(Effect::Haptic);
                    }
                }
                Some(effects)
            }
            _ => Some(Vec::new()),
        }
    }

    pub(super) fn draw_navigation(&self, out: &mut Canvas) {
        let height = self.nav_height();
        if height == 0 {
            return;
        }
        let t = self.theme();
        let width = t.m.dp(108.0);
        let sample = ((self.h.saturating_sub(10) * out.w + out.w / 2) * 4) as usize;
        let p = &out.data[sample..sample + 3];
        let light = u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114;
        let ink = if light > 128000 {
            Rgba::rgb(25, 25, 25)
        } else {
            Rgba::rgb(237, 237, 237)
        };
        out.fill_round_rect(
            (self.w as f32 - width) / 2.0,
            self.h as f32 - t.m.dp(10.0),
            width,
            t.m.dp(4.0),
            t.m.dp(2.0),
            ink,
        );
    }
}

impl App {
    pub(super) fn browser_edge_touch(&mut self, event: Touch, now: u64) -> Option<Vec<Effect>> {
        if self.browser_immersive() || !matches!(self.screen(), Screen::Browser | Screen::BrowserAddress) || !self.state.browser.full_active
        {
            self.browser_edge = None;
            return None;
        }
        let m = self.theme().m;
        if let Touch::Down(x, y) = event {
            let edge = x < m.dp(20.0) || x > self.w as f32 - m.dp(20.0);
            if edge
                && (self.stack.len() > 1 || self.state.browser.at > 0)
                && self
                    .frame
                    .as_ref()
                    .and_then(|frame| frame.browser_area)
                    .is_some_and(|area| area.contains(x, y))
            {
                self.browser_edge = Some(BrowserEdge { x, y });
                self.drag = Drag::Idle;
                self.stir(now);
                return Some(Vec::new());
            }
        }
        let start = self.browser_edge?;
        self.stir(now);
        match event {
            Touch::Move(x, y) => {
                let from_left = start.x < self.w as f32 / 2.0;
                let travel = if from_left { x - start.x } else { start.x - x };
                let dy = y - start.y;
                if travel > m.dp(8.0) && travel > dy.abs() {
                    self.browser_edge = None;
                    let armed = travel > m.dp(BACK_SWIPE_DP);
                    self.drag = Drag::Edge {
                        start: start.x,
                        from_left,
                        armed,
                        x,
                        cy: start.y - m.dp(CUE_LIFT_DP),
                    };
                    self.dirty = true;
                    return Some(if armed && self.state.vibrate {
                        vec![Effect::Haptic]
                    } else {
                        Vec::new()
                    });
                }
                if dy.abs() > m.dp(8.0) {
                    self.browser_edge = None;
                    let rect = self.browser_viewport();
                    let (sx, sy) = input::browser_point(rect, start.x, start.y);
                    let (px, py) = input::browser_point(rect, x, y);
                    self.drag = Drag::BrowserTouch { rect, last: (px, py) };
                    return Some(vec![
                        Effect::BrowserPluginTouch { phase: 0, x: sx, y: sy },
                        Effect::BrowserPluginTouch { phase: 1, x: px, y: py },
                    ]);
                }
                Some(Vec::new())
            }
            Touch::Up => {
                self.browser_edge = None;
                let (x, y) = input::browser_point(self.browser_viewport(), start.x, start.y);
                Some(vec![Effect::BrowserPluginTap { x, y }])
            }
            _ => Some(Vec::new()),
        }
    }
}

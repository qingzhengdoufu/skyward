use super::*;

impl App {
    /// What the status bar shows of the WLAN: the network joined, while it
    /// has an address.
    /// What the status bar shows, its clock reading `clock`: the battery,
    /// the network joined, the data partition's lock, the slot the system
    /// runs from, and the battery's temperature.
    pub fn bar<'a>(&self, clock: &'a str, info: &DeviceInfo) -> Bar<'a> {
        let battery = info.battery.as_ref();
        Bar {
            clock,
            battery: battery.map(|b| (b.level, b.charging().then_some(if b.quick { Bolt::Quick } else { Bolt::Charging }))),
            wlan: self.wlan_mark(),
            data: match self.state.data {
                Data::Erased => None,
                Data::Open => Some(true),
                Data::Locked { user, .. } => Some(user != 0),
                Data::Closed | Data::Opening | Data::Failed(_) => Some(false),
            },
            slot: info.slot.map(Slot::letter),
            temp: battery.and_then(|b| b.temp),
        }
    }
}

impl App {
    /// Render the current page if what it shows has changed: the rows of
    /// it on screen, the rest as they come into view; or, where only some
    /// rows changed, those rows.
    pub fn frame(&mut self, f: &Fonts, info: &DeviceInfo) -> &Frame {
        self.render(f, info, self.w)
    }
}

impl App {
    /// [`App::frame`], drawing of a page rendered anew only its columns
    /// from the left edge to `wide`.
    pub(super) fn render(&mut self, f: &Fonts, info: &DeviceInfo, wide: u32) -> &Frame {
        if self.frame.is_some() && !self.stale.is_empty() {
            let t = self.theme();
            let screen = self.screen();
            for rows in std::mem::take(&mut self.stale) {
                let band = Band {
                    rows,
                    cols: (0, self.cols),
                };
                let cx = Ctx {
                    t: &t,
                    f,
                    info,
                    state: &self.state,
                    term: self.term.as_ref(),
                    band: Some(band),
                };
                let fresh = pages::render(screen, &cx);
                let mut canvas = self.frame.take().expect("checked above").canvas;
                assert_eq!(canvas.h, fresh.canvas.h, "a page drawn again in part is as tall as it was");
                canvas.replace(&fresh.canvas);
                self.frame = Some(Frame { canvas, ..fresh });
            }
        }
        if self.frame.is_none() {
            let t = self.theme();
            let screen = self.screen();
            let first = self.top().1.max(0.0) as u32;
            let band = Band {
                rows: (first, first + self.h),
                cols: (0, wide.min(self.w)),
            };
            let cx = Ctx {
                t: &t,
                f,
                info,
                state: &self.state,
                term: self.term.as_ref(),
                band: Some(band),
            };
            let mut frame = pages::render(screen, &cx);
            self.cols = frame.canvas.cols().end;
            if self.cols < self.w {
                let mut whole = Canvas::unfilled(self.w, frame.canvas.h, frame.canvas.held());
                whole.replace(&frame.canvas);
                frame.canvas = whole;
            }
            self.height = frame.height(&t) + if self.browser_fullscreen() { 0.0 } else { self.nav_height() as f32 };
            self.frame = Some(frame);
            self.clamp_scroll();
            if std::mem::take(&mut self.snap) {
                let (title, bar) = self.bar_wanted().expect("rendered above");
                self.title.jump(title);
                self.bar.jump(bar);
            }
        }
        self.frame.as_ref().expect("rendered above")
    }
}

impl App {
    /// The current page rendered, all of it drawn.
    pub fn whole_frame(&mut self, f: &Fonts, info: &DeviceInfo) -> &Frame {
        let h = self.frame(f, info).canvas.h;
        self.draw_cols(f, info, self.w);
        self.draw_rows(f, info, 0.0, h as f32);
        self.frame.as_ref().expect("rendered above")
    }
}

impl App {
    /// Draw a part of the current page not drawn yet: the rest of the
    /// columns of its rows drawn, then a band of rows below those or else
    /// above them, for a page at rest to have all of it drawn by the time
    /// it scrolls into view; that done, make the About page field's dither,
    /// for that page to open without making it. Whether any is left to do.
    pub fn draw_ahead(&mut self, f: &Fonts, info: &DeviceInfo) -> bool {
        let Some(frame) = &self.frame else { return false };
        let (held, h) = (frame.canvas.held(), frame.canvas.h);
        if self.cols < self.w {
            self.draw_cols(f, info, self.w);
        } else if held.end < h {
            self.draw_rows(f, info, held.end as f32, (held.end + 1) as f32);
        } else if held.start > 0 {
            self.draw_rows(f, info, (held.start - 1) as f32, held.start as f32);
        } else if self.dither.is_none() {
            self.dither = Some(glow::Dither::new(self.w as usize, self.h as usize));
        }
        self.cols < self.w || self.frame.as_ref().is_some_and(|fr| !fr.canvas.whole()) || self.dither.is_none()
    }
}

impl App {
    /// Draw the columns of the current page's rows drawn from where they
    /// were drawn to, on to `wide`.
    pub(super) fn draw_cols(&mut self, f: &Fonts, info: &DeviceInfo, wide: u32) {
        let wide = wide.min(self.w);
        let Some(frame) = &self.frame else { return };
        if self.cols >= wide {
            return;
        }
        let held = frame.canvas.held();
        let t = self.theme();
        let band = Band {
            rows: (held.start, held.end),
            cols: (self.cols, wide),
        };
        let cx = Ctx {
            t: &t,
            f,
            info,
            state: &self.state,
            term: self.term.as_ref(),
            band: Some(band),
        };
        let part = pages::render(self.screen(), &cx).canvas;
        self.frame.as_mut().expect("drawn above").canvas.replace(&part);
        self.cols = wide;
        self.layer = None;
    }
}

impl App {
    /// Draw the rows `from..to` of the current page it has not drawn yet,
    /// a band at least a quarter of a screen tall at a time; the rows drawn
    /// are drawn across first.
    pub(super) fn draw_rows(&mut self, f: &Fonts, info: &DeviceInfo, from: f32, to: f32) {
        let Some(frame) = &self.frame else { return };
        let h = frame.canvas.h;
        let (from, to) = ((from.max(0.0) as u32).min(h), (to.max(0.0).ceil() as u32).min(h));
        let held = frame.canvas.held();
        let least = self.h / 4;
        let mut bands = Vec::new();
        if from < held.start {
            bands.push((from.min(held.start.saturating_sub(least)), held.start));
        }
        if to > held.end {
            bands.push((held.end, to.max(held.end + least).min(h)));
        }
        if bands.is_empty() {
            return;
        }
        self.draw_cols(f, info, self.w);
        let t = self.theme();
        let screen = self.screen();
        for rows in bands {
            let band = Band { rows, cols: (0, self.w) };
            let cx = Ctx {
                t: &t,
                f,
                info,
                state: &self.state,
                term: self.term.as_ref(),
                band: Some(band),
            };
            let part = pages::render(screen, &cx).canvas;
            self.frame.as_mut().expect("drawn above").canvas.join(&part);
        }
        self.layer = None;
    }
}

impl App {
    pub(super) fn max_scroll(&self) -> f32 {
        (self.height - self.h as f32).max(0.0)
    }
}

impl App {
    pub(super) fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        let top = self.top_mut();
        top.1 = top.1.clamp(0.0, max);
    }
}

impl App {
    /// Draw what the panel shows now: the page, the charging screen over it
    /// while a cable has just gone in, both as dark as the content stands
    /// as the panel goes off or comes up.
    pub fn compose(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, now: u64) {
        if self.screen() == Screen::ControlCenter && self.state.control_clock != clock {
            self.state.control_clock = clock.into();
            if let Ok(time) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                self.state.control_date = fmt::date(self.state.lang, time.as_secs() as i64, self.state.zone());
            }
            self.refresh();
        }
        let charge = self.charging.as_ref().map_or(0.0, |c| c.shown.level(now));
        if charge < 1.0 {
            if self.shade.is_some() && self.screen() == Screen::ControlCenter {
                let mut sheet = Canvas::new(out.w, out.h);
                self.compose_page(&mut sheet, f, info, clock, now);
                let shade = self.shade.as_ref().expect("shade remains while drawing");
                let extent = shade.extent(now, self.h as f32);
                out.put(&shade.under, 0);
                out.dim(1.0 - 0.28 * extent / self.h as f32);
                let rows = extent.round() as usize;
                out.put_rows(&shade.frost, 0, rows as i32, 0);
                let bytes = rows.min(out.h as usize) * out.w as usize * 4;
                for (dst, src) in out.data[..bytes]
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(sheet.data.as_chunks::<4>().0)
                {
                    crate::color::blend_px(
                        dst,
                        Rgba {
                            r: src[0],
                            g: src[1],
                            b: src[2],
                            a: src[3],
                        },
                        src[3] as u32,
                    );
                }
                let t = self.theme();
                if extent < widgets::bar_bottom(&t) {
                    widgets::status_bar(out, &t, f, &self.bar(clock, info), t.pal.status);
                } else {
                    widgets::status_marks(out, &t, f, &self.bar(clock, info), t.pal.status);
                }
            } else {
                self.compose_page(out, f, info, clock, now);
            }
        }
        if charge > 0.0 {
            self.compose_charging(out, f, info, charge, now);
        }
        if charge < 1.0 {
            self.draw_navigation(out);
        }
        out.dim(self.curtain.level(now));
    }
}

impl App {
    /// The charging screen, `level` of the way over the page drawn on `out`.
    pub(super) fn compose_charging(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, level: f32, now: u64) {
        let battery = info.battery.as_ref().expect("a cable went in on a phone whose battery is read");
        let (t, lang) = (self.theme(), self.state.lang);
        if !self.charge_art.as_ref().is_some_and(|a| a.shows(lang, battery)) {
            self.charge_art = Some(charge::Art::new(f, &t.m, lang, battery).expect("the screens' face takes the charging screen's weight"));
        }
        let since = self.charging.as_ref().map_or(now, |c| c.since);
        let dither = self.dither.get_or_insert_with(|| glow::Dither::new(out.w as usize, out.h as usize));
        let art = self.charge_art.as_ref().expect("drawn above");
        if level >= 1.0 {
            charge::draw(out, art, &t.m, battery.level, now.saturating_sub(since), dither);
        } else {
            let screen = self.charge_canvas.get_or_insert_with(|| Canvas::new(out.w, out.h));
            charge::draw(screen, art, &t.m, battery.level, now.saturating_sub(since), dither);
            out.lay(screen, 0, 0, level);
        }
    }
}

impl App {
    /// The visible part of the page, pulled past an end if it is, its large
    /// title fading as it scrolls, the bar and its title over a scrolled
    /// page, the `<`, the page sliding in or out over or under the page
    /// left, the status bar, any message, and the back cue while a swipe
    /// from an edge is under way.
    pub(super) fn compose_page(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, now: u64) {
        let opening = self.turn.as_ref().is_some_and(|t| !t.back);
        self.render(f, info, if opening { 0 } else { self.w });
        let screen = self.screen();
        let scroll = self.top().1;
        let offset = (scroll + self.overscroll(now)).round();
        let caret_on = match self.frame.as_ref().is_some_and(|fr| fr.caret.is_some()) {
            true => self.caret_up(screen, now),
            false => {
                self.blink = None;
                self.caret_shown = false;
                false
            }
        };
        if let Some(m) = &self.morph {
            if !m.at(now).1 {
                return self.compose_morph(out, f, info, clock, now, caret_on);
            }
            self.morph = None;
        }
        let t = self.theme();
        if let Some((which, track)) = self.knob.take() {
            self.paint_knob(&t, f, which, track);
        }
        if self.toast.as_ref().is_some_and(|m| now >= m.1) {
            self.toast = None;
        }
        let title = self.frame.as_ref().expect("rendered above").title;
        let key = (title, self.state.dark);
        if self.titles.as_ref().is_none_or(|(k, _, _)| *k != key) {
            self.titles = Some((
                key,
                Blurs::new(widgets::big_title(&t, f, title)),
                Blurs::new(widgets::small_title(&t, f, title)),
            ));
        }
        if let Some(fold) = self.frame.as_ref().and_then(|fr| fr.fold) {
            let key = (fold.hint, self.state.dark);
            if self.search_words.as_ref().is_none_or(|(k, _)| *k != key) {
                self.search_words = Some((key, widgets::search_words(&t, f, fold.bar, fold.hint)));
            }
        }
        if self.chip.as_ref().is_none_or(|c| c.0 != t.dark) {
            let (art, x, y) = widgets::back_chip(&t);
            self.chip = Some((t.dark, art, x, y));
        }
        let button = self.frame.as_ref().is_some_and(|fr| fr.button.is_some());
        if button && self.button_chip.as_ref().is_none_or(|c| c.0 != t.dark) {
            let (art, x, y) = widgets::button_chip(&t);
            self.button_chip = Some((t.dark, art, x, y));
        }
        let slide = self.slid(now);
        if self.frame.as_ref().is_some_and(|fr| fr.hero.is_none()) {
            let wide = match (slide, &self.turn) {
                (Some((at, _, _)), Some(turn)) if !turn.back && self.row_press.is_none() => {
                    (self.w - at.clamp(0, self.w as i32) as u32).max(self.cols + self.w / 4)
                }
                _ => self.w,
            };
            self.draw_cols(f, info, wide);
            self.draw_rows(f, info, offset, offset + self.h as f32);
        }
        if let (Some(press), None) = (self.row_press.filter(|p| now >= p.rise), &self.row_art) {
            let page = &self.frame.as_ref().expect("rendered above").canvas;
            let (x, y, w, h) = row_part(press.rect, page);
            let pressed = t.pressed();
            let band = Band {
                rows: (y, y + h),
                cols: (x, x + w),
            };
            let cx = Ctx {
                t: &pressed,
                f,
                info,
                state: &self.state,
                term: self.term.as_ref(),
                band: Some(band),
            };
            let down = pages::render(screen, &cx).canvas;
            assert_eq!((down.w, down.h), (page.w, page.h), "a page held down is laid out as it was");
            self.row_art = Some((page.crop(x, y, w, h), down.crop(x, y, w, h)));
        }
        let view = View {
            scroll,
            offset,
            bar: self.bar.level(now),
            title: self.title.level(now),
            light: self.light(offset),
            caret: caret_on,
            row: self.row_press.map_or(0.0, |p| p.level(now)),
            pressing: self.pressing(),
        };
        match slide {
            Some((at, shift, dim)) => {
                let back = self.turn.as_ref().expect("sliding").back;
                if !back && at >= self.w as i32 {
                    let left = &self.turn.as_ref().expect("sliding").left;
                    out.slide(left, left, at, shift, dim, self.corners);
                } else {
                    let still = self.frame.as_ref().is_some_and(|fr| fr.hero.is_none());
                    let page = match self.layer.take() {
                        Some((shown, c)) if still && shown == view => c,
                        kept => {
                            let mut c = kept.map(|(_, c)| c).unwrap_or_else(|| Canvas::new(out.w, out.h));
                            self.paint_page(&mut c, &t, Some((f, info)), &view);
                            c
                        }
                    };
                    let turn = self.turn.as_ref().expect("sliding");
                    let (front, behind) = if back { (&turn.left, &page) } else { (&page, &turn.left) };
                    out.slide(front, behind, at, shift, dim, self.corners);
                    self.layer = Some((view, page));
                }
            }
            None => {
                self.layer = None;
                self.paint_page(out, &t, Some((f, info)), &view);
            }
        }
        self.shown = Some(view);
        if screen != Screen::ControlCenter && !self.browser_immersive() {
            if self.browser_fullscreen() {
                out.fill_rect(0.0, 0.0, self.w as f32, t.m.dp(48.0), Rgba::rgb(0, 0, 0).with_alpha(145));
            }
            widgets::status_bar(out, &t, f, &self.bar(clock, info), t.pal.status);
        } else if screen == Screen::ControlCenter && self.shade.is_none() {
            widgets::status_marks(out, &t, f, &self.bar(clock, info), t.pal.status);
        }
        if let Some((text, _)) = &self.toast {
            widgets::toast(out, &t, f, text);
        }
        match self.drag {
            Drag::Edge {
                start,
                from_left,
                armed,
                x,
                cy,
            } => {
                let travel = if from_left { x - start } else { start - x };
                widgets::back_cue(out, &t, from_left, cy, widgets::back_cue_depth(&t, travel), armed);
            }
            _ => {
                if let Some((from_left, cy, s)) = self.cue {
                    widgets::back_cue(out, &t, from_left, cy, s.at(now).0.max(0.0), false);
                }
            }
        }
    }
}

impl App {
    /// Draw the page shown as `view` has it: all of it but the status bar,
    /// any message and the back cue, which lie over every page. Rows of its
    /// long list come into view are drawn with `fresh`; a page drawn again
    /// as it last showed has them drawn already.
    pub(super) fn paint_page(&mut self, out: &mut Canvas, t: &Theme, fresh: Option<(&Fonts, &DeviceInfo)>, view: &View) {
        let m = t.m;
        let screen = self.screen();
        let (scroll, offset, pressing) = (view.scroll, view.offset, view.pressing);
        let browser_fullscreen = self.browser_fullscreen();
        let shine = |out: &mut Canvas, part: Group| {
            if let Some((g, r, radius, tint)) = view.light
                && g == part
            {
                out.fill_round_rect(r.x, r.y, r.w, r.h, radius, tint);
            }
        };
        let frame = self.frame.as_ref().expect("a page is drawn from its frame");
        let (_, big, small) = self.titles.as_mut().expect("a page's titles are drawn with it");
        let press_row = |out: &mut Canvas| {
            if let (Some(press), Some((was, is))) = (self.row_press, &self.row_art) {
                let (x, y, _, _) = row_part(press.rect, &frame.canvas);
                out.relay(was, is, x as i32, y as i32 - offset as i32, view.row);
            }
        };
        if let Some(art) = &frame.hero {
            let glow = self.glow.as_ref().expect("the About page keeps its field");
            let fade = (1.0 - scroll / m.dp(ABOUT_FIELD_DP)).clamp(0.0, 1.0);
            let top = (widgets::bar_bottom(t) + m.dp(ABOUT_FIELD_DP)) / m.h as f32;
            let field = glow::sample(glow, out.w as usize, out.h as usize, FIELD_CELL, top, t.pal.bg, fade);
            let dither = self.dither.get_or_insert_with(|| glow::Dither::new(out.w as usize, out.h as usize));
            glow::paint(
                out,
                &field,
                fade,
                &glow::Layer {
                    canvas: &frame.canvas,
                    runs: &art.runs,
                    offset: offset as isize,
                },
                dither,
            );
            press_row(out);
            let range = m.dp(ABOUT_LOGO_DP) - widgets::bar_bottom(t);
            let v = (scroll / range).clamp(0.0, 1.0);
            art.version.draw_scaled(out, 0.0, 0.0, 1.0 - 0.1 * v, t.hero.version, 1.0 - v);
            art.author.draw_scaled(out, 0.0, 0.0, 1.0 - 0.1 * v, t.hero.version, 1.0 - v);
            let l = ((scroll - range) / m.dp(ABOUT_LOGO_FADE_DP)).clamp(0.0, 1.0);
            glow::ink_wordmark(out, &field, &glow.look().ink, &art.wordmark, 1.0 - 0.1 * l, 1.0 - l, fade, dither);
        } else {
            let h = out.h as f32;
            let span = frame
                .rows
                .as_ref()
                .map(|r| ((r.edges[0] - offset).clamp(0.0, h), (r.bottom() - offset).clamp(0.0, h)));
            let (list_top, list_end) = span.unwrap_or((h, h));
            let first = (-offset).clamp(0.0, h) as usize;
            let last = (frame.canvas.h as f32 - offset).clamp(first as f32, list_top) as usize;
            if first < last {
                let from = offset as i32 + first as i32;
                out.put_rows(&frame.canvas, from, from + (last - first) as i32, first as i32);
            }
            out.fill_rect(0.0, 0.0, out.w as f32, first as f32, frame.bg);
            out.fill_rect(0.0, last as f32, out.w as f32, list_top - last as f32, frame.bg);
            let below = list_end.max(last as f32);
            out.fill_rect(0.0, below, out.w as f32, h - below, frame.bg);
            if let Some(rows) = &frame.rows {
                let near = rows.within(offset - h * NEAR_SCREENS, offset + h * (1.0 + NEAR_SCREENS));
                self.spare
                    .extend(self.drawn.extract_if(.., |(i, _)| !near.contains(i)).map(|(_, c)| c));
                for i in rows.within(offset, offset + h) {
                    let k = match self.drawn.iter().position(|(j, _)| *j == i) {
                        Some(k) => k,
                        None => {
                            let (f, info) = fresh.expect("a page drawn again as it last showed has its rows in view drawn");
                            let mut c = self.spare.pop().unwrap_or_else(|| Canvas::new(out.w, 0));
                            c.resize((rows.edges[i + 1] - rows.edges[i]) as u32);
                            let cx = Ctx {
                                t,
                                f,
                                info,
                                state: &self.state,
                                term: self.term.as_ref(),
                                band: None,
                            };
                            pages::row(screen, &cx, i, &mut c);
                            self.drawn.push((i, c));
                            self.drawn.len() - 1
                        }
                    };
                    out.put(&self.drawn[k].1, (rows.edges[i] - offset) as i32);
                }
            } else {
                self.spare.clear();
            }
            press_row(out);
            if let Some(fold) = &frame.fold {
                let (_, words) = self
                    .search_words
                    .as_ref()
                    .expect("a folding search bar's words are made with its page");
                fold.draw(out, t, words, offset);
            }
            if frame.header {
                let level = (1.0 - frame.title_lift(scroll) / m.dp(TITLE_FADE_DP)).clamp(0.0, 1.0);
                draw_faded(
                    out,
                    big,
                    0.0,
                    -frame.title_lift(offset),
                    t.pal.text,
                    level,
                    (1.0 - level) * m.dp(TITLE_BLUR_DP),
                );
            }
        }
        if let Some(a) = pressing.filter(|a| matches!(a, Action::CloseSearch | Action::Open(Screen::Search))) {
            let r = frame
                .hits
                .iter()
                .find(|h| h.action == a)
                .expect("a button held down is on its page")
                .rect;
            match frame.shown(r, offset) {
                Some(r) if a == Action::CloseSearch => widgets::cancel_held(out, t, r),
                Some(r) => widgets::search_bar_held(out, t, r),
                None => {}
            }
        }
        if let Some(caret) = frame.caret.as_ref().filter(|_| view.caret) {
            out.lay(&caret.art, caret.x, caret.y - offset as i32, 1.0);
        }
        let bar = view.bar;
        let strip = frame.pinned.as_ref().map(|p| (p, pinned_top(t, p, offset)));
        shine(out, Group::Page);
        if bar > 0.0 && frame.browser_area.is_none() {
            match strip {
                Some((p, top)) => widgets::strip_mask(out, t, frame.bg, bar, top.round(), top.round() + p.canvas.h as f32),
                None => {
                    let stops = if frame.hero.is_some() {
                        &widgets::ABOUT_BAR_MASK[..]
                    } else {
                        &widgets::BAR_MASK[..]
                    };
                    widgets::bar_mask(out, t, stops, frame.bg, bar);
                }
            }
        }
        if let Some((p, top)) = strip {
            out.put(&p.canvas, top.round() as i32);
            shine(out, Group::Strip);
        }
        if let Some(d) = &frame.dock {
            if browser_fullscreen {
                out.lay(&d.canvas, 0, t.m.h as i32 - d.canvas.h as i32, 1.0);
            } else {
                out.put(&d.canvas, t.m.h as i32 - d.canvas.h as i32);
            }
            shine(out, Group::Dock);
        }
        if frame.back {
            let (_, chip, x, y) = self.chip.as_ref().expect("a page's `<` is drawn with it");
            out.lay(chip, *x, *y, bar);
            if pressing == Some(Action::Back) {
                widgets::back_held(out, t);
            } else {
                widgets::back_arrow(out, t, t.pal.text);
            }
        }
        if let Some((glyph, action)) = frame.button {
            let (_, chip, x, y) = self.button_chip.as_ref().expect("a page's round button is drawn with its chip");
            out.lay(chip, *x, *y, bar);
            if pressing == Some(action) {
                widgets::button_held(out, t, glyph);
            } else {
                widgets::button_glyph(out, t, glyph, t.pal.text);
            }
        }
        if frame.back || frame.button.is_some() {
            shine(out, Group::Fixed);
        }
        let level = view.title;
        let blur = if frame.hero.is_some() {
            0.0
        } else {
            (1.0 - level) * m.dp(TITLE_BLUR_DP)
        };
        draw_faded(out, small, 0.0, 0.0, t.pal.text, level, blur);
    }
}

impl App {
    /// Where a page sliding in over the page left, or the page left
    /// sliding off, stands at `now`: how many pixels from the left edge the
    /// page in front stands, how many left of where it rests the page
    /// behind, and how dark that is; nothing once the slide is over. See
    /// [`Turn`].
    pub(super) fn slid(&mut self, now: u64) -> Option<(i32, i32, f32)> {
        let turn = self.turn.as_mut()?;
        let spring = *turn.spring.get_or_insert_with(|| {
            if turn.back {
                Spring::new(now, -1000.0, 0.0, BACK_PERIOD)
            } else {
                Spring::new(now, 1000.0, -OPEN_SPEED * 1000.0, OPEN_PERIOD)
            }
        });
        let (x, done) = spring.at(now);
        if done {
            self.backdrop = self.turn.take().map(|t| t.left);
            return None;
        }
        let along = if turn.back { 1.0 + x / 1000.0 } else { x / 1000.0 };
        let w = self.w as f32;
        Some((
            (along * w).round() as i32,
            (BEHIND_SHIFT * w * (1.0 - along)).round() as i32,
            BEHIND_DIM * (1.0 - along),
        ))
    }
}

impl App {
    /// Whether the cursor of the text field on `screen` shows at `now`: it
    /// comes up whole on each page it shows on and at each edit, then
    /// blinks.
    pub(super) fn caret_up(&mut self, screen: Screen, now: u64) -> bool {
        let key = (screen, self.edits);
        if self.blink.is_none_or(|(s, e, _)| (s, e) != key) {
            self.blink = Some((screen, self.edits, now));
        }
        self.caret_shown = self.blink_at(now).is_some_and(|(on, _)| on);
        self.caret_shown
    }
}

impl App {
    /// Draw the search bar on its way between the home page and the search
    /// page; see [`Morph`].
    pub(super) fn compose_morph(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, now: u64, caret_on: bool) {
        let t = self.theme();
        let m = t.m;
        let mut morph = self.morph.take().expect("composing it");
        let (q, _) = morph.at(now);
        let (field, _) = widgets::search_layout(&t);
        let home = Rect::new(m.margin, m.content_top - morph.scroll, m.w as f32 - m.margin * 2.0, m.search_h);
        let along = |a: f32, b: f32| a + (b - a) * q;
        let bar = Rect::new(m.margin, along(home.y, field.y), along(home.w, field.w), m.search_h);
        let rise = bar.y - home.y;
        if !morph.into {
            self.draw_rows(f, info, morph.scroll - rise, morph.scroll - rise + m.h as f32);
        }
        let frame = self.frame.as_ref().expect("rendered above");
        out.fill_rect(0.0, 0.0, out.w as f32, out.h as f32, frame.bg);
        if morph.into {
            let (_, keys_top) = *morph.keyboard.get_or_insert_with(|| keyboard_strip(&t, frame));
            let (body, top) = morph.body.get_or_insert_with(|| {
                let top = (field.y + field.h).ceil() as i32;
                let mut body = Canvas::new(frame.canvas.w, (keys_top - top).max(0) as u32);
                body.put_rows(&frame.canvas, top, keys_top, 0);
                (body, top)
            });
            let (delay, span) = MORPH_BODY_MS;
            out.lay(
                body,
                0,
                *top,
                (now.saturating_sub(morph.start + delay) as f32 / span as f32).min(1.0),
            );
        } else {
            let below = m.content_top + m.search_h;
            out.put_rows(
                &frame.canvas,
                below.floor() as i32,
                frame.canvas.h as i32,
                (below - morph.scroll + rise).floor() as i32,
            );
        }
        let title = morph
            .title
            .get_or_insert_with(|| Blurs::new(widgets::big_title(&t, f, pages::HOME_TITLE)));
        let level = (1.0 - q / MORPH_TITLE_GONE).clamp(0.0, 1.0);
        draw_faded(
            out,
            title,
            0.0,
            rise - morph.scroll,
            t.pal.text,
            level,
            (1.0 - level) * m.dp(TITLE_BLUR_DP),
        );
        if let Some((keys, top)) = &morph.keyboard {
            out.put(keys, (*top as f32 + (1.0 - q) * keys.h as f32).round() as i32);
        }
        widgets::search_capsule(out, &t, f, bar, self.state.lang.pick(texts::search::HINT));
        // The cursor shows from the start, going up with the field.
        if let Some(caret) = frame.caret.as_ref().filter(|_| morph.into && caret_on) {
            out.lay(&caret.art, caret.x, caret.y + (bar.y - field.y).round() as i32, 1.0);
        }
        let (disc, cross) = morph.cancel.get_or_insert_with(|| {
            let (disc, cross) = widgets::cancel_art(&t);
            (Blurs::new(disc), Blurs::new(cross))
        });
        let shown = ((q - MORPH_CANCEL.0) / (MORPH_CANCEL.1 - MORPH_CANCEL.0)).clamp(0.0, 1.0);
        let blur = (1.0 - shown) * m.dp(MORPH_CANCEL_BLUR_DP);
        draw_faded(out, disc, 0.0, 0.0, t.pal.search, shown, blur);
        draw_faded(out, cross, 0.0, 0.0, t.pal.text, shown, blur);
        self.morph = Some(morph);
        widgets::status_bar(out, &t, f, &self.bar(clock, info), t.pal.status);
        if let Some((text, _)) = &self.toast {
            widgets::toast(out, &t, f, text);
        }
    }
}

impl App {
    /// Put the page back where it rests, for a page just opened or returned
    /// to; its first frame shows the bar as its scroll has it.
    pub(super) fn settle_page(&mut self) {
        self.height = 0.0;
        self.fling = None;
        self.glide = None;
        self.pull = 0.0;
        self.bounce = None;
        self.seek = None;
        self.adjusting = false;
        self.titles = None;
        self.glow = None;
        self.row_press = None;
        self.row_art = None;
        let hero = self.screen() == Screen::About;
        self.title = title_fade(hero);
        self.bar = bar_fade(hero);
        self.snap = true;
    }
}

impl App {
    /// Show the user what the runtime has to say about work it was asked
    /// to do.
    pub fn report(&mut self, text: &str, now: u64) {
        self.say(text, now);
    }
}

impl App {
    /// The screenshot [`Effect::Screenshot`] asked for went where `saved`
    /// says, or did not, as the system says why.
    pub fn screenshot_saved(&mut self, saved: Result<Shot, String>, now: u64) {
        let l = self.state.lang;
        let text = match saved {
            Ok(Shot::Album) => l.pick(fmt::SCREENSHOT_SAVED).to_string(),
            Ok(Shot::Held) => l.pick(fmt::SCREENSHOT_HELD).to_string(),
            Err(why) => fmt::screenshot_failed(l, &why),
        };
        self.say(&text, now);
    }
}

impl App {
    /// The screenshots held while the owner's storage was closed did not go
    /// into the gallery's once it opened, as the system says why.
    pub fn screenshots_not_kept(&mut self, why: &str, now: u64) {
        self.say(&fmt::screenshots_not_kept(self.state.lang, why), now);
    }
}

impl App {
    /// File transfer could not be turned on or off as the switch says: it
    /// shows how it stands, and why.
    pub fn file_transfer_failed(&mut self, on: bool, why: &str, now: u64) {
        self.state.file_transfer = on;
        self.refresh();
        self.say(why, now);
    }
}

impl App {
    pub(super) fn say(&mut self, text: &str, now: u64) {
        self.toast = Some((text.to_string(), now + TOAST_MS));
        self.dirty = true;
    }
}

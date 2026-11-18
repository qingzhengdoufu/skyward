use super::*;

impl App {
    pub fn touch(&mut self, ev: Touch, f: &Fonts, now: u64) -> Vec<Effect> {
        if !self.display_on {
            if let Touch::Down(x, y) = ev {
                self.tap(now, x, y);
            }
            return Vec::new();
        }
        if self.charge_shown() {
            if matches!(ev, Touch::Down(..)) {
                self.dismiss_charging(now);
                self.drag = Drag::Waking;
            }
            return Vec::new();
        }
        if matches!(ev, Touch::Down(..)) && self.just_woke(now) {
            self.stir(now);
            self.drag = Drag::Waking;
            return Vec::new();
        }
        if let Some(effects) = self.immersive_touch(ev, now) {
            return effects;
        }
        if let Some(effects) = self.navigation_touch(ev, now) {
            return effects;
        }
        if !self.browser_immersive() && self.shade_touch(ev, now) {
            return Vec::new();
        }
        let shade_content = self.shade.is_some() && self.screen() == Screen::ControlCenter;
        if !shade_content && self.browser_address_touch(ev, f, now) {
            return Vec::new();
        }
        if !shade_content
            && let Touch::Down(x, y) = ev
            && self.state.browser.full_active
            && let Some(area) = self.frame.as_ref().and_then(|f| f.browser_area)
            && area.contains(x, y + self.top().1)
        {
            let mapping = self.browser_viewport();
            let (px, py) = browser_point(mapping, x, y + self.top().1);
            self.drag = Drag::BrowserTouch {
                rect: mapping,
                last: (px, py),
            };
            self.stir(now);
            return vec![Effect::BrowserPluginTouch { phase: 0, x: px, y: py }];
        }
        if let Drag::BrowserTouch { rect, last } = self.drag {
            let effect = match ev {
                Touch::Move(x, y) => {
                    let (x, y) = browser_point(rect, x, y + self.top().1);
                    self.drag = Drag::BrowserTouch { rect, last: (x, y) };
                    Some(Effect::BrowserPluginTouch { phase: 1, x, y })
                }
                Touch::Up => {
                    self.drag = Drag::Idle;
                    Some(Effect::BrowserPluginTouch {
                        phase: 2,
                        x: last.0,
                        y: last.1,
                    })
                }
                _ => None,
            };
            self.stir(now);
            return effect.into_iter().collect();
        }
        if matches!(ev, Touch::Down(..)) {
            if self.keys {
                self.keys = false;
                self.adjusting = false;
                self.dirty = true;
            }
            self.drop_button();
            self.end_turn();
        }
        self.stir(now);
        let t = self.theme();
        let m = t.m;
        let (slop, edge) = (m.dp(8.0), m.dp(20.0));
        let scroll = self.top().1;
        let mut fx = Vec::new();
        let pressing = self.pressing();
        match ev {
            Touch::Down(x, y) => {
                self.tracker.clear();
                self.tracker.add(now, y);
                let side = if x < edge {
                    Some(true)
                } else if x > m.w as f32 - edge {
                    Some(false)
                } else {
                    None
                };
                let at_edge = side.filter(|_| self.stack.len() > 1 && !self.frame.as_ref().is_some_and(|frame| frame.fixed));
                if at_edge.is_some() && self.in_motion(now) {
                    self.drag = Drag::Press {
                        x,
                        y,
                        action: None,
                        edge: at_edge,
                        at: now,
                    };
                    return fx;
                }
                let glided = self.glide.take().filter(|(glide, ..)| !glide.at(now).1);
                let history = self
                    .frame
                    .as_ref()
                    .and_then(|fr| fr.scrollback)
                    .filter(|(r, _)| r.contains(x, y + scroll));
                if let (Some(_), Some((_, pitch))) = (glided, history) {
                    self.drag = Drag::History {
                        last: y,
                        carry: 0.0,
                        pitch,
                    };
                    return fx;
                }
                if self.catch(now) {
                    self.drag = Drag::Scroll { last: y };
                    return fx;
                }
                let back = widgets::back_rect(&t);
                let strip = self
                    .frame
                    .as_ref()
                    .and_then(|fr| fr.pinned.as_ref())
                    .map(|p| (p, pinned_top(&t, p, scroll)));
                let on_strip = strip.filter(|&(p, top)| y >= top && y < top + p.canvas.h as f32);
                let dock = self
                    .frame
                    .as_ref()
                    .and_then(|fr| fr.dock.as_ref())
                    .map(|d| (d, t.m.h as f32 - d.canvas.h as f32));
                let on_dock = dock.filter(|&(_, top)| y >= top);
                let button = self
                    .frame
                    .as_ref()
                    .and_then(|fr| fr.button)
                    .map(|(_, a)| (a, widgets::button_rect(&t)));
                let hit = if self.frame.as_ref().is_some_and(|fr| fr.back) && back.contains(x, y) {
                    Some((Action::Back, back))
                } else if let Some(button) = button.filter(|(_, r)| r.contains(x, y)) {
                    Some(button)
                } else if let Some((d, top)) = on_dock {
                    d.hits.iter().find(|h| h.rect.contains(x, y - top)).map(|h| {
                        (
                            h.action,
                            Rect {
                                y: h.rect.y + top,
                                ..h.rect
                            },
                        )
                    })
                } else if let Some((p, top)) = on_strip {
                    p.hits.iter().find(|h| h.rect.contains(x, y - top)).map(|h| {
                        (
                            h.action,
                            Rect {
                                y: h.rect.y + top,
                                ..h.rect
                            },
                        )
                    })
                } else if self.bar.on && y < widgets::bar_bottom(&t) {
                    None
                } else {
                    let fr = self.frame.as_ref();
                    fr.and_then(|fr| {
                        fr.hit(x, y + scroll)
                            .filter(|h| fr.shown(h.rect, scroll).is_some_and(|r| r.contains(x, y)))
                    })
                    .map(|h| (h.action, h.rect))
                };
                let hit = hit.map(|(action, rect)| {
                    let action = if let Action::BrowserPluginTap(px, py) = action {
                        let (cx, cy) = rect.centre();
                        Action::BrowserPluginTap(
                            (f32::from(px) + (x - cx) / rect.w * (1000.0 / 12.0)).clamp(0.0, 999.0) as u16,
                            (f32::from(py) + (y + scroll - cy) / rect.h * (1000.0 / 24.0)).clamp(0.0, 999.0) as u16,
                        )
                    } else {
                        action
                    };
                    (action, rect)
                });
                let action = hit.map(|h| h.0);
                if matches!(action, Some(Action::Type(_))) && self.state.vibrate {
                    fx.push(Effect::Haptic);
                }
                let row = hit
                    .filter(|&(a, r)| self.frame.as_ref().is_some_and(|fr| fr.hits.contains(&Hit { rect: r, action: a })))
                    .filter(|&(a, r)| {
                        row_wide(&t, r) && !matches!(a, Action::Open(Screen::Search) | Action::Confirm(_) | Action::TypeInto(_))
                    })
                    .map(|(_, r)| r);
                self.drag = match hit {
                    Some((Action::Slider(which), track)) => {
                        self.set_slider(&t, which, track, x);
                        Drag::Slider(which, track)
                    }
                    Some((Action::Confirm(which), track)) if self.on_knob(track, x) => {
                        let knob_x = track.x + (track.w - track.h) * self.state.confirm;
                        Drag::Confirm {
                            which,
                            track,
                            grab: x - knob_x,
                        }
                    }
                    Some((Action::Pattern, _)) => {
                        self.state.pattern.clear();
                        self.trace(&t, x, y + scroll);
                        Drag::Pattern
                    }
                    _ => {
                        if let Some(rect) = row {
                            self.row_press = Some(RowPress {
                                rect,
                                rise: now + ROW_PRESS_DELAY_MS,
                                fall: None,
                            });
                            self.row_art = None;
                        }
                        Drag::Press {
                            x,
                            y,
                            action,
                            edge: at_edge,
                            at: now,
                        }
                    }
                };
            }
            Touch::Move(x, y) => match std::mem::replace(&mut self.drag, Drag::Idle) {
                Drag::Press {
                    x: x0,
                    y: y0,
                    action,
                    edge: at_edge,
                    at,
                } => {
                    self.tracker.add(now, y);
                    let (dx, dy) = (x - x0, y - y0);
                    self.drag = match at_edge {
                        Some(from_left) if dx.abs() > slop && dx.abs() > dy.abs() => {
                            self.dirty = true;
                            Drag::Edge {
                                start: x0,
                                from_left,
                                armed: false,
                                x,
                                cy: y0 - m.dp(CUE_LIFT_DP),
                            }
                        }
                        _ if matches!(action, Some(Action::BrowserPluginTap(..))) && dy.abs() > slop => {
                            Drag::BrowserScroll { start: y0, last: y }
                        }
                        _ if dx.abs().max(dy.abs()) > slop && self.frame.as_ref().is_some_and(|frame| frame.fixed) => Drag::Held,
                        _ if dy.abs() > slop => {
                            let history = self
                                .frame
                                .as_ref()
                                .and_then(|fr| fr.scrollback)
                                .filter(|(r, _)| r.contains(x0, y0 + scroll));
                            if let Some((_, pitch)) = history {
                                self.scroll_history(y0, y, 0.0, pitch)
                            } else {
                                if !self.catch(now) {
                                    self.drag_by(y0 - y);
                                }
                                Drag::Scroll { last: y }
                            }
                        }
                        _ => Drag::Press {
                            x: x0,
                            y: y0,
                            action,
                            edge: at_edge,
                            at,
                        },
                    };
                    if !matches!(self.drag, Drag::Press { .. }) {
                        self.let_row_go(now);
                    }
                }
                Drag::Scroll { last } => {
                    self.tracker.add(now, y);
                    self.drag_by(last - y);
                    self.drag = Drag::Scroll { last: y };
                }
                Drag::BrowserScroll { start, .. } => self.drag = Drag::BrowserScroll { start, last: y },
                Drag::Edge {
                    start,
                    from_left,
                    armed,
                    cy,
                    ..
                } => {
                    let travel = if from_left { x - start } else { start - x };
                    let ready = travel > m.dp(BACK_SWIPE_DP);
                    if ready && !armed && self.state.vibrate {
                        fx.push(Effect::Haptic);
                    }
                    self.drag = Drag::Edge {
                        start,
                        from_left,
                        armed: ready,
                        x,
                        cy,
                    };
                    self.dirty = true;
                }
                Drag::Slider(which, track) => {
                    self.set_slider(&t, which, track, x);
                    self.drag = Drag::Slider(which, track);
                }
                Drag::Confirm { which, track, grab } => {
                    let travel = (track.w - track.h).max(1.0);
                    self.state.confirm = ((x - grab - track.x) / travel).clamp(0.0, 1.0);
                    self.repaint_confirm(&t, f, which, track);
                    self.drag = Drag::Confirm { which, track, grab };
                }
                Drag::Pattern => {
                    self.trace(&t, x, y + scroll);
                    self.drag = Drag::Pattern;
                }
                Drag::History { last, carry, pitch } => {
                    self.tracker.add(now, y);
                    self.drag = self.scroll_history(last, y, carry, pitch);
                }
                Drag::Waking => self.drag = Drag::Waking,
                Drag::Held => self.drag = Drag::Held,
                Drag::Idle => {}
                Drag::BrowserTouch { .. } | Drag::BrowserCaret => {}
            },
            Touch::Up => match std::mem::replace(&mut self.drag, Drag::Idle) {
                Drag::Press { action: Some(a), .. } => {
                    if let Some(p) = &mut self.row_press {
                        p.lift(now);
                    }
                    fx.extend(self.act(a, now));
                }
                Drag::Press { x, y, action: None, .. } if self.screen() == Screen::Editor => self.place_caret(f, x, y + scroll),
                Drag::History { pitch, .. } => {
                    let speed = self.tracker.velocity(now);
                    if speed.abs() >= m.dp(MIN_FLING_DP) {
                        let speed = speed.clamp(-m.dp(MAX_FLING_DP), m.dp(MAX_FLING_DP));
                        self.glide = Some((Fling::new(now, 0.0, speed, self.px_per_dp * 160.0), pitch, 0));
                    }
                }
                Drag::Scroll { .. } if self.pull != 0.0 => {
                    self.bounce = Some(Spring::new(now, motion::rubber(self.pull, self.h as f32), 0.0, 0.4));
                    self.pull = 0.0;
                    self.dirty = true;
                }
                Drag::Scroll { .. } => {
                    let speed = -self.tracker.velocity(now);
                    let folding = |s: f32| self.frame.as_ref().is_some_and(|fr| fr.settle(s) != s);
                    let glide = (speed.abs() >= m.dp(MIN_FLING_DP)).then(|| {
                        let speed = speed.clamp(-m.dp(MAX_FLING_DP), m.dp(MAX_FLING_DP));
                        Fling::new(now, scroll, speed, self.px_per_dp * 160.0)
                    });
                    match glide {
                        Some(glide) if !(folding(scroll) && folding(glide.at(u64::MAX).0)) => {
                            self.fling = Some(glide);
                        }
                        _ => self.settle_fold(now),
                    }
                }
                Drag::BrowserScroll { start, last } => {
                    fx.extend(self.act(Action::BrowserPluginScroll((last - start) / self.px_per_dp), now))
                }
                Drag::Edge { armed: true, .. } => fx.extend(self.back(now)),
                Drag::Edge {
                    start, from_left, x, cy, ..
                } => {
                    let travel = if from_left { x - start } else { start - x };
                    let depth = widgets::back_cue_depth(&t, travel);
                    self.cue = Some((from_left, cy, Spring::new(now, depth, 0.0, 0.15)));
                    self.dirty = true;
                }
                Drag::Confirm { which, track, .. } => {
                    let done = self.state.confirm > SLID;
                    self.state.confirm = 0.0;
                    self.repaint_confirm(&t, f, which, track);
                    if done {
                        fx.extend(self.confirmed(which, now));
                    }
                }
                Drag::Pattern => fx.extend(self.drawn(now)),
                Drag::Slider(Slider::Vibration, _) if self.state.vibrate => fx.push(Effect::Haptic),
                _ => {}
            },
        }
        if self.pressing() != pressing {
            self.dirty = true;
        }
        let held = match self.drag {
            Drag::Press {
                action: Some(a @ (Action::Type(_) | Action::PickSecurity(_))),
                ..
            } => Some(a),
            _ => None,
        };
        if held != self.state.held {
            let was = std::mem::replace(&mut self.state.held, held);
            let rects: Option<Vec<Rect>> = self.frame.as_ref().and_then(|fr| {
                [was, held]
                    .into_iter()
                    .flatten()
                    .map(|a| fr.hits.iter().find(|h| h.action == a).map(|h| h.rect))
                    .collect()
            });
            match rects {
                Some(rects) => rects
                    .into_iter()
                    .for_each(|r| self.refresh_rows(r.y.floor() as u32, (r.y + r.h).ceil() as u32)),
                None => self.refresh(),
            }
        }
        fx
    }
}

fn browser_point(rect: Rect, x: f32, y: f32) -> (u16, u16) {
    (
        ((x - rect.x) / rect.w * 1000.0).clamp(0.0, 999.0) as u16,
        ((y - rect.y) / rect.h * 1000.0).clamp(0.0, 999.0) as u16,
    )
}

impl App {
    /// The button a finger holds down, which shows held at once: `<`, the
    /// search page's round button, or the home page's search bar.
    pub(super) fn pressing(&self) -> Option<Action> {
        let button = self.frame.as_ref().and_then(|fr| fr.button).map(|(_, a)| a);
        match self.drag {
            Drag::Press {
                action: Some(a @ (Action::Back | Action::CloseSearch | Action::Open(Screen::Search))),
                ..
            } => Some(a),
            Drag::Press { action: Some(a), .. } if Some(a) == button => Some(a),
            _ => None,
        }
    }
}

impl App {
    /// The finger left the row it held, scrolling the page or swiping: a
    /// row not darkening yet never does, and one darkening fades back.
    pub(super) fn let_row_go(&mut self, now: u64) {
        let Some(p) = self.row_press else { return };
        if now < p.rise {
            self.row_press = None;
            self.row_art = None;
        } else if p.fall.is_none() {
            self.row_press = Some(RowPress { fall: Some(now), ..p });
        }
        self.dirty = true;
    }
}

impl App {
    /// No touchscreen answers: the keys work the pages, their light shown
    /// from now on.
    pub fn touchscreen_missing(&mut self) {
        self.keys = true;
        self.dirty = true;
    }
}

impl App {
    /// A button went down. With the keys working the page, the volume keys
    /// move the light from control to control and the power key works the
    /// one lit as it comes back up; otherwise the first volume press lights
    /// a control, and the power key turns the panel off. The power key and
    /// volume down together take a screenshot instead: whichever of the two
    /// goes down first waits [`CHORD_MS`] for the other, or until it is let
    /// go, and then does what it does alone, as pressed when it went down.
    pub fn key(&mut self, key: Key, now: u64) -> Vec<Effect> {
        self.fling = None;
        self.end_turn();
        if let Some((first, at)) = self.chord
            && matches!((first, key), (Key::Power, Key::VolumeDown) | (Key::VolumeDown, Key::Power))
            && now < at + CHORD_MS
        {
            return self.screenshot(key, now);
        }
        let mut fx = self.let_chord_go(now);
        if !self.display_on {
            if key == Key::Power {
                self.wake(now);
            }
            self.button = Some((key, Press::Spent));
            return fx;
        }
        if key != Key::Power && self.charge_shown() {
            self.dismiss_charging(now);
            self.button = Some((key, Press::Spent));
            return fx;
        }
        if self.just_woke(now) {
            self.stir(now);
            self.button = Some((key, Press::Spent));
            return fx;
        }
        self.stir(now);
        if key == Key::VolumeUp && self.browser_immersive() {
            self.button = Some((key, Press::Spent));
            return self.set_browser_immersive(false);
        }
        if matches!(key, Key::Power | Key::VolumeDown) {
            self.chord = Some((key, now));
            return fx;
        }
        fx.extend(self.key_down(key, now, now));
        fx
    }
}

impl App {
    /// The power key and volume down went down together: what the panel
    /// shows is saved as a picture, with a tap of the motor, and neither key
    /// does anything more until let go.
    pub(super) fn screenshot(&mut self, key: Key, now: u64) -> Vec<Effect> {
        self.chord = None;
        self.stir(now);
        self.button = Some((key, Press::Spent));
        let mut fx = Vec::new();
        if self.state.vibrate {
            fx.push(Effect::Haptic);
        }
        fx.push(Effect::Screenshot {
            storage: self.state.place_open(Place::Storage),
        });
        fx
    }
}

impl App {
    /// The key waiting for its partner in a screenshot does what it does
    /// alone `now`, as pressed when it went down.
    pub(super) fn let_chord_go(&mut self, now: u64) -> Vec<Effect> {
        match self.chord.take() {
            Some((key, at)) => self.key_down(key, at, now),
            None => Vec::new(),
        }
    }
}

impl App {
    /// What a button that went down `at` does on its own, `now`: a key
    /// held is timed from when it went down, the screen goes off from now.
    pub(super) fn key_down(&mut self, key: Key, at: u64, now: u64) -> Vec<Effect> {
        match key {
            Key::Back => {
                self.button = Some((key, Press::Spent));
                self.act(Action::Back, at)
            }
            Key::VolumeUp | Key::VolumeDown => {
                let down = key == Key::VolumeDown;
                self.button = Some((
                    key,
                    Press::Volume {
                        down,
                        next: at + REPEAT_DELAY_MS,
                    },
                ));
                self.volume(down, at)
            }
            Key::Power if self.keys && self.frame.is_none() => {
                self.button = Some((key, Press::Spent));
                Vec::new()
            }
            Key::Power => {
                self.dirty = true;
                match self.keys.then(|| self.lit()).flatten() {
                    Some(Spot {
                        stop: Stop::Hit(Action::Confirm(which)),
                        rect,
                        ..
                    }) => {
                        self.button = Some((key, Press::Slide { which, track: rect, at }));
                    }
                    Some(spot) => self.button = Some((key, Press::Power { spot, at })),
                    None => {
                        self.button = Some((key, Press::Spent));
                        self.sleep(now);
                        return vec![Effect::Ticks];
                    }
                }
                Vec::new()
            }
        }
    }
}

impl App {
    /// A button came back up: the power key works the control it went down
    /// on, or confirms with the knob it slid far enough. Let go before its
    /// partner in a screenshot came, the key waiting for it first does what
    /// it does alone.
    pub fn key_up(&mut self, key: Key, now: u64) -> Vec<Effect> {
        let mut fx = match self.chord {
            Some((first, _)) if first == key => self.let_chord_go(now),
            _ => Vec::new(),
        };
        let Some((down, press)) = self.button else { return fx };
        if down != key {
            return fx;
        }
        self.button = None;
        self.dirty = true;
        match press {
            Press::Power { spot, .. } => fx.extend(self.work(spot, now)),
            Press::Slide { which, track, .. } => {
                let done = self.state.confirm > SLID;
                self.let_knob_go(which, track);
                if done {
                    fx.extend(self.confirmed(which, now));
                } else {
                    self.say(self.state.lang.pick(["请按住电源键", "請按住電源鍵", "Hold the power key"]), now);
                }
            }
            Press::Volume { .. } | Press::Spent => {}
        }
        fx
    }
}

impl App {
    /// Whatever the button held down was doing comes to nothing until it is
    /// let go: a knob it slid goes back.
    pub(super) fn drop_button(&mut self) {
        let Some((key, press)) = self.button else { return };
        if let Press::Slide { which, track, .. } = press {
            self.let_knob_go(which, track);
        }
        self.button = Some((key, Press::Spent));
    }
}

impl App {
    /// Let go of a swipe-to-confirm control's knob, which goes back to the
    /// start of its track.
    pub(super) fn let_knob_go(&mut self, which: Confirm, track: Rect) {
        self.state.confirm = 0.0;
        self.knob = Some((which, track));
        self.dirty = true;
    }
}

impl App {
    pub(super) fn on_knob(&self, track: Rect, x: f32) -> bool {
        let knob_x = track.x + (track.w - track.h) * self.state.confirm;
        x >= knob_x && x <= knob_x + track.h
    }
}

impl App {
    pub(super) fn repaint_confirm(&mut self, t: &Theme, f: &Fonts, which: Confirm, track: Rect) {
        self.paint_knob(t, f, which, track);
        self.dirty = true;
    }
}

impl App {
    /// Draw the swipe-to-confirm control at `track` on the page again, its
    /// knob where it has come to.
    pub(super) fn paint_knob(&mut self, t: &Theme, f: &Fonts, which: Confirm, track: Rect) {
        let (label, danger) = pages::confirm_label(which, &self.state);
        if let Some(fr) = self.frame.as_mut() {
            fr.canvas.fill_rect(track.x, track.y, track.w, track.h, t.pal.bg);
            widgets::confirm(&mut fr.canvas, t, f, track.y, label, danger, self.state.confirm);
        }
        self.layer = None;
    }
}

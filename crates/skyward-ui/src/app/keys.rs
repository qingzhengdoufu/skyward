use super::*;

impl App {
    /// Work the control the keys are on, as touching it does. The
    /// brightness slider takes the volume keys until the power key lets it
    /// go, and a pattern's dots are joined one press at a time, a second
    /// press on the last one joined ending the pattern.
    pub(super) fn work(&mut self, spot: Spot, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        match spot.stop {
            Stop::Back => self.act(Action::Back, now),
            Stop::Button(action) => self.act(action, now),
            Stop::Hit(Action::Slider(which)) => {
                self.adjusting = !self.adjusting;
                if self.adjusting {
                    let said = match which {
                        Slider::Brightness => [
                            "按音量键调节亮度，按电源键完成",
                            "按音量鍵調整亮度，按電源鍵完成",
                            "Set the brightness with the volume keys, then press the power key",
                        ],
                        Slider::Vibration => [
                            "按音量键调节振动强度，按电源键完成",
                            "按音量鍵調整震動強度，按電源鍵完成",
                            "Set the vibration with the volume keys, then press the power key",
                        ],
                    };
                    self.say(l.pick(said), now);
                }
                Vec::new()
            }
            Stop::Hit(action) => self.act(action, now),
            Stop::Dot(i) if self.state.pattern.last() == Some(&i) => self.drawn(now),
            Stop::Dot(i) => {
                if self.state.pattern.is_empty() {
                    self.say(
                        l.pick([
                            "连接完成后，在最后一个点上再按一次电源键",
                            "連接完成後，在最後一個點上再按一次電源鍵",
                            "When the pattern is done, press the power key again on its last dot",
                        ]),
                        now,
                    );
                }
                self.join(&self.theme(), i);
                Vec::new()
            }
        }
    }
}

impl App {
    /// A pattern drawn to its end: four dots or more are sent to be
    /// checked, read as the number of each dot counted from one across the
    /// rows; fewer are wiped and asked for again.
    pub(super) fn drawn(&mut self, now: u64) -> Vec<Effect> {
        let drawn = std::mem::take(&mut self.state.pattern);
        self.repaint_pattern(&self.theme());
        if drawn.len() < 4 {
            self.say(
                self.state
                    .lang
                    .pick(["请至少连接 4 个点", "請至少連接 4 個點", "Connect at least 4 dots"]),
                now,
            );
            return Vec::new();
        }
        self.unlock(drawn.iter().map(|i| (b'1' + *i as u8) as char).collect())
    }
}

impl App {
    /// A volume key: the first lights the control the keys were last on
    /// if it shows, or else the first in view; each after moves the light
    /// down the page or up it, or moves a slider that takes the keys, the
    /// strength slider tapping the motor as strongly as it was moved to.
    pub(super) fn volume(&mut self, down: bool, now: u64) -> Vec<Effect> {
        if self.state.browser.full_active && matches!(self.screen(), Screen::Browser | Screen::BrowserAddress) {
            self.stir(now);
            return self.set_media_volume(self.state.media_volume + if down { -0.05 } else { 0.05 });
        }
        if self.frame.is_none() {
            return Vec::new();
        }
        if self.adjusting {
            if let Some(Spot {
                stop: Stop::Hit(Action::Slider(which)),
                rect,
                ..
            }) = self.lit()
            {
                let level = self.level(which) + if down { -SLIDER_STEP } else { SLIDER_STEP };
                self.set_level(&self.theme(), which, rect, level);
                if which == Slider::Vibration && self.state.vibrate {
                    return vec![Effect::Haptic];
                }
            }
            return Vec::new();
        }
        if self.keys {
            self.step(down, now);
            return Vec::new();
        }
        self.keys = true;
        self.dirty = true;
        let scroll = self.resting();
        let shows = self.lit().is_some_and(|spot| self.shows(&spot, scroll));
        if !shows {
            self.top_mut().2 = Focus::Unset;
            self.lit();
        }
        Vec::new()
    }
}

impl App {
    /// Move the light to the next control down the page, or up it. A
    /// control far off is scrolled to half a screen at a time, the light
    /// staying on its control while that shows; past the last control the
    /// light goes round to the first, and past the first to the last.
    pub(super) fn step(&mut self, down: bool, now: u64) {
        let scroll = self.resting();
        let current = self.lit();
        if self.spots().is_none() {
            return;
        }
        let spots = self.spots.clone().expect("made above");
        let cur = current.and_then(|c| focus::find(&spots, &c));
        let (top, bottom) = self.band(scroll);
        let (max, n) = (self.max_scroll(), spots.len());
        let page = |k: usize| spots[k].group == Group::Page;
        let held = |k: usize| matches!(spots[k].group, Group::Fixed | Group::Strip);
        let docked = |k: usize| spots[k].group == Group::Dock;
        let from_top = |k: usize| page(k) && self.place(&spots[k], scroll).is_some_and(|r| r.y + r.h > top);
        let from_bottom = |k: usize| page(k) && self.place(&spots[k], scroll).is_some_and(|r| r.y < bottom);
        enum To {
            Spot(usize),
            Scroll,
            Round,
        }
        let to = if down {
            let past = || {
                if scroll < max {
                    To::Scroll
                } else {
                    (0..n).find(|&k| docked(k)).map_or(To::Round, To::Spot)
                }
            };
            match cur {
                Some(i) if page(i) => (i + 1..n).find(|&k| page(k)).map_or_else(past, To::Spot),
                Some(i) if docked(i) => (i + 1..n)
                    .find(|&k| docked(k))
                    .map_or(if scroll < max { To::Scroll } else { To::Round }, To::Spot),
                Some(i) => (i + 1..n)
                    .find(|&k| held(k))
                    .or_else(|| (0..n).find(|&k| from_top(k)))
                    .map_or_else(past, To::Spot),
                None => (0..n).find(|&k| from_top(k)).map_or_else(past, To::Spot),
            }
        } else {
            let before = || {
                if scroll > 0.0 {
                    To::Scroll
                } else {
                    (0..n).rev().find(|&k| held(k)).map_or(To::Round, To::Spot)
                }
            };
            match cur {
                Some(i) if page(i) => (0..i).rev().find(|&k| page(k)).map_or_else(before, To::Spot),
                Some(i) if docked(i) => (0..i)
                    .rev()
                    .find(|&k| docked(k))
                    .or_else(|| (0..n).rev().find(|&k| from_bottom(k)))
                    .map_or_else(before, To::Spot),
                Some(i) => (0..i)
                    .rev()
                    .find(|&k| held(k))
                    .map_or(if scroll > 0.0 { To::Scroll } else { To::Round }, To::Spot),
                None => (0..n).rev().find(|&k| from_bottom(k)).map_or_else(before, To::Spot),
            }
        };
        match to {
            To::Spot(k) => {
                let s = spots[k];
                if s.group == Group::Page {
                    let target = self.reveal(&s, scroll);
                    if (target - scroll).abs() > (bottom - top) / 2.0 {
                        return self.scroll_on(down, &spots, cur, now);
                    }
                    self.seek_to(target, now);
                }
                self.top_mut().2 = Focus::On(s);
            }
            To::Scroll => return self.scroll_on(down, &spots, cur, now),
            To::Round if n == 0 => return,
            To::Round => {
                let s = spots[if down { 0 } else { n - 1 }];
                let end = if down { 0.0 } else { max };
                let target = if self.shows(&s, end) { end } else { self.reveal(&s, end) };
                self.seek_to(target, now);
                self.top_mut().2 = Focus::On(s);
            }
        }
        self.dirty = true;
    }
}

impl App {
    /// Scroll the page half a screen down or up for the keys. The light
    /// stays on its control while that shows, and goes to the first in view
    /// the way the page went when it does not.
    pub(super) fn scroll_on(&mut self, down: bool, spots: &[Spot], cur: Option<usize>, now: u64) {
        let scroll = self.resting();
        let (top, bottom) = self.band(scroll);
        let half = (bottom - top) / 2.0;
        let target = if down {
            (scroll + half).min(self.max_scroll())
        } else {
            (scroll - half).max(0.0)
        };
        let target = self.frame.as_ref().map_or(target, |fr| fr.settle(target));
        self.seek_to(target, now);
        if cur.is_some_and(|i| self.shows(&spots[i], target)) {
            return;
        }
        let band = self.band(target);
        let in_view = |k: &usize| spots[*k].group == Group::Page && self.place(&spots[*k], target).is_some_and(|r| within(r, band));
        let next = if down {
            (0..spots.len()).find(in_view)
        } else {
            (0..spots.len()).rev().find(in_view)
        };
        self.top_mut().2 = next.map_or(Focus::Off, |k| Focus::On(spots[k]));
    }
}

impl App {
    /// The page's controls in the order the keys go through them, once the
    /// page is drawn.
    pub(super) fn spots(&mut self) -> Option<&[Spot]> {
        if self.spots.is_none() {
            let t = self.theme();
            self.spots = Some(focus::spots(&t, self.frame.as_ref()?));
        }
        self.spots.as_deref()
    }
}

impl App {
    /// The control the keys are on, once the page is drawn: the one they
    /// were on, found again, or the first in view when they have not been
    /// anywhere on the page yet or theirs has gone from it.
    pub(super) fn lit(&mut self) -> Option<Spot> {
        let scroll = self.resting();
        self.spots()?;
        let spots = self.spots.as_deref().expect("made above");
        let found = match self.top().2 {
            Focus::Off => return None,
            Focus::On(at) => focus::find(spots, &at),
            Focus::Unset => None,
        };
        let spot = spots[found.or_else(|| self.first_in_view(spots, scroll))?];
        self.stack.last_mut().expect("the page stack always holds a page").2 = Focus::On(spot);
        Some(spot)
    }
}

impl App {
    /// The first of the page's own controls in view at `scroll`, or else
    /// the bottom strip's first, or else the `<` or the top strip's first.
    pub(super) fn first_in_view(&self, spots: &[Spot], scroll: f32) -> Option<usize> {
        let band = self.band(scroll);
        let k = spots
            .iter()
            .position(|s| s.group == Group::Page && self.place(s, scroll).is_some_and(|r| within(r, band)));
        k.or_else(|| spots.iter().position(|s| s.group == Group::Dock))
            .or_else(|| spots.iter().position(|s| matches!(s.group, Group::Fixed | Group::Strip)))
    }
}

impl App {
    /// Where on screen `s` is with the page `offset` pixels down; nowhere
    /// for a search bar folded away.
    pub(super) fn place(&self, s: &Spot, offset: f32) -> Option<Rect> {
        let fr = self.frame.as_ref().expect("a control is placed on the page drawn");
        let dy = match s.group {
            Group::Fixed => 0.0,
            Group::Strip => pinned_top(&self.theme(), fr.pinned.as_ref().expect("the strip holds its controls"), offset),
            Group::Page => return fr.shown(s.rect, offset),
            Group::Dock => self.theme().m.h as f32 - fr.dock.as_ref().expect("the dock holds its controls").canvas.h as f32,
        };
        Some(Rect {
            y: s.rect.y + dy,
            ..s.rect
        })
    }
}

impl App {
    /// The band of the screen the page's own controls show in with the page
    /// at `scroll`: under the bar, or under the strip held there, down to
    /// the strip held at the bottom.
    pub(super) fn band(&self, scroll: f32) -> (f32, f32) {
        let t = self.theme();
        let fr = self.frame.as_ref().expect("the band is the page drawn's");
        let top = match &fr.pinned {
            Some(p) => pinned_top(&t, p, scroll) + p.canvas.h as f32,
            None => widgets::bar_bottom(&t),
        };
        (top, t.m.h as f32 - fr.dock.as_ref().map_or(0.0, |d| d.canvas.h as f32))
    }
}

impl App {
    /// Whether `s` shows, in part at least, with the page at `scroll`.
    pub(super) fn shows(&self, s: &Spot, scroll: f32) -> bool {
        s.group != Group::Page || self.place(s, scroll).is_some_and(|r| within(r, self.band(scroll)))
    }
}

impl App {
    /// How far down to scroll the page from `scroll` to show `s` whole, a
    /// margin clear of the band's edges, or as much of it from its top as
    /// fits. A search bar under the large title shows only unfolded, at
    /// the page's top, and a page scrolled down to show any other control
    /// goes on to fold the bar away whole.
    pub(super) fn reveal(&self, s: &Spot, scroll: f32) -> f32 {
        let fr = self.frame.as_ref().expect("a control is revealed on the page drawn");
        let fold = fr.fold.filter(|_| s.group == Group::Page);
        if fold.is_some_and(|f| f.bar == s.rect) {
            return 0.0;
        }
        let margin = self.theme().m.margin;
        let (top, bottom) = self.band(scroll);
        let (top, bottom) = (top + margin, bottom - margin);
        let r = self.place(s, scroll).expect("only a search bar folded away has no place");
        let d = if r.y < top {
            r.y - top
        } else if r.y + r.h > bottom {
            (r.y + r.h - bottom).min(r.y - top)
        } else {
            0.0
        };
        let target = scroll + d;
        let target = fold.map_or(target, |f| if target > 0.0 && target < f.span { f.span } else { target });
        target.clamp(0.0, self.max_scroll())
    }
}

impl App {
    /// The light on the control the keys are on while they work the page:
    /// the part of the screen its control is drawn in, where the light
    /// falls with the page `offset` pixels down, how round it is, and its
    /// tint, stronger while the power key is held down on it.
    pub(super) fn light(&mut self, offset: f32) -> Option<(Group, Rect, f32, Rgba)> {
        if !self.keys {
            return None;
        }
        let spot = self.lit()?;
        let t = self.theme();
        let keypad = self.screen() == Screen::Decrypt && matches!(self.state.data, Data::Locked { keys: Keys::Digits, .. });
        let (r, radius) = focus::lit(&t, spot.stop, self.place(&spot, offset)?, keypad);
        let held = matches!(self.button, Some((_, Press::Power { .. } | Press::Slide { .. })));
        let alpha = if held || self.adjusting { LIGHT_HELD_ALPHA } else { LIGHT_ALPHA };
        Some((spot.group, r, radius, t.pal.accent.with_alpha(alpha)))
    }
}

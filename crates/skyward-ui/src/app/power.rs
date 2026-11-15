use super::*;

impl App {
    pub fn display_on(&self) -> bool {
        self.display_on
    }
}

impl App {
    /// The backlight at `now`: off, or this share of the brightness at the
    /// slider's top, which holds as the content goes dark after the panel
    /// is told to go off.
    pub fn panel(&self, now: u64) -> Option<f32> {
        if !self.display_on {
            return self.darkening.filter(|_| self.curtain.level(now) > 0.0);
        }
        let (full, level) = (self.state.brightness, self.bright.level(now));
        Some(full * level + dim_level(full) * (1.0 - level))
    }
}

impl App {
    /// Something was touched or pressed: the panel stays at full
    /// brightness for the screen-off time from now.
    pub(crate) fn stir(&mut self, now: u64) {
        self.active = Some(now);
        self.bright.set(true, now);
    }
}

impl App {
    /// Light the panel, as if just touched: the content comes up from as
    /// dark as it is.
    pub(super) fn wake(&mut self, now: u64) {
        if !self.display_on {
            self.display_on = true;
            self.dirty = true;
            self.darkening = None;
            self.curtain.set(true, now);
        }
        self.woke_at = Some(now);
        self.tapped = None;
        self.stir(now);
    }
}

impl App {
    /// A charger or a computer was attached, `cable_in`, or taken away: the
    /// dark panel lights, and a lit one stays at full brightness for the
    /// screen-off time from now, as on the phone's own system. A cable
    /// going in brings up the charging screen, over the page or on the
    /// panel lit for it, which then goes dark again.
    pub fn plugged(&mut self, now: u64, cable_in: bool) {
        if cable_in {
            let dark = !self.display_on;
            let mut shown = self.charging.take().map_or(Fade::hidden(CHARGE_SHOW, CHARGE_HIDE), |c| c.shown);
            if dark {
                shown.jump(true);
            } else {
                shown.set(true, now);
            }
            self.charging = Some(Charging {
                since: now,
                shown,
                dark_after: dark,
            });
            self.dirty = true;
        }
        if self.display_on {
            self.stir(now);
        } else {
            self.wake(now);
        }
    }
}

impl App {
    /// The charging screen has gone, and what was drawn for it with it.
    pub(super) fn end_charging(&mut self) {
        self.charging = None;
        self.charge_art = None;
        self.charge_canvas = None;
    }
}

impl App {
    /// Whether the charging screen shows, or is coming up.
    pub(super) fn charge_shown(&self) -> bool {
        self.charging.as_ref().is_some_and(|c| c.shown.on)
    }
}

impl App {
    /// Send the charging screen away for the page, and keep the panel lit.
    pub(super) fn dismiss_charging(&mut self, now: u64) {
        if let Some(c) = &mut self.charging {
            c.shown.set(false, now);
            c.dark_after = false;
        }
        self.stir(now);
    }
}

impl App {
    /// Whether the panel lit so recently that a touch or key should only
    /// keep it awake.
    pub(super) fn just_woke(&self, now: u64) -> bool {
        self.woke_at.is_some_and(|w| now.saturating_sub(w) < WAKE_GUARD_MS)
    }
}

impl App {
    /// Turn the panel off: the content goes dark under the backlight it has
    /// now, then the panel goes off. A finger on the glass lets go of what
    /// it held, and the page stops where it is.
    pub(super) fn sleep(&mut self, now: u64) {
        self.darkening = self.panel(now);
        self.curtain.set(false, now);
        self.display_on = false;
        self.bright = full_brightness();
        match std::mem::replace(&mut self.drag, Drag::Idle) {
            Drag::Confirm { .. } => {
                self.state.confirm = 0.0;
                self.refresh();
            }
            Drag::Pattern => {
                self.state.pattern.clear();
                self.refresh();
            }
            _ => {}
        }
        self.drop_button();
        self.adjusting = false;
        self.fling = None;
        self.bounce = None;
        self.seek = None;
        self.pull = 0.0;
        self.cue = None;
        self.morph = None;
        self.row_press = None;
        self.row_art = None;
        self.end_turn();
    }
}

impl App {
    /// A page sliding over another comes to rest at once.
    pub(super) fn end_turn(&mut self) {
        if let Some(turn) = self.turn.take() {
            self.backdrop = Some(turn.left);
            self.dirty = true;
        }
    }
}

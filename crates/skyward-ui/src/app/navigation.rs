use super::*;

pub const NAV_DP: f32 = 36.0;

impl App {
    pub(super) fn nav_height(&self) -> u32 {
        if !self.navigation || self.screen() == Screen::Fastboot {
            0
        } else {
            (self.px_per_dp * NAV_DP).round() as u32
        }
    }

    pub fn set_navigation(&mut self, enabled: bool) {
        self.navigation = enabled;
        self.settle_page();
        self.refresh();
    }

    pub(super) fn home(&mut self) {
        if self.shade.is_some() {
            self.dismiss_shade();
        }
        if self.screen() == Screen::Home {
            return;
        }
        if self.screen() != Screen::Tasks {
            self.parked = Some(self.stack.clone());
        }
        self.end_turn();
        self.stack = vec![(Screen::Home, 0.0, Focus::Unset)];
        self.settle_page();
        self.refresh();
    }

    pub(super) fn resume(&mut self) {
        if let Some(stack) = self.parked.take() {
            self.end_turn();
            self.stack = stack;
            self.settle_page();
            self.refresh();
        }
    }

    pub(super) fn navigation_touch(&mut self, ev: Touch, now: u64) -> Option<Vec<Effect>> {
        let height = self.nav_height();
        if height == 0 {
            return None;
        }
        let top = (self.h - height) as f32;
        if let Touch::Down(x, y) = ev
            && y >= top
        {
            let first_press = self.nav_press.is_none();
            self.nav_press = Some((x / (self.w as f32 / 3.0)).floor().clamp(0.0, 2.0) as u8);
            self.drag = Drag::Idle;
            self.stir(now);
            self.dirty = true;
            return Some(if first_press && self.state.vibrate {
                vec![Effect::Haptic]
            } else {
                Vec::new()
            });
        }
        if let Some(button) = self.nav_press {
            match ev {
                Touch::Up => {
                    self.nav_press = None;
                    self.dirty = true;
                    return Some(self.act(
                        match button {
                            0 => Action::NavBack,
                            1 => Action::NavHome,
                            _ => Action::NavTasks,
                        },
                        now,
                    ));
                }
                Touch::Move(_, y) if y < top => {
                    self.nav_press = None;
                    return Some(Vec::new());
                }
                _ => return Some(Vec::new()),
            }
        }
        None
    }

    pub(super) fn draw_navigation(&self, out: &mut Canvas) {
        let height = self.nav_height();
        if height == 0 {
            return;
        }
        let t = self.theme();
        let y = (self.h - height) as f32;
        let cy = y + height as f32 / 2.0;
        out.fill_rect(0.0, y, self.w as f32, height as f32, t.pal.bg);
        out.fill_rect(0.0, y, self.w as f32, 1.0, t.pal.track);
        for index in 0..3 {
            let x = self.w as f32 * (index as f32 + 0.5) / 3.0;
            let ink = if self.nav_press == Some(index) { t.pal.accent } else { t.pal.text };
            let size = t.m.dp(6.0);
            let stroke = t.m.dp(1.5);
            match index {
                0 => {
                    out.line(x + size / 2.0, cy - size, x - size / 2.0, cy, stroke, ink);
                    out.line(x - size / 2.0, cy, x + size / 2.0, cy + size, stroke, ink);
                }
                1 => out.stroke_arc(x, cy, size, 0.0, std::f32::consts::TAU, stroke, ink),
                _ => out.stroke_round_rect(x - size, cy - size, size * 2.0, size * 2.0, t.m.dp(1.0), stroke, ink),
            }
        }
    }
}

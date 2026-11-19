use super::*;

pub const NAV_DP: f32 = 32.0;
const GROUP_DP: f32 = 252.0;

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
        self.settle_page();
        self.refresh();
    }

    pub(super) fn navigation_buttons(&self) -> [Rect; 3] {
        let width = (self.px_per_dp * GROUP_DP).min(self.w as f32);
        let cell = width / 3.0;
        let left = (self.w as f32 - width) / 2.0;
        let height = self.nav_height() as f32;
        std::array::from_fn(|index| Rect::new(left + index as f32 * cell, self.h as f32 - height, cell, height))
    }

    pub(super) fn home(&mut self) {
        self.state.ime.clear();
        self.state.ime.enabled = false;
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
            self.nav_press = self
                .navigation_buttons()
                .iter()
                .position(|rect| rect.contains(x, y))
                .map(|index| index as u8);
            self.drag = Drag::Idle;
            self.stir(now);
            self.dirty = true;
            return Some(if first_press && self.nav_press.is_some() && self.state.vibrate {
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
                Touch::Move(x, y) if !self.navigation_buttons()[button as usize].contains(x, y) => {
                    self.nav_press = None;
                    self.dirty = true;
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
        let background = if self.browser_fullscreen() {
            Rgba::rgb(0, 0, 0).with_alpha(155)
        } else {
            t.pal.bg
        };
        out.fill_rect(0.0, y, self.w as f32, height as f32, background);
        for (index, rect) in self.navigation_buttons().iter().enumerate() {
            let (x, cy) = rect.centre();
            let ink = if self.nav_press == Some(index as u8) {
                t.pal.accent
            } else {
                t.pal.text
            };
            let size = t.m.dp(6.0);
            let stroke = t.m.dp(1.5);
            match index {
                0 => out.fill_convex(&[(x - size, cy), (x + size * 0.65, cy - size), (x + size * 0.65, cy + size)], ink),
                1 => out.stroke_circle(x, cy, size, stroke, ink),
                _ => out.stroke_round_rect(x - size * 0.85, cy - size * 0.85, size * 1.7, size * 1.7, t.m.dp(0.6), stroke, ink),
            }
        }
    }
}

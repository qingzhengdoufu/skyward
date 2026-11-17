use super::*;

impl App {
    pub(super) fn volume_rect(&self) -> Rect {
        let m = self.theme().m;
        Rect::new(m.dp(108.0), m.dp(9.0), m.dp(70.0).min(self.w as f32 * 0.2), m.dp(34.0))
    }

    pub(super) fn media_volume_touch(&mut self, event: Touch, now: u64) -> Option<Vec<Effect>> {
        if !self.state.browser.full_active || !matches!(self.screen(), Screen::Browser | Screen::BrowserAddress) || self.shade.is_some() {
            self.volume_drag = false;
            return None;
        }
        let rect = self.volume_rect();
        let x = match event {
            Touch::Down(x, y) if rect.contains(x, y) => {
                self.volume_drag = true;
                self.drag = Drag::Idle;
                x
            }
            Touch::Move(x, _) if self.volume_drag => x,
            Touch::Up if self.volume_drag => {
                self.volume_drag = false;
                return Some(Vec::new());
            }
            _ => return None,
        };
        self.stir(now);
        let left = rect.x + self.theme().m.dp(15.0);
        let width = rect.w - self.theme().m.dp(16.0);
        Some(self.set_media_volume((x - left) / width))
    }

    pub(super) fn set_media_volume(&mut self, volume: f32) -> Vec<Effect> {
        let percent = (volume.clamp(0.0, 1.0) * 100.0).round() as u8;
        self.state.media_volume = f32::from(percent) / 100.0;
        self.dirty = true;
        vec![Effect::SetMediaVolume(percent)]
    }

    pub(super) fn draw_media_volume(&self, canvas: &mut Canvas) {
        if !self.state.browser.full_active
            || !matches!(self.screen(), Screen::Browser | Screen::BrowserAddress)
            || self.shade.is_some()
            || self.charge_shown()
        {
            return;
        }
        let t = self.theme();
        let rect = self.volume_rect();
        let m = t.m;
        let x = rect.x;
        let cy = rect.centre().1;
        let ink = t.pal.status;
        canvas.fill_convex(
            &[
                (x, cy - m.dp(2.0)),
                (x + m.dp(4.0), cy - m.dp(2.0)),
                (x + m.dp(8.0), cy - m.dp(6.0)),
                (x + m.dp(8.0), cy + m.dp(6.0)),
                (x + m.dp(4.0), cy + m.dp(2.0)),
                (x, cy + m.dp(2.0)),
            ],
            ink,
        );
        let left = x + m.dp(15.0);
        let width = rect.w - m.dp(16.0);
        let height = m.dp(3.0);
        canvas.fill_round_rect(left, cy - height / 2.0, width, height, height / 2.0, ink.with_alpha(70));
        let level = self.state.media_volume.clamp(0.0, 1.0);
        canvas.fill_round_rect(left, cy - height / 2.0, width * level, height, height / 2.0, ink);
        canvas.fill_circle(left + width * level, cy, m.dp(3.5), ink);
    }
}

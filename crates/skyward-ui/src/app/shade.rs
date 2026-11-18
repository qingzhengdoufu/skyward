use super::*;

pub(super) struct Shade {
    pub under: Canvas,
    pub frost: Canvas,
    pub extent: f32,
    pub spring: Option<(f32, Spring)>,
}

#[derive(Clone, Copy)]
pub(super) struct ShadePull {
    pub start: f32,
    pub last: f32,
    pub from: f32,
    pub at: u64,
    pub velocity: f32,
}

impl Shade {
    pub fn extent(&self, now: u64, height: f32) -> f32 {
        self.spring
            .map_or(self.extent, |(target, spring)| target + spring.at(now).0)
            .clamp(0.0, height)
    }
}

impl App {
    pub(super) fn control_hold(&self, action: Action) -> Option<Action> {
        if self.screen() != Screen::ControlCenter {
            return None;
        }
        Some(match action {
            Action::Toggle(Switch::Wlan) => Action::Open(Screen::Wlan),
            Action::Toggle(Switch::Vibrate | Switch::FileTransfer) => Action::Open(Screen::Settings),
            Action::Toggle(Switch::DoubleTap) => Action::Open(Screen::ScreenOff),
            Action::Open(
                screen @ (Screen::Browser
                | Screen::BrowserDownloads
                | Screen::Files
                | Screen::Terminal
                | Screen::RootModules
                | Screen::Partitions
                | Screen::Settings
                | Screen::Wlan),
            ) => Action::Open(screen),
            Action::NavTasks => Action::NavTasks,
            _ => return None,
        })
    }

    pub(super) fn open_shade(&mut self, now: u64, animated: bool) {
        if self.screen() == Screen::ControlCenter {
            return;
        }
        self.end_turn();
        self.leave(false);
        let under = self
            .turn
            .take()
            .map(|turn| turn.left)
            .unwrap_or_else(|| Canvas::filled(self.w, self.h, self.theme().pal.bg));
        self.push(Screen::ControlCenter);
        let frost = frosted(&under);
        self.shade = Some(Shade {
            under,
            frost,
            extent: 0.0,
            spring: None,
        });
        if animated {
            self.settle_shade(true, now, 0.0);
        }
    }

    pub(super) fn settle_shade(&mut self, open: bool, now: u64, velocity: f32) {
        let height = self.h as f32;
        if let Some(shade) = &mut self.shade {
            let current = shade.extent(now, height);
            let target = if open { height } else { 0.0 };
            shade.extent = current;
            shade.spring = Some((target, Spring::new(now, current - target, velocity, 0.3)));
            self.dirty = true;
        }
        self.shade_pull = None;
        self.drag = Drag::Idle;
    }

    pub(super) fn dismiss_shade(&mut self) {
        self.shade = None;
        self.shade_pull = None;
        if self.screen() == Screen::ControlCenter && self.stack.len() > 1 {
            self.stack.pop();
        }
        self.turn = None;
        self.settle_page();
        self.refresh();
    }

    pub(super) fn tick_shade(&mut self, now: u64) {
        if let Some((target, spring)) = self.shade.as_ref().and_then(|s| s.spring) {
            self.dirty = true;
            if spring.at(now).1 {
                if target == 0.0 {
                    self.dismiss_shade();
                } else if let Some(shade) = &mut self.shade {
                    shade.extent = target;
                    shade.spring = None;
                }
            }
        }
    }

    pub(super) fn shade_touch(&mut self, ev: Touch, now: u64) -> bool {
        if self.screen() == Screen::Fastboot {
            return false;
        }
        let height = self.h as f32;
        let slop = self.theme().m.dp(8.0);
        if let Some(mut pull) = self.shade_pull {
            match ev {
                Touch::Move(_, y) => {
                    if self.shade.is_none() && y - pull.start > slop {
                        self.open_shade(now, false);
                    }
                    if let Some(shade) = &mut self.shade {
                        shade.spring = None;
                        shade.extent = (pull.from + y - pull.start).clamp(0.0, height);
                        self.dirty = true;
                    }
                    let dt = now.saturating_sub(pull.at);
                    if dt > 0 {
                        pull.velocity = (y - pull.last) * 1000.0 / dt as f32;
                    }
                    pull.last = y;
                    pull.at = now;
                    self.shade_pull = Some(pull);
                }
                Touch::Up => {
                    self.shade_pull = None;
                    if let Some(shade) = &self.shade {
                        let travel = pull.last - pull.start;
                        let velocity = if now.saturating_sub(pull.at) < 100 { pull.velocity } else { 0.0 };
                        let open = if velocity.abs() > self.theme().m.dp(400.0) {
                            velocity > 0.0
                        } else if pull.from > 0.0 {
                            travel > -self.theme().m.dp(48.0)
                        } else {
                            shade.extent > self.theme().m.dp(56.0)
                        };
                        self.settle_shade(open, now, velocity);
                    }
                }
                Touch::Down(..) => {}
            }
            self.stir(now);
            return true;
        }
        if let Touch::Down(_, y) = ev {
            let grab = if self.shade.is_some() {
                y < self.theme().m.dp(72.0) || y > height - self.theme().m.dp(32.0)
            } else {
                y < self.theme().m.dp(48.0)
            };
            if grab {
                let from = self.shade.as_ref().map_or(0.0, |s| s.extent(now, height));
                if let Some(shade) = &mut self.shade {
                    shade.spring = None;
                    shade.extent = from;
                }
                self.shade_pull = Some(ShadePull {
                    start: y,
                    last: y,
                    from,
                    at: now,
                    velocity: 0.0,
                });
                self.drag = Drag::Idle;
                self.stir(now);
                return true;
            }
        }
        if self.shade.is_some()
            && let Touch::Move(_, y) = ev
            && let Drag::Press {
                y: start,
                action,
                edge: None,
                ..
            } = self.drag
            && y - start < -slop
            && self.top().1 <= 0.0
            && !matches!(action, Some(Action::Slider(_)))
        {
            let from = self.shade.as_ref().unwrap().extent(now, height);
            self.shade_pull = Some(ShadePull {
                start,
                last: y,
                from,
                at: now,
                velocity: 0.0,
            });
            if let Some(shade) = &mut self.shade {
                shade.spring = None;
                shade.extent = (from + y - start).clamp(0.0, height);
            }
            self.drag = Drag::Idle;
            self.row_press = None;
            self.row_art = None;
            self.dirty = true;
            self.stir(now);
            return true;
        }
        self.shade.as_ref().is_some_and(|s| s.spring.is_some())
    }
}

/// Blur a small snapshot once per pull, not the live page on every animation frame.
fn frosted(source: &Canvas) -> Canvas {
    let mut out = source.clone();
    if !source.whole() || source.w == 0 || source.h == 0 {
        return out;
    }
    let (w, h) = (source.w.div_ceil(8) as usize, source.h.div_ceil(8) as usize);
    let mut small = vec![[0u8; 3]; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u32; 3];
            let mut n = 0;
            for sy in y * 8..((y + 1) * 8).min(source.h as usize) {
                for sx in x * 8..((x + 1) * 8).min(source.w as usize) {
                    let at = (sy * source.w as usize + sx) * 4;
                    for k in 0..3 {
                        sum[k] += u32::from(source.data[at + k]);
                    }
                    n += 1;
                }
            }
            small[y * w + x] = sum.map(|v| (v / n) as u8);
        }
    }
    let mut scratch = small.clone();
    for _ in 0..3 {
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 3];
                for dx in -3isize..=3 {
                    let px = small[y * w + x.saturating_add_signed(dx).min(w - 1)];
                    for k in 0..3 {
                        sum[k] += u32::from(px[k]);
                    }
                }
                scratch[y * w + x] = sum.map(|v| (v / 7) as u8);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 3];
                for dy in -3isize..=3 {
                    let px = scratch[y.saturating_add_signed(dy).min(h - 1) * w + x];
                    for k in 0..3 {
                        sum[k] += u32::from(px[k]);
                    }
                }
                small[y * w + x] = sum.map(|v| (v / 7) as u8);
            }
        }
    }
    for y in 0..source.h as usize {
        for x in 0..source.w as usize {
            let (sx, sy) = (x / 8, y / 8);
            let (fx, fy) = (x % 8, y % 8);
            let (nx, ny) = ((sx + 1).min(w - 1), (sy + 1).min(h - 1));
            let at = (y * source.w as usize + x) * 4;
            for k in 0..3 {
                let top = usize::from(small[sy * w + sx][k]) * (8 - fx) + usize::from(small[sy * w + nx][k]) * fx;
                let bottom = usize::from(small[ny * w + sx][k]) * (8 - fx) + usize::from(small[ny * w + nx][k]) * fx;
                out.data[at + k] = ((top * (8 - fy) + bottom * fy) / 64) as u8;
            }
            out.data[at + 3] = 255;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frost_preserves_flat_color_and_blends_edges_without_changing_source() {
        let color = Rgba::rgb(70, 110, 190);
        for (w, h) in [(1, 1), (17, 31), (64, 32)] {
            let flat = Canvas::filled(w, h, color);
            assert_eq!(frosted(&flat).data, flat.data);
        }
        let mut source = Canvas::filled(64, 32, Rgba::rgb(0, 0, 0));
        source.fill_rect(32.0, 0.0, 32.0, 32.0, Rgba::rgb(255, 255, 255));
        let frost = frosted(&source);
        let boundary = (16 * 64 + 32) * 4;
        assert_eq!(source.data[boundary], 255);
        assert!((40..215).contains(&frost.data[boundary]));
        assert!(frost.data.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }
}

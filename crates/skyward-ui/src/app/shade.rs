use super::*;

pub(super) struct Shade {
    pub under: Canvas,
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
        self.shade = Some(Shade {
            under,
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

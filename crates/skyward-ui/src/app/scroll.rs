use super::*;

impl App {
    /// Where the page comes to rest: where the keys are scrolling it to, or
    /// where it is.
    pub(super) fn resting(&self) -> f32 {
        self.seek.map_or(self.top().1, |(target, _)| target)
    }
}

impl App {
    /// A page come to rest with its search bar part folded eases on from
    /// there to fold the bar away or unfold it whole, whichever is nearer.
    pub(super) fn settle_fold(&mut self, now: u64) {
        let Some(fr) = self.frame.as_ref() else { return };
        let at = self.top().1;
        let rest = fr.settle(at).min(self.max_scroll());
        if rest != at {
            self.seek = Some((rest, Spring::new(now, at - rest, 0.0, FOLD_PERIOD)));
            self.dirty = true;
        }
    }
}

impl App {
    /// Scroll the page to `target` for the keys, easing there.
    pub(super) fn seek_to(&mut self, target: f32, now: u64) {
        let at = self.top().1;
        self.fling = None;
        self.seek = (target != at).then(|| (target, Spring::new(now, at - target, 0.0, SEEK_PERIOD)));
        self.dirty = true;
    }
}

impl App {
    /// Whether the page is gliding, springing back, or easing on of its own
    /// accord at `now`.
    pub(super) fn in_motion(&self, now: u64) -> bool {
        self.fling.is_some_and(|fl| !fl.at(now).1)
            || self.bounce.is_some_and(|b| !b.at(now).1)
            || self.seek.is_some_and(|(_, s)| !s.at(now).1)
    }
}

impl App {
    /// Stop a page gliding, springing back or easing on of its own accord,
    /// where it has got to, for a finger to hold: one past its end stays
    /// out there as the finger's pull. Whether there was one.
    pub(super) fn catch(&mut self, now: u64) -> bool {
        let glide = self.fling.take().is_some_and(|fl| !fl.at(now).1);
        let seeking = self.seek.take().is_some_and(|(_, s)| !s.at(now).1);
        let spring = self.bounce.take().map(|b| b.at(now)).filter(|&(_, done)| !done);
        if let Some((offset, _)) = spring {
            self.pull = motion::unrubber(offset, self.h as f32);
        }
        glide || seeking || spring.is_some()
    }
}

impl App {
    /// Move the page `d` pixels under a dragging finger (positive moves it
    /// up). Past an end the finger's travel goes into the pull, which the
    /// page follows with growing resistance; coming back undoes the pull
    /// before the page scrolls again.
    pub(super) fn drag_by(&mut self, d: f32) {
        self.dirty = true;
        let mut d = d;
        if self.pull != 0.0 {
            let pull = self.pull + d;
            if pull * self.pull > 0.0 {
                self.pull = pull;
                return;
            }
            self.pull = 0.0;
            d = pull;
        }
        let max = self.max_scroll();
        let top = self.top_mut();
        let next = top.1 + d;
        top.1 = next.clamp(0.0, max);
        self.pull = next - top.1;
    }
}

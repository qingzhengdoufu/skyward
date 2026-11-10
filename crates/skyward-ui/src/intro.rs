//! One-shot startup overlay; the existing event loop drives it without sleeps.

use crate::{sprite::Sprite, widgets, Canvas, Fonts, Rgba, Theme};

pub const DURATION_MS: u64 = 680;

fn smooth(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

/// Wordmark opacity and page opacity. They never overlap, even during the
/// handover, so the mark cannot obscure an actionable page underneath it.
pub fn levels(elapsed: u64) -> (f32, f32) {
    match elapsed {
        0..=279 => (smooth(elapsed as f32 / 280.0), 0.0),
        280..=399 => (1.0, 0.0),
        400..=509 => (1.0 - smooth((elapsed - 400) as f32 / 110.0), 0.0),
        510..=679 => (0.0, smooth((elapsed - 510) as f32 / 170.0)),
        _ => (0.0, 1.0),
    }
}

pub struct Intro {
    mark: Sprite,
    began: u64,
}

impl Intro {
    pub fn new(theme: &Theme, fonts: &Fonts, now: u64) -> Self {
        let mark = widgets::wordmark(theme, fonts, "Skyward", theme.m.h as f32 * 0.43, theme.m.dp(42.0));
        Self { mark, began: now }
    }

    pub fn active(&self, now: u64) -> bool {
        now.saturating_sub(self.began) < DURATION_MS
    }

    /// Composite on the already-rendered page in place; no extra full-screen
    /// image, file I/O, device writes or allocation is needed per frame.
    pub fn draw(&self, page: &mut Canvas, now: u64) {
        let (mark, page_opacity) = levels(now.saturating_sub(self.began));
        if page_opacity >= 1.0 { return; }
        if page_opacity == 0.0 {
            page.clear(Rgba::rgb(0, 0, 0));
        } else {
            let weight = (page_opacity * 256.0).round() as u32;
            for pixel in page.data.as_chunks_mut::<4>().0 {
                for channel in &mut pixel[..3] { *channel = ((u32::from(*channel) * weight + 128) >> 8) as u8; }
                pixel[3] = 255;
            }
        }
        self.mark.draw(page, 0.0, 0.0, Rgba::rgb(245, 245, 247), mark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_phases_never_overlay_wordmark_and_page() {
        assert_eq!(levels(0), (0.0, 0.0));
        assert_eq!(levels(280), (1.0, 0.0));
        assert_eq!(levels(510), (0.0, 0.0));
        assert_eq!(levels(DURATION_MS), (0.0, 1.0));
        assert_eq!(levels(u64::MAX), (0.0, 1.0));
        for ms in 0..1000 {
            let (mark, page) = levels(ms);
            assert!((0.0..=1.0).contains(&mark) && (0.0..=1.0).contains(&page));
            assert!(mark == 0.0 || page == 0.0);
        }
    }

    #[test]
    fn animation_is_nonblank_at_peak_and_does_not_modify_a_finished_page() {
        let fonts = crate::font::bundled();
        for (w, h, dpi) in [(720, 1600, 300), (1080, 2400, 440), (1440, 3200, 600)] {
            let theme = Theme::dark(w, h, dpi as f32 / 160.0);
            let intro = Intro::new(&theme, &fonts, 100);
            let mut page = Canvas::filled(w, h, Rgba::rgb(55, 80, 120));
            let before = page.data.clone();
            intro.draw(&mut page, 100 + DURATION_MS);
            assert_eq!(page.data, before);
            assert!(!intro.active(100 + DURATION_MS));
            intro.draw(&mut page, 440);
            let lit: Vec<_> = page.data.as_chunks::<4>().0.iter().enumerate().filter(|(_, pixel)| pixel[0] > 0).collect();
            assert!(!lit.is_empty());
            assert!(lit.len() < (w * h / 10) as usize);
            assert!(lit.iter().all(|(index, _)| {
                let x = *index as u32 % w;
                let y = *index as u32 / w;
                x > w / 10 && x < w * 9 / 10 && y > h / 3 && y < h * 2 / 3
            }));
        }
    }
}

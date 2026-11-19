//! Shared, offline Chinese/English composition strip.
use crate::app::{Action, Hit};
use crate::geom::Rect;
use crate::pinyin::Composition;
use crate::{Canvas, Fonts, Theme};

pub fn height(t: &Theme) -> f32 {
    if cfg!(feature = "pinyin-input") { t.m.dp(44.0) } else { 0.0 }
}

pub fn bar(canvas: &mut Canvas, t: &Theme, f: &Fonts, ime: &Composition, top: f32, browser: bool) -> Vec<Hit> {
    let m = &t.m;
    canvas.fill_rect(0.0, top, m.w as f32, m.dp(44.0), t.pal.bg);
    let mut out = Vec::new();
    let toggle = Rect::new(m.dp(8.0), top, m.dp(52.0), m.dp(44.0));
    canvas.text_center(
        &f.heading,
        if ime.enabled { "中" } else { "EN" },
        toggle.centre().0,
        toggle.centre().1,
        m.dp(16.0),
        t.pal.accent,
    );
    out.push(Hit {
        rect: toggle,
        action: if browser { Action::BrowserImeToggle } else { Action::ImeToggle },
    });
    let right = m.w as f32 - m.dp(44.0);
    let cell = (right - m.dp(64.0)) / 3.0;
    for (offset, candidate) in ime.candidates.iter().skip(ime.page * 3).take(3).enumerate() {
        let rect = Rect::new(m.dp(64.0) + offset as f32 * cell, top, cell, m.dp(44.0));
        canvas.text_vc(
            &f.body,
            &candidate.text,
            rect.x + m.dp(6.0),
            rect.y + m.dp(30.0),
            m.dp(17.0),
            t.pal.text,
            Some(cell - m.dp(12.0)),
        );
        out.push(Hit {
            rect,
            action: if browser {
                Action::BrowserImePick(ime.page * 3 + offset)
            } else {
                Action::ImePick(ime.page * 3 + offset)
            },
        });
    }
    if !ime.spelling.is_empty() && !ime.candidates.is_empty() {
        canvas.text_top(
            &f.value,
            &ime.spelling,
            m.dp(70.0),
            top + m.dp(2.0),
            m.dp(10.0),
            t.pal.subtitle,
            Some(right - m.dp(76.0)),
        );
    }
    if ime.candidates.is_empty() && !ime.spelling.is_empty() {
        canvas.text_vc(
            &f.body,
            &ime.spelling,
            m.dp(70.0),
            top + m.dp(22.0),
            m.dp(16.0),
            t.pal.text,
            Some(right - m.dp(76.0)),
        );
    }
    if ime.candidates.len() > 3 {
        let rect = Rect::new(right, top, m.dp(44.0), m.dp(44.0));
        let (x, y) = rect.centre();
        canvas.line(x - m.dp(3.0), y - m.dp(6.0), x + m.dp(3.0), y, m.dp(2.0), t.pal.text);
        canvas.line(x + m.dp(3.0), y, x - m.dp(3.0), y + m.dp(6.0), m.dp(2.0), t.pal.text);
        out.push(Hit {
            rect,
            action: if browser {
                Action::BrowserImePage(true)
            } else {
                Action::ImePage(true)
            },
        });
    }
    out
}

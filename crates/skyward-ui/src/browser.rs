use crate::app::{App, State};
use crate::Canvas;
use crate::{geom::Rect, theme::Theme};

pub fn viewport(t: &Theme, keyboard: bool) -> Rect {
    let top = t.m.dp(220.0);
    let bottom = if keyboard { t.m.h as f32 - crate::widgets::keyboard_height(t) } else { t.m.h as f32 };
    let height = (bottom - top - t.m.dp(8.0)).max(1.0);
    let width = t.m.w as f32 - t.m.margin * 2.0;
    Rect::new(t.m.margin, top, width, height.min(width * 2100.0 / 1080.0))
}

/// The visible slice of the fixed 360x700 CSS page. Keep the same scale
/// with the keyboard open, moving only far enough to reveal the tapped field.
pub fn source_top(view: Rect, keyboard: bool, focus_y: f32) -> f32 {
    if !keyboard { return 0.0; }
    let visible = view.h * 360.0 / view.w;
    (focus_y + 48.0 - visible).clamp(0.0, (700.0 - visible).max(0.0))
}

pub fn page_point(view: Rect, top: f32, x: f32, y: f32) -> (i32,i32) {
    (((x-view.x) * 360.0 / view.w).clamp(0.0,359.0) as i32,
     (top + (y-view.y) * 360.0 / view.w).clamp(0.0,699.0) as i32)
}

pub fn draw_pixels(canvas: &mut Canvas, t: &Theme, state: &State) {
    let Some(pixels) = &state.browser_pixels else { return };
    let view = viewport(t,state.browser_keyboard);
    let top = source_top(view,state.browser_keyboard,state.browser_focus_y)*3.0;
    let rows = canvas.held();
    let cols = canvas.cols();
    let x0 = (view.x as u32).max(cols.start);
    let x1 = ((view.x+view.w) as u32).min(cols.end);
    if x0>=x1 { return; }
    let source_x: Vec<usize> = (x0..x1).map(|x| (((x as f32-view.x)/view.w*1080.0) as usize).min(1079)*4).collect();
    for y in (view.y as u32).max(rows.start)..((view.y+view.h) as u32).min(rows.end) {
        let sy = ((top+(y as f32-view.y)/view.w*1080.0) as usize).min(2099);
        let source = &pixels[sy*1080*4..(sy+1)*1080*4];
        let into = ((y-rows.start)*(cols.end-cols.start)+x0-cols.start) as usize*4;
        let out = &mut canvas.data[into..into+(x1-x0) as usize*4];
        for (pixel,from) in out.as_chunks_mut::<4>().0.iter_mut().zip(&source_x) {
            *pixel=[source[from+2],source[from+1],source[*from],255];
        }
    }
}

impl App {
    pub fn browser_focus(&mut self, y: i32) {
        if (self.state.browser_focus_y-y as f32).abs()>0.5 {
            self.state.browser_focus_y=y as f32;
            if self.state.browser_keyboard { self.refresh(); }
        }
    }
    pub fn browser_activity(&mut self, now: u64) { self.stir(now); }
    pub fn browser_started(&mut self) {
        self.state.browser_live = true;
        self.state.browser_keyboard = false;
        self.state.browser_address_edit = false;
        self.state.browser_status.clear();
        self.state.browser_checking = false;
        self.refresh();
    }

    pub fn browser_stopped(&mut self, why: String) {
        self.state.browser_live = false;
        self.state.browser_pixels = None;
        self.browser_checked(why);
    }

    pub fn browser_closed(&mut self) {
        self.state.browser_checking = false;
        self.state.browser_live = false;
        self.state.browser_pixels = None;
        self.state.browser_keyboard = true;
        self.state.browser_address_edit = true;
        self.state.browser_progress = 0;
        self.refresh();
    }

    pub fn browser_event(&mut self, message: skyward_browser::protocol::Message) {
        let mut changed=false;
        match message.kind {
            33 => {
                changed |= self.state.browser_progress != message.value;
                self.state.browser_progress = message.value;
                if !message.text.is_empty() && !self.state.browser_address_edit && self.state.browser_address != message.text {
                    self.state.browser_address = message.text;
                    changed=true;
                }
            }
            34 => { changed=self.state.browser_status!=message.text; self.state.browser_status = message.text; }
            35 => { changed=!self.state.browser_keyboard || self.state.browser_address_edit; self.state.browser_keyboard = true; self.state.browser_address_edit = false; }
            36 => { changed=self.state.browser_keyboard; self.state.browser_keyboard = false; }
            _ => {}
        }
        if changed { self.refresh(); }
    }

    pub fn browser_checked(&mut self, status: String) {
        self.state.browser_checking = false;
        self.state.browser_status = status;
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Screen;

    #[test]
    fn browser_state_is_separate_from_installation_and_avb() {
        let mut app = App::new(Screen::Browser, 1440, 3200, 3.75);
        let before = app.settings();
        app.browser_checked("sandbox unavailable".into());
        assert_eq!(app.settings(), before);
        assert!(!app.state.browser_checking);
        assert_eq!(app.state.browser_status, "sandbox unavailable");
    }
}

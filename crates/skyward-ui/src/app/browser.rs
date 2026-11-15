use super::*;

impl App {
    pub fn browser_frame(&mut self,pixels: Vec<u8>) {
        if pixels.len()!=skyward_browser::protocol::FRAME_BYTES { return; }
        self.state.browser_pixels=Some(pixels.into());
        if self.screen()==Screen::Browser {
            let theme=self.theme();
            if let Some(frame)=&mut self.frame {
                crate::browser::draw_pixels(&mut frame.canvas,&theme,&self.state);
            }
            self.layer=None;
            self.dirty=true;
        }
    }
}

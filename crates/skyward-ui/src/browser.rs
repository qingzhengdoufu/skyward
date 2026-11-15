use crate::app::App;

impl App {
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

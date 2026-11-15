use super::*;
use skyward_browser::{protocol::Message, session::Browser};

impl Pages<'_> {
    pub(super) fn open_browser(&mut self, address: String, now: u64) {
        if self.browser.is_some() {
            let mut nav = Message::command(1);
            nav.text = address;
            self.browser_send(nav, now);
            if self.browser.is_some() { self.app.browser_started(); }
            return;
        }
        if self.browser_start.is_some() { return; }
        if !self.app.state.data.storage_open() {
            self.app.browser_checked(self.app.state.lang.pick(["请先解密 data","請先解密 data","Decrypt data first"]).into());
            return;
        }
        let dns = self.app.state.wlan.as_ref().and_then(|w| w.joined.as_ref()).and_then(|j| match &j.stage {
            Joining::Connected(link) => Some(link.dns.clone()),
            _ => None,
        }).unwrap_or_default();
        if dns.is_empty() {
            self.app.browser_checked(self.app.state.lang.pick(["请先连接 WLAN", "請先連接 WLAN", "Connect WLAN first"]).into());
            return;
        }
        if self.installer.is_some() || self.partition_busy() {
            self.partition_wait(now);
            return;
        }
        let cfg = self.cfg.clone();
        match skyward_sys::worker::Worker::start("browser-start", move |tell| {
            tell.tell(crate::browser::prepare(&cfg).map_err(|e| format!("{e:#}")));
        }) {
            Ok(worker) => {
                self.browser_address = address;
                self.browser_dns = dns;
                self.browser_start = Some(worker);
                self.app.state.browser_checking = true;
                self.app.refresh();
            }
            Err(e) => self.app.browser_checked(format!("{e:#}")),
        }
    }

    pub(super) fn browser_send(&mut self, message: Message, now: u64) {
        if let Some(browser) = &self.browser && let Err(e) = browser.session.send(&message) {
            self.browser = None;
            self.app.browser_stopped(format!("{e:#}"));
            self.app.report(&format!("{e:#}"), now);
        }
    }

    pub(super) fn browser_told(&mut self, now: u64) {
        if let Some(worker) = &self.browser_start && let Some(result) = worker.events().pop() {
            self.browser_start = None;
            if self.app.screen() == Screen::Browser {
                let started = result.map_err(anyhow::Error::msg).and_then(|runtime| Browser::start(runtime, &self.browser_dns));
                match started {
                    Ok(browser) => {
                        self.browser = Some(browser);
                        self.app.browser_started();
                        let mut nav = Message::command(1);
                        nav.text = self.browser_address.clone();
                        self.browser_send(nav, now);
                    }
                    Err(e) => {
                        let _ = skyward_sys::kmsg(&format!("browser start failed: {e:#}"));
                        self.app.browser_stopped(format!("{e:#}"));
                    }
                }
            } else { self.app.browser_closed(); }
        }
        if self.app.screen() != Screen::Browser {
            if self.browser.take().is_some() { self.app.browser_closed(); }
            self.browser_touch = None;
            return;
        }
        let result = (|| -> Result<()> {
            let Some(browser) = &mut self.browser else { return Ok(()) };
            ensure!(browser.running()?, "browser process exited; see /tmp/skyward-browser.log");
            for _ in 0..12 {
                let Some(message) = browser.session.receive()? else { break };
                if message.kind == 32 {
                    self.app.browser_frame(browser.session.copy_frame(&message)?);
                } else { self.app.browser_event(message); }
            }
            Ok(())
        })();
        if let Err(e) = result {
            let _ = skyward_sys::kmsg(&format!("browser stopped: {e:#}"));
            self.browser = None;
            self.app.browser_stopped(format!("{e:#}"));
        }
    }

    pub(super) fn browser_touch(&mut self, event: skyward_ui::app::Touch, now: u64) -> bool {
        use skyward_ui::app::Touch;
        if self.browser.is_none() || self.app.screen() != Screen::Browser || !self.app.display_on() { return false; }
        let view = skyward_ui::browser::viewport(&self.app.theme(), self.app.state.browser_keyboard);
        let top = skyward_ui::browser::source_top(view,self.app.state.browser_keyboard,self.app.state.browser_focus_y);
        let point = |x,y| skyward_ui::browser::page_point(view,top,x,y);
        let (kind, at) = match event {
            Touch::Down(x,y) if view.contains(x,y) => { let p = point(x,y); self.browser_touch = Some(p); (6,p) }
            Touch::Move(x,y) if self.browser_touch.is_some() => { let p = point(x,y); self.browser_touch = Some(p); (7,p) }
            Touch::Up if self.browser_touch.is_some() => (8,self.browser_touch.take().unwrap()),
            _ => return false,
        };
        self.app.browser_activity(now);
        let mut message = Message::command(kind);
        (message.x,message.y) = at;
        self.browser_send(message,now);
        if kind == 8 { self.app.browser_focus(at.1); }
        true
    }
}

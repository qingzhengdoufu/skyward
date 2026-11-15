use super::*;
impl Pages<'_> {
    pub(super) fn browser_told(&mut self, now: u64) {
        let mut downloads = Vec::new();
        let mut stream_dead = false;
        if let Some(worker) = &self.browser_stream {
            for ev in worker.events() {
                match ev {
                    crate::browser::Event::Shot(Ok(frame)) => self.app.browser_plugin_shot(Ok(frame)),
                    crate::browser::Event::Shot(Err(why)) => {
                        self.app.browser_plugin_shot(Err(why));
                        stream_dead = true;
                    }
                    crate::browser::Event::Focus(focus) => self.app.browser_plugin_focus(focus),
                    crate::browser::Event::Navigated(url) => self.app.browser_navigated(url),
                    crate::browser::Event::Download { url, name } => downloads.push((url, name)),
                    _ => {}
                }
            }
        }
        if stream_dead {
            self.browser_stream = None;
        }
        let mut download_done = false;
        if let Some(worker) = &self.browser_download {
            for event in worker.events() {
                match event {
                    crate::browser::Event::Saved(saving) => self.app.browser_saved(saving.done, saving.total),
                    crate::browser::Event::Ended(ended) => {
                        self.app.browser_save_ended(ended.map(|_| ()), now);
                        download_done = true;
                    }
                    crate::browser::Event::PluginSaved { done, total, speed } => self.app.plugin_saved(done, total, speed),
                    crate::browser::Event::PluginEnded(ended) => {
                        self.app.plugin_downloaded(ended);
                        download_done = true;
                    }
                    _ => {}
                }
            }
        }
        if download_done {
            self.browser_download = None;
        }
        let mut browser_done = false;
        if let Some(worker) = &self.browser {
            for ev in worker.events() {
                match ev {
                    crate::browser::Event::Opened(opened) => {
                        self.app.browser_opened(opened);
                        browser_done = true;
                    }
                    crate::browser::Event::Navigated(url) => self.app.browser_navigated(url),
                    crate::browser::Event::Download { url, name } => downloads.push((url, name)),
                    crate::browser::Event::Saved(saving) => self.app.browser_saved(saving.done, saving.total),
                    crate::browser::Event::Ended(ended) => {
                        self.app.browser_save_ended(ended.map(|_| ()), now);
                        browser_done = true;
                    }
                    crate::browser::Event::Installed(ended) => {
                        self.app.browser_plugin_installed(ended);
                        browser_done = true;
                    }
                    crate::browser::Event::Shot(shot) => {
                        self.app.browser_plugin_shot(shot);
                        browser_done = true;
                    }
                    crate::browser::Event::Focus(focus) => self.app.browser_plugin_focus(focus),
                    crate::browser::Event::PluginSaved { done, total, speed } => self.app.plugin_saved(done, total, speed),
                    crate::browser::Event::PluginEnded(ended) => {
                        self.app.plugin_downloaded(ended);
                        browser_done = true;
                    }
                }
            }
        }
        if browser_done {
            self.browser = None;
        }
        for (url, name) in downloads {
            for effect in self.app.browser_offered_download(url, name, now) {
                if let Err(error) = self.effect(effect, now) {
                    self.app.browser_save_ended(Err(error.to_string()), now);
                }
            }
        }
    }
}

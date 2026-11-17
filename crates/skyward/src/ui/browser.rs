use super::*;
impl Pages<'_> {
    pub(super) fn browser_told(&mut self, now: u64) {
        if let Some(worker) = &self.browser_stream
            && matches!(self.app.screen(), Screen::Browser | Screen::BrowserAddress)
        {
            let theme = self.app.theme();
            // Keyboard overlays the page; never resize/reflow on focus changes.
            let area = skyward_ui::browser::full_viewport(&theme);
            let size = (area.w.round() as u32, area.h.round() as u32);
            if self.browser_viewport != Some(size) {
                worker.command(crate::browser::Control::Viewport {
                    width: size.0,
                    height: size.1,
                });
                self.browser_viewport = Some(size);
            }
        }
        let mut downloads = Vec::new();
        let mut stream_failure = None;
        if let Some(worker) = &self.browser_stream {
            for ev in worker.events() {
                match ev {
                    #[cfg(feature = "aera-browser")]
                    crate::browser::Event::Pixels { width, height, rgba } => self.app.browser_plugin_pixels(width, height, rgba),
                    crate::browser::Event::Shot(Ok(frame)) => self.app.browser_plugin_shot(Ok(frame)),
                    crate::browser::Event::Shot(Err(why)) => {
                        stream_failure = Some(why);
                        break;
                    }
                    crate::browser::Event::Focus(focus) => self.app.browser_plugin_focus(focus),
                    crate::browser::Event::MediaPlaying(playing) => self.app.browser_plugin_media_playing(playing, now),
                    crate::browser::Event::Navigated(url) => self.app.browser_navigated(url),
                    crate::browser::Event::Download { url, name } => downloads.push((url, name)),
                    _ => {}
                }
            }
        }
        if let Some(why) = stream_failure {
            self.browser_stream = None;
            let mut effects = VecDeque::from(self.app.browser_plugin_failed(why, now));
            while let Some(effect) = effects.pop_front() {
                match self.effect(effect, now) {
                    Ok(next) => effects.extend(next),
                    Err(error) => self.app.report(&format!("{error:#}"), now),
                }
            }
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
            if worker.stopped() && worker.ended() && !download_done {
                if worker.plugin_download() {
                    self.app.plugin_downloaded(Err("stopped as asked".into()));
                } else {
                    self.app.browser_save_ended(Err("stopped as asked".into()), now);
                }
                download_done = true;
            }
        }
        if download_done && self.browser_removal.is_none() {
            self.browser_download = None;
        }
        if self.browser_removal.is_some() && self.browser_download.as_ref().is_none_or(crate::browser::Worker::ended) {
            self.browser_download = None;
            let path = self.browser_removal.take().expect("checked above");
            let result = skyward_net::web::discard_partial(Path::new(&path)).map_err(|error| format!("{error:#}"));
            self.app.browser_download_removed(&path, result, now);
        }
        let mut browser_done = false;
        if let Some(worker) = &self.browser {
            for ev in worker.events() {
                match ev {
                    #[cfg(feature = "aera-browser")]
                    crate::browser::Event::Pixels { width, height, rgba } => self.app.browser_plugin_pixels(width, height, rgba),
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
                    crate::browser::Event::MediaPlaying(_) => {}
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

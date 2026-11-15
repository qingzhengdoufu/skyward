use super::*;

/// How far asking the update server for the newest system has come.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Check {
    #[default]
    Idle,
    Asking,
    Failed(Unanswered),
    /// The newest system the server knows; none when it knows of none for
    /// the installed one.
    Answered(Option<Release>),
}

/// What internal storage holds of the newest system's full package.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Have {
    #[default]
    Nothing,
    /// Part of it, this many bytes, from a download stopped before its end.
    Part(u64),
    /// All of it, checked, this many bytes.
    Whole(u64),
}

/// Downloading the newest system's full package, as the update page shows
/// it: how far it has come, how fast, and how it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadJob {
    /// None until the download has begun.
    pub progress: Option<Progress>,
    /// Bytes a second.
    pub rate: u64,
    pub outcome: Option<DownloadEnd>,
    /// Stopping was asked for and is under way.
    pub stopping: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadEnd {
    Downloaded,
    Stopped,
    Failed(Failure),
}

/// Where the full packages downloaded go, from the top of internal
/// storage: the folder the stock updater keeps them in.
pub const DOWNLOADS: &str = "downloaded_rom";

impl App {
    /// What the update server answered, and what internal storage holds of
    /// the full package of the newest system it told of.
    pub fn update_checked(&mut self, answer: Result<Option<Release>, Unanswered>, have: Have) {
        self.state.check = match answer {
            Ok(newest) => Check::Answered(newest),
            Err(why) => Check::Failed(why),
        };
        self.state.have = have;
        if matches!(self.screen(), Screen::Advanced | Screen::Update) {
            self.refresh();
        }
    }
}

impl App {
    /// How far downloading the full package has come, and how fast it goes,
    /// in bytes a second.
    pub fn download_progress(&mut self, progress: Progress, rate: u64) {
        let job = self.state.download.as_mut().expect("only a download under way moves on");
        job.progress = Some(progress);
        job.rate = rate;
        if matches!(self.screen(), Screen::Update | Screen::ControlCenter) {
            self.refresh();
        }
    }
}

impl App {
    /// Downloading the full package has ended, leaving `have` of it: the
    /// update page says how, lit for the user to read.
    pub fn download_ended(&mut self, end: DownloadEnd, have: Have, now: u64) {
        let job = self.state.download.as_mut().expect("only a download under way ends");
        job.stopping = false;
        job.outcome = Some(end);
        self.state.have = have;
        self.wake(now);
        self.refresh();
    }
}

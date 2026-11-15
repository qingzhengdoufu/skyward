use super::*;

/// How far asking GitHub for Skyward's newest release has come.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SkywardCheck {
    #[default]
    Idle,
    Asking,
    Failed(Unanswered),
    Answered(Newest),
}

/// Updating Skyward, as its page shows it: downloading the newest release's
/// build for the phone, then writing it into the recovery partitions, how
/// far that has come, and how it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkywardJob {
    /// None until the download has begun.
    pub stage: Option<SkywardStage>,
    /// Bytes a second, while it downloads.
    pub rate: u64,
    pub outcome: Option<SkywardEnd>,
    /// Stopping the download was asked for and is under way.
    pub stopping: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkywardStage {
    Download(Progress),
    /// The build is written into the recovery partitions and read back:
    /// bytes so far, of all.
    Write {
        done: u64,
        total: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkywardEnd {
    Ready(Slot),
    /// The build is in the recovery partitions of these slots.
    Updated(Vec<Slot>),
    /// The download was stopped; what it downloaded is kept to go on from.
    Stopped,
    /// The download did not end with the build whole.
    Download(Failure),
    /// The build could not go in, or where it goes could not be told: why.
    /// Nothing was written.
    NotWritten(String),
    /// Writing it into the recovery partitions of these slots failed, after
    /// some of it was written: why.
    WriteFailed(Vec<Slot>, String),
}

impl App {
    /// What GitHub answered of Skyward's newest release.
    pub fn skyward_checked(&mut self, answer: Result<Newest, Unanswered>) {
        self.state.skyward_check = match answer {
            Ok(newest) => SkywardCheck::Answered(newest),
            Err(why) => SkywardCheck::Failed(why),
        };
        if matches!(self.screen(), Screen::Settings | Screen::SkywardUpdate) {
            self.refresh();
        }
    }
}

impl App {
    /// How far updating Skyward has come, and how fast the download goes,
    /// in bytes a second.
    pub fn skyward_progress(&mut self, stage: SkywardStage, rate: u64) {
        let job = self.state.skyward_job.as_mut().expect("only an update under way moves on");
        job.stage = Some(stage);
        job.rate = rate;
        if self.screen() == Screen::SkywardUpdate {
            self.refresh();
        }
    }
}

impl App {
    /// Updating Skyward has ended: its page says how, lit for the user to
    /// read.
    pub fn skyward_ended(&mut self, end: SkywardEnd, now: u64) {
        self.state.confirm = 0.0;
        let job = self.state.skyward_job.as_mut().expect("only an update under way ends");
        job.stopping = false;
        job.outcome = Some(end);
        self.wake(now);
        self.refresh();
    }
}

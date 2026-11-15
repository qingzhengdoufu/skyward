use super::*;

/// Giving the data partition's files their SELinux labels, as its page
/// shows it: how far it has come and how it ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelabelJob {
    /// Entries looked at and how many there are, 0 while they are still
    /// counted, and how many took a new label.
    pub checked: u64,
    pub total: u64,
    pub changed: u64,
    pub outcome: Option<JobEnd>,
    /// Stopping was asked for and is under way.
    pub stopping: bool,
}

impl App {
    /// Giving the data partition's files their labels has started: its page
    /// follows how far it comes.
    pub fn relabel_started(&mut self) {
        self.state.relabel = Some(RelabelJob::default());
        self.refresh();
    }
}

impl App {
    /// How far giving the data partition's files their labels has come:
    /// entries looked at, of how many, and how many took a new label.
    pub fn relabel_progress(&mut self, checked: u64, total: u64, changed: u64) {
        let job = self.state.relabel.as_mut().expect("only work under way moves on");
        (job.checked, job.total, job.changed) = (checked, total, changed);
        if self.screen() == Screen::Relabel {
            self.refresh();
        }
    }
}

impl App {
    /// Giving the data partition's files their labels has ended: its page
    /// says how, lit for the user to read.
    pub fn relabel_ended(&mut self, end: JobEnd, now: u64) {
        let job = self.state.relabel.as_mut().expect("only work under way ends");
        job.stopping = false;
        job.outcome = Some(end);
        self.wake(now);
        self.refresh();
    }
}

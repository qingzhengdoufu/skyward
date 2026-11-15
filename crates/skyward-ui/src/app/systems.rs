use super::*;

/// A system to install beside the installed one, and the room its root
/// takes so far.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemChoice {
    pub name: String,
    pub version: String,
    /// The sizes there is room for, the least first.
    pub sizes: Vec<u64>,
    /// Which of them is chosen.
    pub size: usize,
}

/// The system that starts, as the runtime last read the systems.
pub fn starting(systems: &Overview) -> Who {
    if systems.other_starts { Who::Other } else { Who::Installed }
}

/// Work on the systems the runtime carries out for the systems page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemsWork {
    /// Writing this system's boot set into the boot partitions; the phone
    /// restarts into it once that is done.
    Switching(Who),
    /// Taking the other system away.
    Removing,
}

impl App {
    /// What the boot control holds of the slots, as the runtime read it, or
    /// why it could not.
    pub fn set_boot_slots(&mut self, slots: Result<BootSlots, String>) {
        self.state.boot_slots = Some(slots);
        self.refresh();
    }
}

impl App {
    /// How switching the slot to start next went: what the boot control
    /// holds of the slots now, or why it did not switch.
    pub fn slot_switched(&mut self, outcome: Result<BootSlots, String>, now: u64) {
        match outcome {
            Ok(slots) => {
                self.state.next_slot = None;
                self.state.boot_slots = Some(Ok(slots));
                let slot = slots.active.letter();
                self.say(
                    &tr!(
                        self.state.lang,
                        "下次将从槽位 {slot} 启动",
                        "下次將從槽位 {slot} 啟動",
                        "Slot {slot} starts next"
                    ),
                    now,
                );
                self.refresh();
            }
            Err(why) => self.say(
                &tr!(self.state.lang, "切换失败：{why}", "切換失敗：{why}", "Couldn't switch: {why}"),
                now,
            ),
        }
    }
}

impl App {
    /// The systems, as the runtime read them, or why it could not; work on
    /// them that was under way has ended.
    pub fn set_systems(&mut self, systems: Result<Overview, String>) {
        self.state.systems = Some(systems);
        self.state.systems_work = None;
        self.refresh();
    }
}

impl App {
    /// Why the work on the systems the page asked for did not go: the
    /// switch did not happen, or the other system was not taken away.
    pub fn systems_work_failed(&mut self, why: &str, now: u64) {
        let l = self.state.lang;
        let said = match self.state.systems_work.take() {
            Some(SystemsWork::Removing) => {
                tr!(l, "删除失败：{why}", "刪除失敗：{why}", "Couldn't remove: {why}")
            }
            _ => {
                tr!(l, "切换失败：{why}", "切換失敗：{why}", "Couldn't switch: {why}")
            }
        };
        self.say(&said, now);
        self.refresh();
    }
}

impl App {
    /// The second system's start stopped, for `why`, as its first program
    /// told it: the systems page shows it, over the home page.
    pub fn system_stopped(&mut self, why: String) {
        self.state.system_notice = Some(why);
        self.open(Screen::Systems);
    }
}

impl App {
    /// A switch to `to` a system asked for before the recovery started is
    /// under way: the systems page shows it, over the home page.
    pub fn switch_asked(&mut self, to: Who) {
        self.state.systems_work = Some(SystemsWork::Switching(to));
        self.open(Screen::Systems);
    }
}

impl App {
    /// Where the update stands, as the runtime read it or a merge has come.
    pub fn set_update(&mut self, standing: Standing) {
        if self.state.update != standing {
            self.state.update = standing;
            self.refresh();
        }
    }
}

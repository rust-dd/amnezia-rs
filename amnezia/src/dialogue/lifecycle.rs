use crate::windowskin::motion::Motion;

#[derive(Default)]
pub(crate) struct Lifecycle {
    pub(crate) message: Motion,
    pub(crate) gold: Motion,
    pub(super) last_frame: Option<u32>,
    pub(super) battle: bool,
    pub(super) page_wait: bool,
    pub(super) reveal_ticks: Option<u32>,
    closed_frame: bool,
    closing_started: bool,
}

impl Lifecycle {
    pub(super) fn open(&mut self) {
        self.message.open(!self.battle);
        self.closed_frame = false;
        self.closing_started = false;
        self.page_wait = false;
        self.reveal_ticks = None;
    }

    pub(super) fn close(&mut self, frame: u32) {
        self.message.close(!self.battle);
        self.gold.close(!self.battle);
        self.closed_frame = self.battle;
        self.closing_started = true;
        self.last_frame = Some(frame);
        self.reveal_ticks = Some(0);
    }

    pub(super) fn busy(&self) -> bool {
        self.message.visible() || self.closed_frame
    }

    pub(super) fn allows_next(&self, foreground: bool) -> bool {
        !self.busy() || (foreground && self.closing_started)
    }

    pub(super) fn step(&mut self) {
        self.closing_started = false;
        self.closed_frame = self.message.step();
        self.gold.step();
    }

    pub(super) fn prepare(&mut self, frame: u32, active: bool, battle: bool, paused: bool) {
        if self.last_frame == Some(frame) && self.reveal_ticks.is_some() {
            return;
        }
        self.battle = battle;
        if battle && active {
            self.message.open(false);
        }
        let previous = self.last_frame.replace(frame);
        let ticks = previous.map_or(u32::from(active && self.message.ready()), |previous| {
            frame.wrapping_sub(previous)
        });
        let mut reveal = 0;
        if !paused {
            for _ in 0..ticks {
                self.step();
                reveal += u32::from(active && self.message.ready());
            }
        }
        self.reveal_ticks = Some(reveal);
    }
}

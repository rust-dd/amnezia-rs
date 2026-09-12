#[derive(Debug, Default)]
pub(super) struct PauseArrow {
    active: bool,
    frame: u32,
}

impl PauseArrow {
    pub(super) fn set(&mut self, active: bool) {
        self.active = active;
        self.frame = 0;
    }

    pub(super) fn advance(&mut self, frames: u32) {
        if self.active {
            self.frame = (self.frame + frames % 40) % 40;
        }
    }

    pub(super) fn visible(&self) -> bool {
        self.active && self.frame < 20
    }
}

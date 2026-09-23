mod raster;
#[cfg(test)]
mod tests;

pub(crate) use raster::{Pixels, register};

const FRAMES: u8 = 7;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    #[default]
    Hidden,
    Opening(u8),
    Open,
    Closing(u8),
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Motion(Phase);

impl Motion {
    pub(crate) fn open(&mut self, animate: bool) {
        self.0 = if self.visible() || !animate {
            Phase::Open
        } else {
            Phase::Opening(0)
        };
    }

    pub(crate) fn close(&mut self, animate: bool) {
        self.0 = if self.visible() && animate {
            // RPG_RT includes the first closing step in the decision frame.
            Phase::Closing(1)
        } else {
            Phase::Hidden
        };
    }

    pub(crate) fn step(&mut self) -> bool {
        self.0 = match self.0 {
            Phase::Opening(n) if n + 1 == FRAMES => Phase::Open,
            Phase::Opening(n) => Phase::Opening(n + 1),
            Phase::Closing(n) if n + 1 == FRAMES => {
                self.0 = Phase::Hidden;
                return true;
            }
            Phase::Closing(n) => Phase::Closing(n + 1),
            phase => phase,
        };
        false
    }

    pub(crate) fn visible(self) -> bool {
        self.0 != Phase::Hidden
    }

    pub(crate) fn ready(self) -> bool {
        self.0 == Phase::Open
    }

    pub(crate) fn closing(self) -> bool {
        matches!(self.0, Phase::Closing(_))
    }

    pub(crate) fn half_height(self, height: u32) -> u32 {
        let half = f64::from(height) / 2.0;
        let increment = half / f64::from(FRAMES);
        let (mut count, step, frames) = match self.0 {
            Phase::Hidden => return 0,
            Phase::Open => return height / 2,
            Phase::Opening(frames) => (0.0, increment, frames),
            Phase::Closing(frames) => (half, -increment, frames),
        };
        for _ in 0..frames {
            count += step;
        }
        count as u32
    }
}

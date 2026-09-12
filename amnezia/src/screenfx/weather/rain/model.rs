use crate::interpreter::EventRng;
use bevy::prelude::*;

pub(super) const WIDTH: i32 = 320;
pub(super) const HEIGHT: i32 = 160;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Drop {
    pub x: i32,
    pub y: i32,
    pub life: u8,
}

#[derive(Resource)]
pub(super) struct Rain {
    pub drops: Vec<Drop>,
    pub fraction: f64,
    rng: EventRng,
}

impl Default for Rain {
    fn default() -> Self {
        Self::new(EventRng::default())
    }
}

impl Rain {
    pub(super) fn new(mut rng: EventRng) -> Self {
        let drops = (0..100)
            .map(|_| Drop {
                life: (rng.next_u64() % 40) as u8,
                x: (rng.next_u64() % WIDTH as u64) as i32,
                y: (rng.next_u64() % HEIGHT as u64) as i32,
            })
            .collect();
        Self {
            drops,
            fraction: 0.0,
            rng,
        }
    }

    pub(super) fn advance(&mut self, seconds: f64) {
        self.fraction += seconds * 60.0;
        let ticks = (self.fraction + 1e-6).floor() as u32;
        self.fraction = (self.fraction - f64::from(ticks)).max(0.0);
        for _ in 0..ticks {
            self.tick();
        }
    }

    fn tick(&mut self) {
        for drop in &mut self.drops {
            if drop.life != 0 {
                drop.life -= 1;
                drop.x -= 1;
                drop.y += 4;
            } else if self.rng.next_u64() % 100 < 10 {
                drop.life = 12;
                drop.x = (self.rng.next_u64() % WIDTH as u64) as i32;
                drop.y = (self.rng.next_u64() % HEIGHT as u64) as i32;
            }
        }
    }
}

#[cfg(test)]
mod tests;

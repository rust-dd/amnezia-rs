use crate::timing::GameFrames;
use bevy::prelude::*;

pub(super) const KEYS: [KeyCode; 4] = [
    KeyCode::ArrowDown,
    KeyCode::ArrowUp,
    KeyCode::ArrowRight,
    KeyCode::ArrowLeft,
];

#[derive(Resource, Default)]
pub(in crate::menu) struct Input {
    last_frame: Option<u32>,
    elapsed: u32,
    held: [u32; 4],
    start: [u32; 4],
    pressed: [bool; 4],
    triggered: [bool; 4],
    pub rewound: bool,
}

impl Input {
    pub(super) fn advance(&mut self, now: u32, keys: &ButtonInput<KeyCode>) {
        self.elapsed = self
            .last_frame
            .replace(now)
            .map_or(0, |last| now.wrapping_sub(last));
        self.rewound = self.elapsed > i32::MAX as u32;
        if self.rewound {
            self.held = [0; 4];
            self.elapsed = 0;
        }
        self.pressed = KEYS.map(|key| keys.pressed(key));
        self.triggered = KEYS.map(|key| keys.just_pressed(key));
        self.start = std::array::from_fn(|index| {
            if !self.pressed[index] || self.triggered[index] {
                0
            } else {
                self.held[index]
            }
        });
        self.held = std::array::from_fn(|index| {
            if self.pressed[index] {
                self.start[index].saturating_add(self.elapsed)
            } else {
                0
            }
        });
    }

    pub(in crate::menu) fn timed(&self) -> bool {
        self.elapsed > 0
    }

    pub(in crate::menu) fn steps(&self) -> impl Iterator<Item = [bool; 4]> + '_ {
        (0..self.elapsed.max(1)).map(|step| {
            std::array::from_fn(|index| {
                let held = self.start[index].saturating_add(step + 1);
                (step == 0 && self.triggered[index])
                    || (self.timed() && self.pressed[index] && held >= 24 && held.is_multiple_of(4))
            })
        })
    }
}

pub(in crate::menu) fn update(
    frames: Res<GameFrames>,
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<Input>,
) {
    input.advance(frames.frame, &keys);
}

#[cfg(test)]
mod tests;

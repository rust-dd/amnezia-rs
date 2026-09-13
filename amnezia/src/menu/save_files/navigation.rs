use crate::save::slots::COUNT;
use bevy::prelude::*;

const KEYS: [KeyCode; 4] = [
    KeyCode::ArrowDown,
    KeyCode::ArrowUp,
    KeyCode::PageDown,
    KeyCode::PageUp,
];

#[derive(Default)]
pub(super) struct Navigation {
    pub index: usize,
    pub top: usize,
    pub arrow: u32,
    pub cursors: [u32; COUNT as usize],
    old_top: usize,
    movement: Option<u32>,
    held: [u32; 4],
}

impl Navigation {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            top: index.saturating_sub(2),
            ..default()
        }
    }

    pub fn offset(&self) -> i32 {
        self.movement.map_or(0, |frame| {
            let distance = (self.top as i32 - self.old_top as i32) * 64;
            distance - distance * frame.min(7) as i32 / 7
        })
    }

    pub fn tick(&mut self, keys: &ButtonInput<KeyCode>, triggered: bool, timed: bool) -> bool {
        self.arrow = (self.arrow + u32::from(timed)) % 40;
        let moving = self.movement.is_some();
        if timed && let Some(frame) = &mut self.movement {
            *frame += 1;
            if *frame > 7 {
                self.movement = None;
            }
        }
        let mut repeated = [false; 4];
        for (index, key) in KEYS.iter().enumerate() {
            self.held[index] = if keys.pressed(*key) {
                self.held[index].saturating_add(u32::from(timed))
            } else {
                0
            };
            repeated[index] = (triggered && keys.just_pressed(*key))
                || (timed && self.held[index] >= 24 && self.held[index].is_multiple_of(4));
        }
        if moving {
            self.cursors[self.index] = (self.cursors[self.index] + u32::from(timed)) % 21;
            return false;
        }
        let old_index = self.index;
        for (action, repeated) in repeated.into_iter().enumerate() {
            if repeated {
                self.navigate(action, triggered && keys.just_pressed(KEYS[action]));
            }
        }
        self.old_top = self.top;
        self.top = self.top.max(self.index.saturating_sub(2)).min(self.index);
        if self.old_top != self.top {
            self.movement = Some(u32::from(timed));
        }
        self.cursors[self.index] = (self.cursors[self.index] + u32::from(timed)) % 21;
        old_index != self.index
    }

    pub fn moving(&self) -> bool {
        self.movement.is_some()
    }

    fn navigate(&mut self, action: usize, triggered: bool) {
        let last = COUNT as usize - 1;
        self.index = match action {
            0 if triggered || self.index < last => (self.index + 1) % COUNT as usize,
            1 if triggered || self.index > 0 => (self.index + last) % COUNT as usize,
            2 => (self.index + 3).min(last),
            3 => self.index.saturating_sub(3),
            _ => self.index,
        };
    }
}

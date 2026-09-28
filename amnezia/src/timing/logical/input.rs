use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct Buffered {
    held: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    released: HashSet<KeyCode>,
}

impl Buffered {
    pub(super) fn capture(&mut self, keys: &ButtonInput<KeyCode>) {
        self.held = keys.get_pressed().copied().collect();
        self.pressed.extend(keys.get_just_pressed().copied());
        self.released.extend(keys.get_just_released().copied());
    }

    pub(super) fn take(&mut self) -> ButtonInput<KeyCode> {
        let keys = self.current();
        self.pressed.clear();
        self.released.clear();
        keys
    }

    pub(super) fn current(&self) -> ButtonInput<KeyCode> {
        let mut keys = ButtonInput::default();
        for key in self.pressed.iter().chain(&self.released) {
            keys.press(*key);
        }
        for key in &self.released {
            keys.release(*key);
        }
        for key in &self.held {
            keys.press(*key);
        }
        for key in keys.get_pressed().copied().collect::<Vec<_>>() {
            if !self.held.contains(&key) {
                keys.release(key);
            }
        }
        for key in keys.get_just_pressed().copied().collect::<Vec<_>>() {
            if !self.pressed.contains(&key) {
                keys.clear_just_pressed(key);
            }
        }
        for key in keys.get_just_released().copied().collect::<Vec<_>>() {
            if !self.released.contains(&key) {
                keys.clear_just_released(key);
            }
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_every_combination_without_inventing_edges() {
        for held in [false, true] {
            for pressed in [false, true] {
                for released in [false, true] {
                    let mut buffered = Buffered::default();
                    let key = KeyCode::Enter;
                    if held {
                        buffered.held.insert(key);
                    }
                    if pressed {
                        buffered.pressed.insert(key);
                    }
                    if released {
                        buffered.released.insert(key);
                    }
                    let keys = buffered.take();
                    assert_eq!(keys.pressed(key), held);
                    assert_eq!(keys.just_pressed(key), pressed);
                    assert_eq!(keys.just_released(key), released);
                    let next = buffered.take();
                    assert_eq!(next.pressed(key), held);
                    assert!(!next.just_pressed(key));
                    assert!(!next.just_released(key));
                }
            }
        }
    }

    #[test]
    fn keeps_a_complete_tap_between_logical_updates() {
        let mut buffered = Buffered::default();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::Enter);
        buffered.capture(&keys);
        keys.clear();
        keys.release(KeyCode::Enter);
        buffered.capture(&keys);
        keys.clear();
        buffered.capture(&keys);
        let delivered = buffered.take();
        assert!(delivered.just_pressed(KeyCode::Enter));
        assert!(delivered.just_released(KeyCode::Enter));
        assert!(!delivered.pressed(KeyCode::Enter));
        assert!(!buffered.take().just_pressed(KeyCode::Enter));
    }
}

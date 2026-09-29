use super::*;

#[derive(Clone, Copy, Default)]
pub(in crate::battle) struct Controls {
    pub fast: bool,
    pub hold: bool,
}

impl Controls {
    pub(in crate::battle) fn from_keys(keys: &ButtonInput<KeyCode>) -> Self {
        Self {
            fast: keys.any_pressed([
                KeyCode::Enter,
                KeyCode::Space,
                KeyCode::KeyZ,
                KeyCode::ShiftLeft,
                KeyCode::ShiftRight,
            ]),
            hold: keys.any_pressed([KeyCode::Escape, KeyCode::KeyX]),
        }
    }
}

#[derive(Default)]
pub(in crate::battle) struct Wait {
    remaining: u32,
    threshold: u32,
}

impl Wait {
    pub(in crate::battle) fn set(&mut self, min: u32, max: u32) {
        assert!(min <= max);
        self.remaining = max;
        self.threshold = max - min;
    }

    pub(in crate::battle) fn ready(&mut self, controls: Controls) -> bool {
        if self.remaining == 0 {
            return true;
        }
        if controls.hold {
            return false;
        }
        self.remaining -= 1;
        if self.remaining > self.threshold || (!controls.fast && self.remaining > 0) {
            return false;
        }
        self.remaining = 0;
        true
    }
}

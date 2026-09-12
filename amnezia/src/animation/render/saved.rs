use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FlashState {
    elapsed: u32,
    power: u32,
    rgb: [u8; 3],
}

impl FlashQuad {
    pub(in crate::animation) fn snapshot(&self) -> FlashState {
        FlashState {
            elapsed: self.elapsed,
            power: self.power,
            rgb: self.rgb,
        }
    }
}

impl FlashState {
    pub(in crate::animation) fn valid(&self) -> bool {
        self.elapsed <= FLASH_LAST_FRAME
            && self.power <= 31
            && self
                .rgb
                .into_iter()
                .all(|channel| channel <= 248 && channel.is_multiple_of(8))
    }

    pub(in crate::animation) fn restore(self, commands: &mut Commands, frame: u32) {
        spawn_screen_flash(
            commands,
            self.rgb,
            self.power,
            FlashStamp {
                age: self.elapsed,
                frame,
            },
        );
    }
}

use super::Character;

#[derive(Default)]
pub(crate) struct CharacterAnimation {
    pub(crate) mode: u32,
    pub(crate) paused: bool,
    count: u32,
    fraction: f64,
}

impl CharacterAnimation {
    pub(crate) fn keeps_facing(&self) -> bool {
        matches!(self.mode, 2..=5)
    }

    fn frames(&mut self, dt: f32) -> u32 {
        self.fraction += f64::from(dt.max(0.0)) * 60.0;
        let frames = (self.fraction + 1e-6).floor() as u32;
        self.fraction = (self.fraction - f64::from(frames)).max(0.0);
        frames
    }

    pub(crate) fn advance<C: Character>(
        &mut self,
        character: &mut C,
        speed: u32,
        moving: bool,
        jumping: bool,
        dt: f32,
    ) {
        for _ in 0..self.frames(dt) {
            self.tick(character, speed, moving, jumping);
        }
    }

    fn tick<C: Character>(&mut self, character: &mut C, speed: u32, moving: bool, jumping: bool) {
        let speed = speed.clamp(1, 6) as usize - 1;
        if self.mode == 5 {
            self.count += 1;
            if self.count >= [24, 16, 12, 8, 6, 4][speed] {
                character.set_dir((character.dir() + 1) % 4);
                self.count = 0;
            }
            return;
        }
        if self.paused || jumping {
            self.count = 0;
            if self.mode != 4 && character.frame() != 1 {
                character.set_frame(1);
            }
            return;
        }
        if matches!(self.mode, 4 | 6) {
            return;
        }
        let stationary = [12, 10, 8, 6, 5, 4][speed];
        let continuous = [16, 12, 10, 8, 7, 6][speed];
        if matches!(self.mode, 1 | 3)
            || moving
            || matches!(character.frame(), 0 | 2)
            || self.count < stationary - 1
        {
            self.count += 1;
        }
        if self.count >= continuous || (moving && self.count >= stationary) {
            character.set_frame((character.frame() + 1) % 4);
            self.count = 0;
        }
    }

    pub(crate) fn advance_vehicle<C: Character>(
        &mut self,
        character: &mut C,
        animated: bool,
        moving: bool,
        dt: f32,
    ) {
        for _ in 0..self.frames(dt) {
            if !animated {
                self.count = 0;
                if character.frame() != 1 {
                    character.set_frame(1);
                }
            } else {
                self.count += 1;
                if self.count >= if moving { 12 } else { 16 } {
                    character.set_frame((character.frame() + 1) % 4);
                    self.count = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

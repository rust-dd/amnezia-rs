use super::{Character, RouteStepper, toward_dir};

impl RouteStepper {
    pub(crate) fn direction<C: Character>(&self, ch: &C) -> u32 {
        self.direction.unwrap_or_else(|| ch.dir())
    }

    pub(crate) fn set_direction<C: Character>(&mut self, ch: &mut C, dir: u32) {
        self.direction = Some(dir);
        self.update_facing(ch);
    }

    pub(crate) fn update_facing<C: Character>(&self, ch: &mut C) {
        if self.facing_lock.is_some() || self.animation.keeps_facing() {
            return;
        }
        let direction = self.direction(ch);
        if (4..8).contains(&direction) {
            let compatible = [(0, 1), (2, 1), (2, 3), (0, 3)][(direction - 4) as usize];
            if ch.dir() != compatible.0 && ch.dir() != compatible.1 {
                ch.set_dir((ch.dir() + 2) % 4);
            }
        } else {
            ch.set_dir(direction);
        }
    }

    pub(crate) fn normalize_direction<C: Character>(&mut self, ch: &C) -> u32 {
        if self.direction(ch) >= 4 {
            self.direction = Some(ch.dir());
        }
        self.direction(ch)
    }

    pub(crate) fn face_toward<C: Character>(&mut self, ch: &mut C, hero: (i32, i32)) {
        self.direction = Some(self.direction(ch));
        if self.facing_lock.is_none() && !self.animation.keeps_facing() {
            ch.set_dir(toward_dir(hero, ch.tile()));
        }
    }
}

#[cfg(test)]
mod tests;

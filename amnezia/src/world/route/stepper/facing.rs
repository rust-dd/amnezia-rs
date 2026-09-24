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
        if self.facing_lock.is_none() && !self.animation.keeps_facing() {
            ch.set_dir(self.direction(ch));
        }
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

use super::*;

impl RouteStepper {
    pub(super) fn begin_jump<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        can_step: &impl Fn(&C, i32, i32, bool, bool) -> bool,
    ) -> Step {
        let end = self.commands[self.index + 1..]
            .iter()
            .position(|cmd| cmd.code == 25)
            .map(|offset| self.index + 1 + offset);
        let (mut dx, mut dy) = (0, 0);
        let previous = self.direction(ch);
        let previous_facing = ch.dir();
        let mut direction = previous;
        for index in self.index + 1..end.unwrap_or(self.commands.len()) {
            let code = self.commands[index].code;
            match code {
                0..=7 => direction = code,
                8 | 20 => direction = self.random_dir(),
                9 | 21 => direction = toward_dir(hero, ch.tile()),
                10 | 22 => direction = away_dir(hero, ch.tile()),
                12..=15 => direction = code - 12,
                16 => direction = (direction + 1) % 4,
                17 => direction = (direction + 3) % 4,
                18 => direction = (direction + 2) % 4,
                19 => direction = (direction + if self.random_bit() { 1 } else { 3 }) % 4,
                _ => {}
            }
            if code <= 11 {
                let delta = dir_delta(direction);
                dx += delta.0;
                dy += delta.1;
            }
        }
        self.direction = Some(direction);
        let Some(end) = end else {
            self.index = self.commands.len() - 1;
            return Step::Next;
        };
        let direction = if dx.abs() > dy.abs() {
            if dx < 0 { DIR_LEFT } else { DIR_RIGHT }
        } else if dy < 0 {
            DIR_UP
        } else {
            DIR_DOWN
        };
        self.direction = Some(direction);
        if (dx != 0 || dy != 0)
            && self.facing_lock.is_none()
            && !matches!(self.animation.mode, 2..=4)
        {
            ch.set_dir(direction);
        }
        if (dx != 0 || dy != 0) && !can_step(ch, dx, dy, true, self.through) {
            self.timer = step_delay_secs(self.frequency);
            if self.skippable {
                self.direction = Some(previous);
                ch.set_dir(previous_facing);
                self.index = end;
                return Step::Next;
            }
            return Step::Retry;
        }
        self.index = end;
        self.timer = step_delay_secs(self.frequency);
        let per_frame = [8_u32, 12, 16, 24, 32, 64][(self.speed - 1) as usize];
        let seconds = 256_u32.div_ceil(per_frame) as f32 / FPS;
        Step::Gate(Some((
            RouteAction::Jump {
                dx,
                dy,
                face: ch.dir(),
            },
            seconds,
        )))
    }
}

#[cfg(test)]
mod tests;

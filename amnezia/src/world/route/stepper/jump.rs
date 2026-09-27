use super::*;

impl RouteStepper {
    pub(super) fn begin_jump<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        program: &MoveRouteDef,
        turn: &Turn,
    ) -> Step {
        let start = turn.index(self);
        let end = program.commands[start + 1..]
            .iter()
            .position(|cmd| cmd.code == 25)
            .map(|offset| start + 1 + offset);
        let (mut dx, mut dy) = (0, 0);
        let previous = self.direction(ch);
        let previous_facing = ch.dir();
        let mut direction = previous;
        for index in start + 1..end.unwrap_or(program.commands.len()) {
            let code = program.commands[index].code;
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
            turn.set_index(self, program.commands.len() - 1);
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
        turn.set_index(self, end);
        Step::Attempt(Attempt {
            origin: ch.tile(),
            delta: (dx, dy),
            jumping: true,
            previous,
            facing: previous_facing,
            forward: false,
            start,
        })
    }
}

#[cfg(test)]
mod tests;

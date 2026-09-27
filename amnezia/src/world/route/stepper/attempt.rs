use super::*;

#[derive(Clone, Copy)]
pub(crate) struct Attempt {
    pub(crate) origin: (i32, i32),
    pub(crate) delta: (i32, i32),
    pub(crate) jumping: bool,
    pub(super) previous: u32,
    pub(super) facing: u32,
    pub(super) forward: bool,
    pub(super) start: usize,
}

impl RouteStepper {
    pub(super) fn prepare_move<C: Character>(
        &mut self,
        ch: &mut C,
        direction: Option<u32>,
        delta: (i32, i32),
    ) -> Step {
        let previous = self.direction(ch);
        let facing = ch.dir();
        self.set_direction(ch, direction.unwrap_or(previous));
        Step::Attempt(Attempt {
            origin: ch.tile(),
            delta,
            jumping: false,
            previous,
            facing,
            forward: direction.is_none(),
            start: 0,
        })
    }

    pub(super) fn resolve_attempt<C: Character>(
        &mut self,
        ch: &mut C,
        attempt: Attempt,
        success: bool,
        turn: &Turn,
    ) -> Step {
        if !success {
            if !turn.skippable(self) {
                if attempt.jumping {
                    turn.set_index(self, attempt.start);
                }
                return Step::Retry;
            }
            self.direction = Some(attempt.previous);
            ch.set_dir(attempt.facing);
            if !attempt.jumping {
                self.set_stop_maximum(stop_clock::step(self.frequency));
            }
            return Step::Next;
        }
        if attempt.forward {
            ch.set_dir(attempt.facing);
        }
        self.set_stop_maximum(stop_clock::step(self.frequency));
        let (dx, dy) = attempt.delta;
        let face = ch.dir();
        let (action, seconds) = if attempt.jumping {
            let per_frame = [8_u32, 12, 16, 24, 32, 64][(self.speed - 1) as usize];
            (
                RouteAction::Jump { dx, dy, face },
                256_u32.div_ceil(per_frame) as f32 / FPS,
            )
        } else {
            (
                RouteAction::Step { dx, dy, face },
                step_secs_for_speed(self.speed),
            )
        };
        Step::Gate(Some((action, seconds)))
    }
}

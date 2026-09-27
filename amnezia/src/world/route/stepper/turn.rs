use super::*;

pub(crate) struct Turn {
    program: MoveRouteDef,
    len: usize,
    start: usize,
    forced: bool,
    refresh: bool,
    stopped: bool,
}

pub(in crate::world) enum Progress {
    Done,
    Refresh,
    Move(RouteAction, f32),
}

pub(in crate::world) enum Boundary {
    Ready(Progress),
    Attempt(Attempt),
}

impl Turn {
    pub(in crate::world) fn new(route: &mut RouteStepper) -> Self {
        route.moving = false;
        Self {
            // A page replacement keeps the current call's original command parameters.
            program: MoveRouteDef {
                commands: route.commands.clone(),
                repeat: route.repeat,
                skippable: route.skippable,
            },
            len: route.commands.len(),
            start: route.index,
            forced: route.forced,
            refresh: false,
            stopped: false,
        }
    }

    pub(super) fn index(&self, route: &RouteStepper) -> usize {
        if !self.forced && route.forced {
            route
                .suspended
                .as_ref()
                .map_or(route.index, |page| page.index)
        } else {
            route.index
        }
    }

    pub(super) fn set_index(&self, route: &mut RouteStepper, index: usize) {
        if self.forced && !route.forced {
            return;
        }
        if !self.forced
            && route.forced
            && let Some(page) = &mut route.suspended
        {
            page.index = index;
        } else {
            route.index = index;
        }
    }

    pub(super) fn skippable(&self, route: &RouteStepper) -> bool {
        if self.forced {
            route.skippable
        } else {
            self.program.skippable
        }
    }
}

impl RouteStepper {
    pub(in crate::world) fn advance_turn<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        can_step: &impl Fn(&C, i32, i32, bool, bool) -> bool,
        effects: &mut Vec<StepEffect>,
        turn: &mut Turn,
    ) -> Progress {
        loop {
            match self.prepare_turn(ch, hero, effects, turn) {
                Boundary::Ready(progress) => return progress,
                Boundary::Attempt(attempt) => {
                    let (dx, dy) = attempt.delta;
                    let success = (attempt.jumping && (dx, dy) == (0, 0))
                        || can_step(ch, dx, dy, attempt.jumping, self.through);
                    if let Some(progress) = self.resolve_turn(ch, attempt, success, turn) {
                        return progress;
                    }
                }
            }
        }
    }

    pub(in crate::world) fn prepare_turn<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        effects: &mut Vec<StepEffect>,
        turn: &mut Turn,
    ) -> Boundary {
        if (std::mem::take(&mut turn.refresh) && turn.index(self) == 0)
            || (turn.forced && !self.forced)
        {
            return Boundary::Ready(Progress::Done);
        }
        if turn.forced {
            turn.program.commands.clone_from(&self.commands);
            turn.program.repeat = self.repeat;
            turn.program.skippable = self.skippable;
        }
        loop {
            if turn.stopped || self.stop_active() {
                return Boundary::Ready(Progress::Done);
            }
            let mut index = turn.index(self);
            if index >= turn.len {
                if turn.forced || !self.forced {
                    self.first_pass_complete = true;
                }
                if !turn.program.repeat || turn.len == 0 {
                    if turn.forced {
                        self.cancel_forced();
                    } else if !self.forced && !self.repeat {
                        self.active = false;
                    }
                    return Boundary::Ready(Progress::Done);
                }
                turn.set_index(self, 0);
                index = 0;
                if index == turn.start {
                    return Boundary::Ready(Progress::Done);
                }
            }
            // A callback can replace a forced program with a shorter one.
            let Some(command) = turn.program.commands.get(index) else {
                return Boundary::Ready(Progress::Done);
            };
            let result = self.step_one(ch, hero, effects, command, &turn.program, turn);
            if let Step::Attempt(attempt) = result {
                return Boundary::Attempt(attempt);
            }
            let refresh = matches!(command.code, 32 | 33);
            if let Some(progress) = self.complete_step(result, turn) {
                return Boundary::Ready(progress);
            }
            if refresh {
                turn.refresh = true;
                return Boundary::Ready(Progress::Refresh);
            }
        }
    }

    pub(in crate::world) fn resolve_turn<C: Character>(
        &mut self,
        ch: &mut C,
        attempt: Attempt,
        success: bool,
        turn: &mut Turn,
    ) -> Option<Progress> {
        if turn.forced && !self.forced && !success {
            return Some(Progress::Done);
        }
        let result = self.resolve_attempt(ch, attempt, success, turn);
        self.complete_step(result, turn)
    }

    fn complete_step(&mut self, step: Step, turn: &mut Turn) -> Option<Progress> {
        match step {
            Step::Gate(action) => {
                turn.set_index(self, turn.index(self) + 1);
                self.moving = action.is_some() && self.forced == turn.forced;
                if let Some((action, seconds)) = action {
                    return Some(Progress::Move(action, seconds));
                }
            }
            Step::Retry => return Some(Progress::Done),
            Step::Next => turn.set_index(self, turn.index(self) + 1),
            Step::Attempt(_) => unreachable!(),
        }
        turn.stopped = turn.index(self) == turn.start;
        None
    }
}

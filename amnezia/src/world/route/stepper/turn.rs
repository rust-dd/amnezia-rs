use super::*;

pub(crate) struct Turn {
    program: MoveRouteDef,
    start: usize,
    refresh: bool,
    stopped: bool,
}

pub(in crate::world) enum Progress {
    Done,
    Refresh,
    Move(RouteAction, f32),
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
            start: route.index,
            refresh: false,
            stopped: false,
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
        if std::mem::take(&mut turn.refresh) && self.index == 0 {
            return Progress::Done;
        }
        loop {
            if turn.stopped || self.stop_active() {
                return Progress::Done;
            }
            if self.index >= turn.program.commands.len() {
                self.first_pass_complete = true;
                if !turn.program.repeat || turn.program.commands.is_empty() {
                    if self.forced {
                        self.cancel_forced();
                    } else if !self.repeat {
                        self.active = false;
                    }
                    return Progress::Done;
                }
                self.index = 0;
                if self.index == turn.start {
                    return Progress::Done;
                }
            }
            let command = &turn.program.commands[self.index];
            let result = self.step_one(ch, hero, can_step, effects, command, &turn.program);
            match result {
                Step::Gate(action) => {
                    self.index += 1;
                    self.moving = action.is_some();
                    if let Some((action, seconds)) = action {
                        return Progress::Move(action, seconds);
                    }
                }
                Step::Retry => return Progress::Done,
                Step::Next => self.index += 1,
            }
            turn.stopped = self.index == turn.start;
            if matches!(command.code, 32 | 33) {
                turn.refresh = true;
                return Progress::Refresh;
            }
        }
    }
}

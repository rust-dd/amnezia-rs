use super::{RouteStepper, StopClock, stop_clock};

impl RouteStepper {
    pub(super) fn legacy_stop_clock(&self, autonomy: Option<f32>) -> StopClock {
        let previous = self
            .index
            .checked_sub(1)
            .and_then(|index| self.commands.get(index));
        let maximum = match previous.map(|command| command.code) {
            Some(12..=22) if autonomy.is_none() => stop_clock::turn(self.frequency),
            Some(23) if autonomy.is_none() => stop_clock::wait(self.frequency),
            _ => stop_clock::step(self.frequency),
        };
        let seconds = if self.legacy_autonomy_reset && autonomy.is_some() {
            maximum as f32 / 60.0
        } else {
            autonomy.unwrap_or(self.legacy_timer)
        };
        StopClock::from_legacy(seconds, maximum)
    }

    pub(crate) fn restore_stop_clock(&mut self, autonomy: Option<f32>) {
        if self.stop.is_none() {
            self.stop = Some(self.legacy_stop_clock(autonomy));
        }
        self.legacy_timer = 0.0;
        self.legacy_autonomy_reset = false;
    }
}

impl RouteStepper {
    pub(crate) fn valid(&self) -> bool {
        self.valid_with_facings(4)
    }

    pub(crate) fn valid_for_event(&self) -> bool {
        self.valid_with_facings(8)
    }

    fn valid_with_facings(&self, facings: u32) -> bool {
        self.index <= self.commands.len().saturating_add(1)
            && (1..=6).contains(&self.speed)
            && (1..=8).contains(&self.frequency)
            && self.transparency <= 7
            && stop_clock::legacy_valid(self.legacy_timer)
            && self.stop.is_none_or(StopClock::valid)
            && self.facing_lock.is_none_or(|dir| dir < facings)
            && self.direction.is_none_or(|dir| dir < 8)
            && self.animation.valid()
            && self.suspended.as_ref().is_none_or(|route| route.valid())
    }
}

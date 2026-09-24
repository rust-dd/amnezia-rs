use super::RouteStepper;

impl RouteStepper {
    pub(crate) fn valid(&self) -> bool {
        self.index <= self.commands.len().saturating_add(1)
            && (1..=6).contains(&self.speed)
            && (1..=8).contains(&self.frequency)
            && self.transparency <= 7
            && self.timer.is_finite()
            && self.facing_lock.is_none_or(|dir| dir < 4)
            && self.direction.is_none_or(|dir| dir < 8)
            && self.animation.valid()
            && self.suspended.as_ref().is_none_or(|route| route.valid())
    }
}

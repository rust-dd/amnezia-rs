use super::{RouteStepper, StopClock};

impl RouteStepper {
    pub(crate) fn stop_active(&self) -> bool {
        self.stop_clock().active()
    }

    pub(crate) fn stop_count(&self) -> u32 {
        self.stop_clock().count
    }

    pub(crate) fn stop_maximum(&self) -> u32 {
        self.stop_clock().maximum
    }

    pub(crate) fn set_stop_count(&mut self, count: u32) {
        self.restore_stop_clock(None);
        self.stop.as_mut().unwrap().count = count;
    }

    pub(crate) fn set_stop_maximum(&mut self, maximum: u32) {
        self.restore_stop_clock(None);
        self.stop.as_mut().unwrap().maximum = maximum;
    }

    pub(crate) fn advance_stop_clock(&mut self, moving: bool, allowed: bool) {
        self.restore_stop_clock(None);
        if self.page_present {
            self.stop
                .as_mut()
                .unwrap()
                .advance(moving, self.forced, allowed);
        }
    }

    fn stop_clock(&self) -> StopClock {
        self.stop.unwrap_or_else(|| self.legacy_stop_clock(None))
    }
}

#[cfg(test)]
mod tests;

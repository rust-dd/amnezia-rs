use std::time::{Duration, Instant};

pub(super) struct Clock {
    windowed: bool,
    last: Instant,
    credit: f64,
    pub ticks: u32,
}

impl Clock {
    pub fn new(windowed: bool) -> Self {
        Self {
            windowed,
            last: Instant::now(),
            credit: 0.0,
            ticks: 0,
        }
    }

    pub fn poll(&mut self, remaining: u32, started: bool) -> Duration {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        self.ticks = self.advance(remaining, started, elapsed);
        Duration::from_secs_f64(f64::from(self.ticks) / 60.0)
    }

    fn advance(&mut self, remaining: u32, started: bool, elapsed: Duration) -> u32 {
        if remaining == 0 {
            self.credit = 0.0;
            return 0;
        }
        if !self.windowed {
            return 1;
        }
        if started {
            self.credit = 1.0;
        } else {
            self.credit += elapsed.as_secs_f64() * 60.0;
        }
        // Bevy's virtual clock clamps each render delta to 250 ms.
        let ticks = (self.credit + 1e-6)
            .floor()
            .min(f64::from(remaining))
            .min(15.0) as u32;
        self.credit = (self.credit - f64::from(ticks)).max(0.0);
        if ticks == remaining {
            self.credit = 0.0;
        }
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_commands_keep_real_time_at_different_render_rates() {
        for fps in [12, 15, 30, 60, 120, 144] {
            let mut clock = Clock::new(true);
            let mut total = 0;
            for _ in 0..fps * 5 {
                total += clock.advance(1000, false, Duration::from_secs_f64(1.0 / f64::from(fps)));
            }
            assert_eq!(total, 300, "{fps} FPS");
        }
    }

    #[test]
    fn idle_time_is_not_replayed_and_commands_do_not_overshoot() {
        let mut clock = Clock::new(true);
        assert_eq!(clock.advance(0, false, Duration::from_secs(90)), 0);
        assert_eq!(clock.advance(7, true, Duration::from_secs(90)), 1);
        assert_eq!(clock.advance(6, false, Duration::from_secs(2)), 6);
        assert_eq!(clock.advance(1, true, Duration::from_secs(90)), 1);
        assert_eq!(clock.advance(0, false, Duration::from_secs(90)), 0);
    }

    #[test]
    fn headless_commands_retain_one_tick_per_callback() {
        let mut clock = Clock::new(false);
        assert_eq!(clock.advance(300, true, Duration::from_secs(5)), 1);
        assert_eq!(clock.advance(299, false, Duration::ZERO), 1);
    }

    #[test]
    fn render_stalls_keep_credit_within_bevys_virtual_time_limit() {
        let mut clock = Clock::new(true);
        assert_eq!(clock.advance(1000, false, Duration::from_secs(1)), 15);
        for _ in 0..3 {
            assert_eq!(clock.advance(1000, false, Duration::ZERO), 15);
        }
        assert_eq!(clock.advance(1000, false, Duration::ZERO), 0);
    }
}

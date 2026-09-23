use bevy::time::TimeUpdateStrategy;
use std::time::{Duration, Instant};

pub(super) struct Clock {
    last: Instant,
    rest: Option<Instant>,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            last: Instant::now(),
            rest: None,
        }
    }
}

impl Clock {
    pub(super) fn step(&mut self, now: Instant, resting: bool) -> TimeUpdateStrategy {
        let elapsed = now.saturating_duration_since(self.last);
        self.last = now;
        let delta = if resting {
            let start = *self.rest.get_or_insert(now);
            assert!(
                now.duration_since(start) < Duration::from_secs(20),
                "inn did not finish automatically"
            );
            // Audio sinks use wall time, even when smoke input uses a synthetic clock.
            elapsed
        } else {
            self.rest = None;
            Duration::from_secs_f64(1.0 / 60.0)
        };
        TimeUpdateStrategy::ManualDuration(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overnight_time_follows_audio_at_every_render_rate_without_resetting_real_time() {
        for fps in [15, 30, 60, 120, 144, 240] {
            let start = Instant::now();
            let mut clock = Clock {
                last: start,
                rest: None,
            };
            let mut total = Duration::ZERO;
            for frame in 1..=fps * 8 {
                let now = start + Duration::from_secs_f64(frame as f64 / fps as f64);
                let TimeUpdateStrategy::ManualDuration(delta) = clock.step(now, true) else {
                    unreachable!()
                };
                total += delta;
            }
            assert_eq!(total, Duration::from_secs(8));
            let TimeUpdateStrategy::ManualDuration(delta) = clock.step(start + total, false) else {
                unreachable!()
            };
            assert_eq!(delta, Duration::from_secs_f64(1.0 / 60.0));
            assert!(clock.rest.is_none());
        }
    }

    #[test]
    #[should_panic(expected = "inn did not finish automatically")]
    fn a_stalled_audio_or_scene_still_has_a_wall_time_deadline() {
        let start = Instant::now();
        let mut clock = Clock {
            last: start,
            rest: None,
        };
        clock.step(start, true);
        clock.step(start + Duration::from_secs(20), true);
    }
}

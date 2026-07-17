//! The game countdown clock (`Timer` 10230): a resource the interpreter sets and
//! starts, ticked down here each frame while running and frozen at zero. A HUD or
//! a timer conditional reads [`GameClock::seconds`]; both land with the
//! interpreter's Timer arm.

use bevy::prelude::*;

/// The RM2000 game timer: `remaining` seconds counting down while `running`.
#[derive(Resource, Default)]
pub struct GameClock {
    pub remaining: f32,
    pub running: bool,
}

impl GameClock {
    /// Set the clock to `secs` whole seconds. The interpreter's Timer arm calls
    /// this before [`start`](Self::start).
    #[allow(dead_code)]
    pub fn set_secs(&mut self, secs: u32) {
        self.remaining = secs as f32;
    }

    /// Begin counting down.
    #[allow(dead_code)]
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Freeze the countdown, keeping the remaining time.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// The remaining whole seconds, for a HUD or a timer conditional.
    #[allow(dead_code)]
    pub fn seconds(&self) -> u32 {
        self.remaining.max(0.0) as u32
    }
}

pub struct GameClockPlugin;

impl Plugin for GameClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameClock>()
            .add_systems(Update, tick_clock);
    }
}

/// Count `remaining` down by real time while `running`, clamping at zero and
/// stopping once it gets there.
fn tick_clock(time: Res<Time>, mut clock: ResMut<GameClock>) {
    if !clock.running {
        return;
    }
    clock.remaining = (clock.remaining - time.delta_secs()).max(0.0);
    if clock.remaining <= 0.0 {
        clock.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_secs_sets_remaining_seconds() {
        let mut clock = GameClock::default();
        clock.set_secs(90);
        assert_eq!(clock.remaining, 90.0);
        assert_eq!(clock.seconds(), 90);
    }

    #[test]
    fn seconds_floors_then_clamps_at_zero() {
        let clock = GameClock {
            remaining: 5.9,
            running: false,
        };
        assert_eq!(clock.seconds(), 5);
        let clock = GameClock {
            remaining: -1.0,
            running: false,
        };
        assert_eq!(clock.seconds(), 0);
    }

    #[test]
    fn start_and_stop_toggle_running() {
        let mut clock = GameClock::default();
        assert!(!clock.running);
        clock.start();
        assert!(clock.running);
        clock.stop();
        assert!(!clock.running);
    }
}

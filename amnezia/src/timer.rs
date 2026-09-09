//! Scripted countdowns, including their map/battle visibility and pause rules.

mod view;

use bevy::prelude::*;

/// The RM2000 game timer: `remaining` seconds counting down while `running`.
#[derive(Resource, Default)]
pub struct GameClock {
    pub remaining: f32,
    pub running: bool,
    pub visible: bool,
    pub in_battle: bool,
    pub(crate) expired: bool,
}

impl GameClock {
    /// Set the clock to `secs` whole seconds. The interpreter's Timer arm calls
    /// this before [`start`](Self::start).
    pub fn set_secs(&mut self, secs: u32) {
        self.remaining = secs as f32 + 59.0 / 60.0;
        self.expired = false;
    }

    /// Begin counting down.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Freeze the countdown, keeping the remaining time.
    pub fn stop(&mut self) {
        self.running = false;
        self.visible = false;
    }

    /// The remaining whole seconds, for a HUD or a timer conditional.
    pub fn seconds(&self) -> u32 {
        self.remaining.max(0.0) as u32
    }

    pub fn apply(&mut self, params: &[i32], variables: &crate::state::Variables) {
        match params.first().copied() {
            Some(0) => {
                let value = params.get(2).copied().unwrap_or(0);
                let seconds = if params.get(1) == Some(&1) {
                    variables.get(value as u32)
                } else {
                    value
                };
                self.set_secs(seconds.max(0) as u32);
            }
            Some(1) => {
                self.visible = params.get(3).is_some_and(|v| *v != 0);
                self.in_battle = params.get(4).is_some_and(|v| *v != 0);
                self.start();
            }
            Some(2) => self.stop(),
            _ => {}
        }
    }

    fn advance(&mut self, dt: f32, in_battle: bool, paused: bool) {
        self.expired = false;
        if !self.running || paused || (in_battle && !self.in_battle) {
            return;
        }
        self.remaining = (self.remaining - dt).max(0.0);
        if self.seconds() == 0 {
            self.expired = true;
            self.stop();
        }
    }
}

/// Total whole seconds the game has been played, accumulated each frame and
/// persisted in the save (RM2000 shows it on the save screen); a resumed game keeps
/// counting from the saved total.
#[derive(Resource, Default)]
pub struct PlayTime {
    pub seconds: u64,
    frac: f32,
}

pub struct GameClockPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ClockTick;

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct ClockScene<'w> {
    menu: Option<Res<'w, crate::menu::MenuOpen>>,
    shop: Option<Res<'w, crate::shop::ShopOpen>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    battle: Option<Res<'w, crate::battle::BattleActive>>,
}

impl ClockScene<'_> {
    fn outside_game(&self) -> bool {
        self.title.as_ref().is_some_and(|v| v.0) || self.gameover.as_ref().is_some_and(|v| v.0)
    }

    fn paused(&self) -> bool {
        self.outside_game()
            || self.menu.as_ref().is_some_and(|v| v.0)
            || self.shop.as_ref().is_some_and(|v| v.0)
    }

    fn in_battle(&self) -> bool {
        self.battle.as_ref().is_some_and(|v| v.0)
    }
}

impl Plugin for GameClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameClock>()
            .init_resource::<PlayTime>()
            .add_systems(Update, tick_clock.in_set(ClockTick))
            .add_systems(Update, (tick_playtime, view::update).after(ClockTick))
            .add_systems(PostStartup, view::spawn);
    }
}

/// Count `remaining` down by real time while `running`, clamping at zero and
/// stopping once it gets there.
fn tick_clock(time: Res<Time>, scene: ClockScene, mut clock: ResMut<GameClock>) {
    clock.advance(time.delta_secs(), scene.in_battle(), scene.paused());
}

/// Accumulate real playtime, carrying the sub-second remainder so the whole-second
/// count stays accurate over a long session.
fn tick_playtime(time: Res<Time>, scene: ClockScene, mut play: ResMut<PlayTime>) {
    if scene.outside_game() {
        return;
    }
    play.frac += time.delta_secs();
    while play.frac >= 1.0 {
        play.seconds += 1;
        play.frac -= 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_secs_sets_remaining_seconds() {
        let mut clock = GameClock::default();
        clock.set_secs(90);
        assert_eq!(clock.seconds(), 90);
        clock.start();
        clock.advance(1.0 / 60.0, false, false);
        assert_eq!(clock.seconds(), 90);
    }

    #[test]
    fn seconds_floors_then_clamps_at_zero() {
        let clock = GameClock {
            remaining: 5.9,
            running: false,
            ..default()
        };
        assert_eq!(clock.seconds(), 5);
        let clock = GameClock {
            remaining: -1.0,
            running: false,
            ..default()
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

    #[test]
    fn original_timed_mission_shows_the_clock_and_runs_during_battles() {
        let mut clock = GameClock::default();
        let vars = crate::state::Variables::default();
        clock.apply(&[0, 0, 1200, 0, 0], &vars);
        clock.apply(&[1, 0, 900, 1, 1], &vars);
        assert!(clock.visible && clock.in_battle);
        assert_eq!(clock.seconds(), 1200);
        clock.advance(1.0, true, false);
        assert_eq!(clock.seconds(), 1199);
        clock.advance(10.0, false, true);
        assert_eq!(clock.seconds(), 1199);
        clock.apply(&[2, 0, 0, 0, 0], &vars);
        assert!(!clock.visible && !clock.running);
    }

    #[test]
    fn field_only_timer_pauses_in_battle_and_expires_once() {
        let mut clock = GameClock::default();
        clock.set_secs(2);
        clock.start();
        clock.advance(5.0, true, false);
        assert_eq!(clock.seconds(), 2);
        clock.advance(2.0, false, false);
        assert!(clock.expired && !clock.running);
        clock.advance(1.0, false, false);
        assert!(!clock.expired);
    }
}

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

mod scene;
pub(crate) use scene::SceneFrames;

/// Raw 60 Hz clock for input and transitions, including asynchronous waits.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct GameFrames {
    pub frame: u32,
    fraction: f64,
}

impl GameFrames {
    pub(crate) fn advance(&mut self, seconds: f64) {
        if !seconds.is_finite() || seconds < 0.0 {
            return;
        }
        self.sanitize();
        self.fraction += seconds * 60.0;
        // Nanosecond rounding at rates such as 144 Hz must not postpone a tick.
        let ticks = (self.fraction + 1e-6).floor();
        self.fraction = (self.fraction - ticks).max(0.0);
        self.frame = self.frame.wrapping_add(ticks as u64 as u32);
    }

    pub(crate) fn sanitize(&mut self) {
        if !(0.0..1.0).contains(&self.fraction) {
            self.fraction = 0.0;
        }
    }
}

pub(crate) struct TimingPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FrameClockSet;

impl Plugin for TimingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameFrames>()
            .init_resource::<SceneFrames>()
            .add_systems(
                PreUpdate,
                tick.in_set(FrameClockSet).after(crate::save::SaveSet),
            );
    }
}

fn tick(
    time: Res<Time>,
    mut frames: ResMut<GameFrames>,
    mut scene: ResMut<SceneFrames>,
    waiting: scene::Waiting,
) {
    let before = frames.frame;
    frames.advance(time.delta_secs_f64());
    if !waiting.pending() {
        scene.frame = scene.frame.wrapping_add(frames.frame.wrapping_sub(before));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_frames_match_at_low_high_and_irregular_render_rates() {
        for fps in [15, 30, 60, 120, 144] {
            let mut time = GameFrames::default();
            for _ in 0..fps * 60 {
                time.advance(1.0 / fps as f64);
            }
            assert_eq!(time.frame, 3600, "{fps} FPS");
        }
        let mut time = GameFrames::default();
        for dt in [0.2, 0.001, 0.13, 0.269, 0.4] {
            time.advance(dt);
        }
        assert_eq!(time.frame, 60);
        time.frame = u32::MAX;
        time.advance(1.0 / 60.0);
        assert_eq!(time.frame, 0);
    }

    #[test]
    fn frame_clock_round_trip_keeps_fraction_and_rejects_invalid_remainders() {
        let mut time = GameFrames::default();
        time.advance(1.0 / 144.0);
        let text = ron::to_string(&time).unwrap();
        let mut loaded = ron::from_str::<GameFrames>(&text).unwrap();
        assert_eq!(loaded, time);
        loaded.advance(1.0 / 60.0);
        assert_eq!(loaded.frame, 1);
        loaded.fraction = f64::NAN;
        loaded.advance(1.0 / 60.0);
        assert_eq!(loaded.frame, 2);
        assert_eq!(loaded.fraction, 0.0);
    }

    #[test]
    fn bevy_clock_keeps_counting_in_menus_battles_and_title_scenes() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TimingPlugin))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 144.0),
            ))
            .insert_resource(crate::menu::MenuOpen(true))
            .insert_resource(crate::shop::ShopOpen(true))
            .insert_resource(crate::battle::BattleActive(true))
            .insert_resource(crate::title::TitleActive(true));
        app.update();
        for _ in 0..144 {
            app.update();
        }
        assert_eq!(app.world().resource::<GameFrames>().frame, 60);
    }
}

//! Screen-tone interpolation and the picture/UI camera's world alignment.

use super::{ScreenEffect, ScreenShakeSet};
use crate::world::MainCamera;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

/// Pictures and UI remain above the world and do not inherit its tone.
pub const PICTURE_LAYER: usize = 3;

/// RM2000 neutral tone: every channel 100, i.e. no change.
const NEUTRAL: [f32; 4] = [100.0, 100.0, 100.0, 100.0];

/// The picture/UI camera stays untinted above the world.
#[derive(Component)]
pub struct FrontCamera;

/// The live screen tone: the currently displayed values, the command's target,
/// and the seconds left to reach it. Values are RM2000 0..200. Public so the save
/// system can snapshot the current tone and restore it (via [`Self::set_tone`]).
#[derive(Resource)]
pub struct TintState {
    current: [f32; 4],
    target: [f32; 4],
    secs_left: f32,
}

impl Default for TintState {
    fn default() -> Self {
        Self {
            current: NEUTRAL,
            target: NEUTRAL,
            secs_left: 0.0,
        }
    }
}

impl TintState {
    /// The currently displayed tone in RM2000 units (R, G, B, saturation; each
    /// 0..200, 100 neutral) — what a save snapshots.
    pub fn tone(&self) -> [f32; 4] {
        self.current
    }

    /// Restore a settled tone: snap both the displayed value and the target to
    /// `tone` with no interpolation left, so a loaded tint holds at once.
    pub fn set_tone(&mut self, tone: [f32; 4]) {
        self.current = tone;
        self.target = tone;
        self.secs_left = 0.0;
    }
}

pub struct ScreenTonePlugin;

impl Plugin for ScreenTonePlugin {
    fn build(&self, app: &mut App) {
        crate::legacy_colors::world::register(app);
        app.init_resource::<TintState>()
            .add_systems(Update, update_tone)
            .add_systems(
                PostUpdate,
                sync_front_camera
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// Update the shared tone while scene transitions hold its interpolation.
fn update_tone(
    transition: crate::transitions::TransitionPause,
    time: Res<Time>,
    mut effects: MessageReader<ScreenEffect>,
    mut state: ResMut<TintState>,
) {
    for effect in effects.read() {
        if let ScreenEffect::Tint { r, g, b, sat, secs } = *effect {
            state.target = [r as f32, g as f32, b as f32, sat as f32];
            state.secs_left = secs;
            if secs <= 0.0 {
                state.current = state.target;
            }
        }
    }
    if !transition.paused() {
        step_tint(&mut state, time.delta_secs());
    }
}

/// Advance the tone toward its target so it arrives after `secs_left` more
/// seconds (RM2000 interpolates the tone over the `TintScreen` duration).
fn step_tint(state: &mut TintState, dt: f32) {
    if state.secs_left <= 0.0 {
        state.current = state.target;
        return;
    }
    let t = (dt / state.secs_left).clamp(0.0, 1.0);
    for i in 0..4 {
        state.current[i] += (state.target[i] - state.current[i]) * t;
    }
    state.secs_left -= dt;
}

/// Keep the picture/UI camera aligned with the followed, shaken main camera, so
/// pictures placed relative to it stay in the right screen position. Runs after
/// the shake offset lands and before transform propagation.
fn sync_front_camera(
    main: Query<&Transform, (With<MainCamera>, Without<FrontCamera>)>,
    mut front: Query<&mut Transform, With<FrontCamera>>,
) {
    let Ok(source) = main.single() else {
        return;
    };
    let Ok(mut target) = front.single_mut() else {
        return;
    };
    target.translation = source.translation;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::legacy_colors::tone::{apply, uniform as tone_uniform};

    #[test]
    fn transitions_hold_tint_tweens_but_accept_immediate_command_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(250),
            ))
            .init_resource::<TintState>()
            .init_resource::<crate::transitions::Transition>()
            .add_message::<ScreenEffect>()
            .add_systems(Update, update_tone);
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .start(crate::transitions::Kind::Mosaic, true, 0, IVec2::ZERO);
        app.world_mut()
            .write_message(ScreenEffect::tint(&[0, 0, 0, 100, 20, 0]));
        app.update();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(app.world().resource::<TintState>().tone(), NEUTRAL);
        assert_eq!(app.world().resource::<TintState>().secs_left, 2.0);
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .clear();
        app.update();
        assert_eq!(app.world().resource::<TintState>().tone()[0], 87.5);
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .start(crate::transitions::Kind::Mosaic, true, 0, IVec2::ZERO);
        app.world_mut()
            .write_message(ScreenEffect::tint(&[50, 50, 50, 100, 0, 0]));
        app.update();
        assert_eq!(
            app.world().resource::<TintState>().tone(),
            [50.0, 50.0, 50.0, 100.0]
        );
    }

    #[test]
    fn channel_maps_neutral_and_clamps_to_original_byte_range() {
        assert_eq!(
            tone_uniform([100.0, 0.0, 50.0, 300.0]),
            Vec4::new(128.0, 0.0, 64.0, 255.0)
        );
    }

    #[test]
    fn neutral_tone_is_the_identity() {
        let src = [204, 102, 51];
        assert_eq!(apply(src, NEUTRAL), src);
        assert_eq!(tone_uniform(NEUTRAL), Vec4::splat(128.0));
    }

    #[test]
    fn below_one_hundred_darkens_with_original_integer_rounding() {
        assert_eq!(apply([153; 3], [50.0, 50.0, 50.0, 100.0]), [76; 3]);
    }

    #[test]
    fn above_one_hundred_blends_toward_white_instead_of_multiplying() {
        assert_eq!(
            apply([32, 156, 0], [150.0, 150.0, 150.0, 100.0]),
            [145, 207, 129]
        );
        assert_eq!(apply([102, 76, 51], [200.0, 200.0, 200.0, 100.0]), [255; 3]);
    }

    #[test]
    fn saturation_zero_uses_integer_luminance() {
        assert_eq!(apply([204, 51, 25], [100.0, 100.0, 100.0, 0.0]), [93; 3]);
    }

    #[test]
    fn saturation_above_one_hundred_uses_the_steeper_original_curve() {
        assert_eq!(
            apply([32, 156, 0], [100.0, 100.0, 100.0, 150.0]),
            [0, 211, 0]
        );
    }

    #[test]
    fn tint_interpolates_over_its_duration() {
        let mut state = TintState {
            current: NEUTRAL,
            target: [50.0, 50.0, 50.0, 50.0],
            secs_left: 1.0,
        };
        step_tint(&mut state, 0.5);
        assert!((state.current[0] - 75.0).abs() < 1e-4);
        assert!(state.secs_left > 0.0);
        step_tint(&mut state, 0.5);
        assert!((state.current[0] - 50.0).abs() < 1e-4);
    }

    #[test]
    fn zero_duration_tint_snaps_to_target() {
        let mut state = TintState {
            current: NEUTRAL,
            target: [0.0, 0.0, 0.0, 100.0],
            secs_left: 0.0,
        };
        step_tint(&mut state, 0.016);
        assert_eq!(state.current, [0.0, 0.0, 0.0, 100.0]);
    }
}

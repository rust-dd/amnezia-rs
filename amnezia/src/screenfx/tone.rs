//! Screen-tone interpolation and the picture camera's world alignment.

use super::{ScreenEffect, ScreenShakeSet};
use crate::world::MainCamera;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

/// Pictures and UI remain above the world and do not inherit its tone.
pub const PICTURE_LAYER: usize = 3;

/// RM2000 neutral tone: every channel 100, i.e. no change.
const NEUTRAL: [f64; 4] = [100.0; 4];

/// The picture camera stays untinted above the world.
#[derive(Component)]
pub struct FrontCamera;

/// The original retains fractional channels and an integer 60 Hz time remaining.
#[derive(Resource, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TintState {
    current: [f64; 4],
    target: [f64; 4],
    frames_left: u32,
    fraction: f64,
}

impl Default for TintState {
    fn default() -> Self {
        Self {
            current: NEUTRAL,
            target: NEUTRAL,
            frames_left: 0,
            fraction: 0.0,
        }
    }
}

impl TintState {
    /// The displayed channels in renderer units, with 100 neutral.
    pub fn tone(&self) -> [f32; 4] {
        self.current.map(|value| value as f32)
    }

    /// Restore a settled tone: snap both the displayed value and the target to
    /// `tone` with no interpolation left, so a loaded tint holds at once.
    pub fn set_tone(&mut self, tone: [f32; 4]) {
        self.current = tone.map(f64::from);
        self.target = self.current;
        self.frames_left = 0;
        self.fraction = 0.0;
    }

    pub(super) fn valid(&self) -> bool {
        self.current
            .iter()
            .chain(&self.target)
            .all(|value| value.is_finite() && (*value as f32).is_finite())
            && self.fraction.is_finite()
            && (0.0..1.0).contains(&self.fraction)
    }
}

pub struct ScreenTonePlugin;

#[derive(Resource, Default)]
pub(super) struct Inbox(bevy::ecs::message::MessageCursor<ScreenEffect>);

impl Inbox {
    fn apply(&mut self, effects: &Messages<ScreenEffect>, state: &mut TintState) {
        for effect in self.0.read(effects) {
            if let ScreenEffect::Tint { r, g, b, sat, secs } = *effect {
                state.target = [r, g, b, sat].map(f64::from);
                state.frames_left = (secs * 60.0).round().max(0.0) as u32;
                state.fraction = 0.0;
                if state.frames_left == 0 {
                    state.current = state.target;
                }
            }
        }
    }
}

impl Plugin for ScreenTonePlugin {
    fn build(&self, app: &mut App) {
        crate::legacy_colors::world::register(app);
        app.init_resource::<TintState>()
            .init_resource::<Inbox>()
            .add_systems(
                Update,
                (
                    update_tone
                        .in_set(super::ScreenAdvance)
                        .before(super::flash::channel::Advance),
                    receive_tone.in_set(super::ScreenEffectsSet),
                ),
            )
            .add_systems(
                PostUpdate,
                sync_front_camera
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// Update the shared tone while scene transitions hold its interpolation.
pub(super) fn update_tone(
    pause: super::EffectPause,
    time: Res<Time>,
    effects: Res<Messages<ScreenEffect>>,
    mut inbox: ResMut<Inbox>,
    mut state: ResMut<TintState>,
) {
    inbox.apply(&effects, &mut state);
    if !pause.paused() {
        step_tint(&mut state, time.delta_secs());
    }
}

pub(super) fn receive_tone(
    effects: Res<Messages<ScreenEffect>>,
    mut inbox: ResMut<Inbox>,
    mut state: ResMut<TintState>,
) {
    inbox.apply(&effects, &mut state);
}

pub(super) fn step_tint(state: &mut TintState, dt: f32) {
    if state.frames_left == 0 {
        return;
    }
    state.fraction += f64::from(dt) * 60.0;
    while state.fraction + 1e-6 >= 1.0 && state.frames_left > 0 {
        let frames = f64::from(state.frames_left);
        for i in 0..4 {
            state.current[i] = (state.current[i] * (frames - 1.0) + state.target[i]) / frames;
        }
        state.frames_left -= 1;
        state.fraction = (state.fraction - 1.0).max(0.0);
    }
    if state.frames_left == 0 {
        state.fraction = 0.0;
    }
}

/// Align the picture camera after shake and before transform propagation.
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

    #[test]
    fn tone_uses_original_integer_steps_and_keeps_its_subframe_at_15_to_144_fps() {
        let mut reference = Vec::from([NEUTRAL]);
        for left in (1..=60).rev() {
            let previous = *reference.last().unwrap();
            reference.push(std::array::from_fn(|i| {
                (previous[i] * f64::from(left - 1) + [70.0, 90.0, 110.0, 50.0][i]) / f64::from(left)
            }));
        }
        for fps in [15, 30, 60, 120, 144] {
            let mut state = TintState {
                target: [70.0, 90.0, 110.0, 50.0],
                frames_left: 60,
                ..default()
            };
            let mut clock = crate::timing::GameFrames::default();
            for _ in 0..fps * 2 {
                clock.advance(1.0 / f64::from(fps));
                step_tint(&mut state, 1.0 / fps as f32);
                assert_eq!(state.current, reference[clock.frame.min(60) as usize]);
                assert!(state.valid());
            }
        }
    }

    #[test]
    fn invalid_tone_channels_and_fractional_clocks_are_rejected() {
        for case in 0..5 {
            let mut state = TintState::default();
            match case {
                0 => state.current[0] = f64::NAN,
                1 => state.target[0] = f64::MAX,
                2 => state.fraction = f64::INFINITY,
                3 => state.fraction = -0.01,
                _ => state.fraction = 1.0,
            }
            assert!(!state.valid());
        }
    }

    use crate::legacy_colors::tone::{apply, uniform as tone_uniform};

    #[test]
    fn transitions_hold_tint_tweens_but_accept_immediate_command_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(250),
            ))
            .init_resource::<TintState>()
            .init_resource::<Inbox>()
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
        assert_eq!(app.world().resource::<TintState>().current, NEUTRAL);
        assert_eq!(app.world().resource::<TintState>().frames_left, 120);
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
        assert_eq!(apply(src, NEUTRAL.map(|v| v as f32)), src);
        assert_eq!(tone_uniform(NEUTRAL.map(|v| v as f32)), Vec4::splat(128.0));
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
            frames_left: 60,
            fraction: 0.0,
        };
        step_tint(&mut state, 0.5);
        assert!((state.current[0] - 75.0).abs() < 1e-4);
        assert_eq!(state.frames_left, 30);
        step_tint(&mut state, 0.5);
        assert!((state.current[0] - 50.0).abs() < 1e-4);
    }

    #[test]
    fn zero_duration_tint_snaps_to_target() {
        let mut state = TintState::default();
        state.set_tone([0.0, 0.0, 0.0, 100.0]);
        step_tint(&mut state, 0.016);
        assert_eq!(state.current, [0.0, 0.0, 0.0, 100.0]);
    }
}

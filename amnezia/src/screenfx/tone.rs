//! The RM2000 screen tone (`TintScreen` 11030) as a faithful camera post-process.
//!
//! RM2000's screen tone is R/G/B/Saturation, each 0..200 with 100 neutral. It
//! quantizes channels around 128, adjusts saturation, then applies the original
//! hard-light color table (0 = black, 100 = neutral, 200 = white). An
//! alpha overlay can only darken, so instead [`ScreenTone`] is a fullscreen
//! post-process on the main camera (Bevy's [`FullscreenMaterial`]) running
//! `shaders/screen_tone.wgsl`, mirroring `picture/render.rs`'s per-picture tone
//! but over the entire rendered scene.
//!
//! Pictures must stay untinted (RM2000 `affected_by_tint` defaults off), so they
//! render on [`PICTURE_LAYER`] via the separate [`FrontCamera`] that composites
//! above this post-process; that camera also owns the UI, keeping message and
//! menu windows untinted and above the pictures. [`sync_front_camera`] keeps it
//! aligned with the (followed, shaken) main camera so pictures stay in place.

use super::{ScreenEffect, ScreenShakeSet};
use crate::world::MainCamera;
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::core_pipeline::tonemapping::tonemapping;
use bevy::core_pipeline::{Core2d, Core2dSystems};
use bevy::ecs::schedule::{IntoScheduleConfigs, ScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;
use bevy::transform::TransformSystems;

/// The render layer pictures (and the front camera that draws them) live on, kept
/// clear of the layer-0 world the tone post-process covers so pictures render
/// untinted above it.
pub const PICTURE_LAYER: usize = 3;

/// RM2000 neutral tone: every channel 100, i.e. no change.
const NEUTRAL: [f32; 4] = [100.0, 100.0, 100.0, 100.0];

/// Quantized RGB/saturation tone channels; 128 is neutral. Present only on the
/// main camera so the pictures and UI remain untinted.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct ScreenTone {
    channels: Vec4,
}

impl Default for ScreenTone {
    fn default() -> Self {
        Self {
            channels: Vec4::splat(128.0),
        }
    }
}

impl FullscreenMaterial for ScreenTone {
    fn fragment_shader() -> ShaderRef {
        "shaders/screen_tone.wgsl".into()
    }

    /// This is a 2D game, so the pass runs in the 2D core pipeline rather than the
    /// 3D default.
    fn schedule() -> impl ScheduleLabel + Clone {
        Core2d
    }

    /// Run in the 2D `PostProcess` set (after the world main pass, before the 2D
    /// tonemapping step). The trait default targets `Core3dSystems`, which is not
    /// chained after the main pass in the 2D schedule.
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .in_set(Core2dSystems::PostProcess)
            .before(tonemapping)
    }
}

/// The camera that renders pictures and UI above the toned world. It carries no
/// [`ScreenTone`], so its contents stay untinted.
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
    /// `update_tone` repaints the camera's [`ScreenTone`] from it next frame.
    pub fn set_tone(&mut self, tone: [f32; 4]) {
        self.current = tone;
        self.target = tone;
        self.secs_left = 0.0;
    }
}

pub struct ScreenTonePlugin;

impl Plugin for ScreenTonePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FullscreenMaterialPlugin::<ScreenTone>::default())
            .init_resource::<TintState>()
            .add_systems(Update, update_tone)
            .add_systems(
                PostUpdate,
                sync_front_camera
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// Ingest a `TintScreen`, interpolate the current tone toward its target over the
/// command's duration, and push the result into the main camera's [`ScreenTone`].
fn update_tone(
    time: Res<Time>,
    mut effects: MessageReader<ScreenEffect>,
    mut state: ResMut<TintState>,
    mut tinted: Query<&mut ScreenTone>,
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
    step_tint(&mut state, time.delta_secs());
    let uniform = tone_uniform(state.current);
    for mut tone in &mut tinted {
        tone.channels = uniform;
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

/// The same quantized tone used by on-screen pictures.
fn tone_uniform(tone: [f32; 4]) -> Vec4 {
    crate::legacy_colors::tone::uniform(tone)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::legacy_colors::tone::apply;

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

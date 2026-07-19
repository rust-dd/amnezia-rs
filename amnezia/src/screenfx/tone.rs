//! The RM2000 screen tone (`TintScreen` 11030) as a faithful camera post-process.
//!
//! RM2000's screen tone is R/G/B/Saturation, each 0..200 with 100 neutral. It
//! multiplies the whole scene by `channel / 100` (so < 100 darkens, > 100
//! brightens) and blends toward luminance by the saturation (0 = grayscale). An
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

/// The per-camera tone uniform for `screen_tone.wgsl`. `rgb_sat.xyz` are the RGB
/// multipliers (1 = neutral) and `rgb_sat.w` the saturation (1 = neutral, 0 =
/// grayscale). Present on the main camera; absent elsewhere so only the world is
/// toned.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct ScreenTone {
    rgb_sat: Vec4,
}

impl Default for ScreenTone {
    fn default() -> Self {
        Self { rgb_sat: Vec4::ONE }
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
/// and the seconds left to reach it. Values are RM2000 0..200.
#[derive(Resource)]
struct TintState {
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
        tone.rgb_sat = uniform;
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

/// An RM2000 tone channel (0..200, 100 neutral) as a 0..2 shader multiplier, the
/// same mapping the picture tone uses.
fn channel(value: f32) -> f32 {
    (value / 100.0).clamp(0.0, 2.0)
}

/// The `rgb_sat` uniform for a tone `[r, g, b, saturation]` in RM2000 units.
fn tone_uniform(tone: [f32; 4]) -> Vec4 {
    Vec4::new(
        channel(tone[0]),
        channel(tone[1]),
        channel(tone[2]),
        channel(tone[3]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CPU mirror of `screen_tone.wgsl`: apply an RM2000 tone `[r, g, b, sat]`
    /// to a linear source colour, for asserting the tone maths the shader runs.
    fn tone_color(src: [f32; 3], tone: [f32; 4]) -> [f32; 3] {
        let m = tone_uniform(tone);
        let rgb = [src[0] * m.x, src[1] * m.y, src[2] * m.z];
        let luma = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
        let s = m.w;
        [
            luma + (rgb[0] - luma) * s,
            luma + (rgb[1] - luma) * s,
            luma + (rgb[2] - luma) * s,
        ]
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6)
    }

    #[test]
    fn channel_maps_neutral_and_clamps_to_0_2() {
        assert_eq!(channel(100.0), 1.0);
        assert_eq!(channel(0.0), 0.0);
        assert_eq!(channel(50.0), 0.5);
        assert_eq!(channel(200.0), 2.0);
        assert_eq!(channel(300.0), 2.0);
    }

    #[test]
    fn neutral_tone_is_the_identity() {
        let src = [0.8, 0.4, 0.2];
        assert!(close(tone_color(src, NEUTRAL), src));
        assert_eq!(tone_uniform(NEUTRAL), Vec4::ONE);
    }

    #[test]
    fn below_one_hundred_darkens() {
        // A uniform 50 tone halves every channel.
        let out = tone_color([0.6, 0.6, 0.6], [50.0, 50.0, 50.0, 100.0]);
        assert!(close(out, [0.3, 0.3, 0.3]));
    }

    #[test]
    fn above_one_hundred_brightens() {
        // A uniform 200 tone doubles every channel (the game's [200,200,200,200]).
        let out = tone_color([0.4, 0.3, 0.2], [200.0, 200.0, 200.0, 100.0]);
        assert!(close(out, [0.8, 0.6, 0.4]));
    }

    #[test]
    fn saturation_zero_is_grayscale() {
        // Neutral RGB, saturation 0 (the game's [100,100,100,0] flashback): every
        // channel collapses to the luminance.
        let src = [0.8, 0.2, 0.1];
        let out = tone_color(src, [100.0, 100.0, 100.0, 0.0]);
        let luma = 0.299 * src[0] + 0.587 * src[1] + 0.114 * src[2];
        assert!(close(out, [luma, luma, luma]));
    }

    #[test]
    fn saturation_above_one_hundred_oversaturates() {
        // saturation 200 pushes channels away from the luminance (past the source).
        let src = [0.7, 0.5, 0.2];
        let out = tone_color(src, [100.0, 100.0, 100.0, 200.0]);
        let luma = 0.299 * src[0] + 0.587 * src[1] + 0.114 * src[2];
        assert!(out[0] > src[0] && out[2] < src[2]);
        assert!((out[0] - luma).abs() > (src[0] - luma).abs());
    }

    #[test]
    fn tint_interpolates_over_its_duration() {
        // From neutral toward a 50 tone over 1 s: partway at half, arrived at end.
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

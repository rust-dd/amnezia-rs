//! Screen presentation effects driven by the event interpreter: the erase/show
//! black fade, the color tint, the brief flash, and the camera shake. The
//! interpreter stays decoupled by emitting a [`ScreenEffect`] message; the
//! plugins here consume it.
//!
//! The color tint is a faithful camera post-process (see the [`tone`] submodule)
//! that can darken, brighten, and desaturate the whole scene; flash and fade stay
//! as fullscreen UI overlays, and the shake offsets the camera. The [`shake`] and
//! [`fade`] submodules hold the RM2000-matched motion and timing maths.
//!
//! The shake avoids touching `player.rs`: [`apply_camera_shake`] runs in
//! `PostUpdate` (after the `Update` `camera_follow` has set the base position)
//! and adds an offset the follow overwrites again next frame, so it never
//! accumulates.

use crate::world::MainCamera;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use flash::Flashing;
use shake::ShakeState;

mod fade;
mod flash;
mod shake;
mod tone;
mod weather;

pub use fade::transition_secs;
pub use tone::{FrontCamera, PICTURE_LAYER, ScreenTone, TintState};
pub use weather::{Weather, WeatherStrength};

/// A screen effect the interpreter emits; consumed by [`step_effects`] (and, for
/// the tint, by the [`tone`] submodule).
#[derive(Message, Debug, Clone, PartialEq)]
pub enum ScreenEffect {
    /// Fade to black over `secs` and hold (`EraseScreen` 11010).
    Erase { secs: f32 },
    /// Fade from black back to the scene over `secs` (`ShowScreen` 11020).
    Show { secs: f32 },
    /// Shift the screen color over `secs` (`TintScreen` 11030). `r,g,b,sat` are
    /// RM2000 0..200, 100 = neutral.
    Tint {
        r: i32,
        g: i32,
        b: i32,
        sat: i32,
        secs: f32,
    },
    /// Flash `r,g,b` (0..31) at `intensity` (0..31), decaying over `secs`
    /// (`FlashScreen` 11040).
    Flash {
        r: i32,
        g: i32,
        b: i32,
        intensity: i32,
        secs: f32,
    },
    /// Shake the screen at `power`/`speed` for `secs` (`ShakeScreen` 11050).
    Shake { power: i32, speed: i32, secs: f32 },
}

impl ScreenEffect {
    /// Map a `TintScreen` command `[r, g, b, saturation, duration, wait]`; the
    /// duration is in tenths of a second.
    pub fn tint(params: &[i32]) -> Self {
        Self::Tint {
            r: param(params, 0, 100),
            g: param(params, 1, 100),
            b: param(params, 2, 100),
            sat: param(params, 3, 100),
            secs: tenths(param(params, 4, 0)),
        }
    }

    /// Map a `FlashScreen` command `[r, g, b, intensity, duration, wait]`.
    pub fn flash(params: &[i32]) -> Self {
        Self::Flash {
            r: param(params, 0, 0),
            g: param(params, 1, 0),
            b: param(params, 2, 0),
            intensity: param(params, 3, 0),
            secs: tenths(param(params, 4, 0)),
        }
    }

    /// Map a `ShakeScreen` command `[power, speed, duration, wait]`.
    pub fn shake(params: &[i32]) -> Self {
        Self::Shake {
            power: param(params, 0, 0),
            speed: param(params, 1, 0),
            secs: tenths(param(params, 2, 0)),
        }
    }
}

/// The value at `index`, or `fallback` when the list is too short.
fn param(params: &[i32], index: usize, fallback: i32) -> i32 {
    params.get(index).copied().unwrap_or(fallback)
}

/// Convert an RM2000 tenths-of-a-second duration to seconds.
fn tenths(v: i32) -> f32 {
    v as f32 / 10.0
}

/// The set holding [`apply_camera_shake`], so pictures can order their placement
/// after the shake offset lands (they track the shaken camera).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScreenShakeSet;

/// Which overlay a fullscreen node is, so one query drives both.
#[derive(Component, Clone, Copy)]
enum FxLayer {
    Flash,
    Fade,
}

/// The live effect state: fade progress, the decaying flash, and the running
/// shake with its current offset. The tint lives in the [`tone`] submodule.
#[derive(Resource, Default)]
struct Fx {
    fade_alpha: f32,
    fade_target: f32,
    fade_secs: f32,
    flash: Option<Flashing>,
    shake: ShakeState,
    shake_offset: Vec2,
}

pub struct ScreenFxPlugin;

impl Plugin for ScreenFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ScreenEffect>()
            .init_resource::<Fx>()
            .add_plugins(tone::ScreenTonePlugin)
            .add_plugins(weather::WeatherPlugin)
            .add_systems(Startup, spawn_overlays)
            .add_systems(Update, step_effects)
            .add_systems(
                PostUpdate,
                apply_camera_shake
                    .in_set(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// The two fullscreen overlays. They render on the front (picture/UI) camera, so
/// they sit above the toned world and pictures. The flash sits below the
/// menu/message UI; the erase fade sits above everything so a black-out truly
/// covers the screen. Weather draws on its own layer-0 sprites (see [`weather`]).
fn spawn_overlays(mut commands: Commands) {
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(-10),
        FxLayer::Flash,
    ));
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(1003),
        FxLayer::Fade,
    ));
}

/// An absolutely-positioned node filling the whole screen.
fn full_screen() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

/// A fully transparent background, the resting state of every overlay.
fn transparent() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0))
}

/// Ingest new effects, advance every running effect, and repaint the overlays.
fn step_effects(
    time: Res<Time>,
    mut effects: MessageReader<ScreenEffect>,
    mut fx: ResMut<Fx>,
    mut layers: Query<(&mut BackgroundColor, &FxLayer)>,
) {
    for effect in effects.read() {
        apply_effect(&mut fx, effect);
    }
    let dt = time.delta_secs();
    step_fade(&mut fx, dt);
    step_flash(&mut fx, dt);
    step_shake(&mut fx, dt);
    for (mut background, layer) in &mut layers {
        background.0 = overlay_color(&fx, *layer);
    }
}

/// Fold one incoming effect into the live state. The tint is handled separately
/// by the [`tone`] submodule, so it is a no-op here.
fn apply_effect(fx: &mut Fx, effect: &ScreenEffect) {
    match *effect {
        ScreenEffect::Erase { secs } => {
            fx.fade_target = 1.0;
            fx.fade_secs = secs;
        }
        ScreenEffect::Show { secs } => {
            fx.fade_target = 0.0;
            fx.fade_secs = secs;
        }
        ScreenEffect::Tint { .. } => {}
        ScreenEffect::Flash {
            r,
            g,
            b,
            intensity,
            secs,
        } => {
            fx.flash = Some(Flashing::new(r, g, b, intensity, secs));
        }
        ScreenEffect::Shake { power, speed, secs } => {
            fx.shake.start(power, speed, secs);
        }
    }
}

/// Advance the fade linearly so it reaches its target after `fade_secs` seconds
/// (a zero duration snaps, matching an instant transition).
fn step_fade(fx: &mut Fx, dt: f32) {
    if fx.fade_secs <= 0.0 {
        fx.fade_alpha = fx.fade_target;
        return;
    }
    fx.fade_alpha = approach(fx.fade_alpha, fx.fade_target, dt / fx.fade_secs);
}

fn step_flash(fx: &mut Fx, dt: f32) {
    if fx.flash.as_mut().is_some_and(|flash| !flash.step(dt)) {
        fx.flash = None;
    }
}

fn step_shake(fx: &mut Fx, dt: f32) {
    fx.shake_offset = Vec2::new(fx.shake.step(dt), 0.0);
}

/// The color an overlay should paint given the current state.
fn overlay_color(fx: &Fx, layer: FxLayer) -> Color {
    match layer {
        FxLayer::Fade => Color::srgba(0.0, 0.0, 0.0, fx.fade_alpha),
        FxLayer::Flash => match &fx.flash {
            Some(flash) => flash.color(),
            None => Color::srgba(0.0, 0.0, 0.0, 0.0),
        },
    }
}

/// Add the current shake offset to the camera after `camera_follow` set its base
/// position. Runs every frame; the follow re-centres next frame, so the offset
/// never accumulates.
fn apply_camera_shake(fx: Res<Fx>, mut cameras: Query<&mut Transform, With<MainCamera>>) {
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    camera.translation.x += fx.shake_offset.x;
    camera.translation.y += fx.shake_offset.y;
}

/// Step `cur` toward `target` by at most `step`.
fn approach(cur: f32, target: f32, step: f32) -> f32 {
    if cur < target {
        (cur + step).min(target)
    } else {
        (cur - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_reaches_target_over_its_duration() {
        let mut fx = Fx {
            fade_target: 1.0,
            fade_secs: 0.5,
            ..default()
        };
        step_fade(&mut fx, 0.25);
        assert!((fx.fade_alpha - 0.5).abs() < 1e-6);
        step_fade(&mut fx, 0.25);
        assert!((fx.fade_alpha - 1.0).abs() < 1e-6);
    }

    #[test]
    fn instant_fade_snaps_to_target() {
        let mut fx = Fx {
            fade_target: 1.0,
            fade_secs: 0.0,
            ..default()
        };
        step_fade(&mut fx, 0.016);
        assert_eq!(fx.fade_alpha, 1.0);
    }

    #[test]
    fn approach_moves_toward_target_without_overshoot() {
        assert_eq!(approach(0.0, 1.0, 0.3), 0.3);
        assert_eq!(approach(0.9, 1.0, 0.3), 1.0);
        assert_eq!(approach(1.0, 0.0, 0.3), 0.7);
        assert_eq!(approach(0.1, 0.0, 0.3), 0.0);
    }

    #[test]
    fn tint_command_maps_params_and_tenths_duration() {
        assert_eq!(
            ScreenEffect::tint(&[70, 60, 70, 100, 5, 1]),
            ScreenEffect::Tint {
                r: 70,
                g: 60,
                b: 70,
                sat: 100,
                secs: 0.5,
            }
        );
    }

    #[test]
    fn flash_command_maps_intensity_and_duration() {
        assert_eq!(
            ScreenEffect::flash(&[31, 31, 31, 20, 5, 0]),
            ScreenEffect::Flash {
                r: 31,
                g: 31,
                b: 31,
                intensity: 20,
                secs: 0.5,
            }
        );
    }

    #[test]
    fn shake_command_maps_power_speed_duration() {
        assert_eq!(
            ScreenEffect::shake(&[3, 4, 40, 0]),
            ScreenEffect::Shake {
                power: 3,
                speed: 4,
                secs: 4.0,
            }
        );
    }
}

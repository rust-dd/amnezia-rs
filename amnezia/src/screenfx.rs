//! Event-driven tint, flash and camera shake. Erase/show is handled by the shared
//! scene-transition controller. Tint is a world-camera post-process; flash uses
//! a fullscreen overlay below message/menu windows.
//!
//! The shake avoids touching `player.rs`: [`apply_camera_shake`] runs in
//! `PostUpdate` (after camera follow has set the base position)
//! and adds an offset the follow overwrites again next frame, so it never
//! accumulates.

use crate::world::MainCamera;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use flash::Flashing;
use shake::ShakeState;

mod flash;
mod shake;
mod tone;
mod weather;

pub use tone::{FrontCamera, PICTURE_LAYER, ScreenTone, TintState};
pub use weather::{Weather, WeatherStrength};

/// A screen effect the interpreter emits; consumed by [`step_effects`] (and, for
/// the tint, by the [`tone`] submodule).
#[derive(Message, Debug, Clone, PartialEq)]
pub enum ScreenEffect {
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

#[derive(Component)]
struct FlashOverlay;

/// Flash and shake state. The tint lives in the [`tone`] submodule.
#[derive(Resource, Default)]
struct Fx {
    flash: Option<Flashing>,
    shake: ShakeState,
    shake_offset: Vec2,
}

pub(crate) fn reset_transient(world: &mut World) {
    world.insert_resource(Fx::default());
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

fn spawn_overlays(mut commands: Commands) {
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(-10),
        FlashOverlay,
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
    transition: Option<Res<crate::transitions::Transition>>,
    time: Res<Time>,
    mut effects: MessageReader<ScreenEffect>,
    mut fx: ResMut<Fx>,
    mut layers: Query<&mut BackgroundColor, With<FlashOverlay>>,
) {
    for effect in effects.read() {
        apply_effect(&mut fx, effect);
    }
    if !transition.as_ref().is_some_and(|t| t.busy()) {
        let dt = time.delta_secs();
        step_flash(&mut fx, dt);
        step_shake(&mut fx, dt);
    }
    for mut background in &mut layers {
        background.0 = fx.flash.as_ref().map_or(Color::NONE, |flash| flash.color());
    }
}

/// Fold one incoming effect into the live state. The tint is handled separately
/// by the [`tone`] submodule, so it is a no-op here.
fn apply_effect(fx: &mut Fx, effect: &ScreenEffect) {
    match *effect {
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

fn step_flash(fx: &mut Fx, dt: f32) {
    if fx.flash.as_mut().is_some_and(|flash| !flash.step(dt)) {
        fx.flash = None;
    }
}

fn step_shake(fx: &mut Fx, dt: f32) {
    fx.shake_offset = Vec2::new(fx.shake.step(dt), 0.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_transition_holds_a_flash_until_scene_updates_resume() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ))
            .init_resource::<Fx>()
            .init_resource::<crate::transitions::Transition>()
            .add_message::<ScreenEffect>()
            .add_systems(Update, step_effects);
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .start(crate::transitions::Kind::Mosaic, true, 0, IVec2::ZERO);
        app.world_mut()
            .write_message(ScreenEffect::flash(&[31, 31, 31, 31, 10, 0]));
        app.update();
        let initial = app.world().resource::<Fx>().flash.as_ref().unwrap().color();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Fx>().flash.as_ref().unwrap().color(),
            initial
        );
        app.world_mut()
            .resource_mut::<crate::transitions::Transition>()
            .clear();
        app.update();
        assert_ne!(
            app.world().resource::<Fx>().flash.as_ref().unwrap().color(),
            initial
        );
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

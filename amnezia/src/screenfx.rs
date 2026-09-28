//! Event-driven tint, flash and camera shake. Erase/show is handled by the shared
//! scene-transition controller. Tint is applied to world bitmaps; flash uses
//! a fullscreen overlay below message/menu windows.
//!
//! Camera projection replaces its previous shake offset, including while the
//! character update is suspended by an asynchronous operation.

use crate::world::MainCamera;
use bevy::prelude::*;
use flash::Flashing;
use shake::ShakeState;

pub(crate) mod battle_smoke;
#[cfg(test)]
mod battle_tests;
mod camera;
pub(crate) mod flash;
#[cfg(test)]
mod logical_tests;
#[cfg(test)]
mod map_tests;
#[cfg(test)]
mod pause_tests;
#[cfg(test)]
mod save_tests;
pub(crate) mod saved;
mod shake;
mod tone;
mod weather;

pub(crate) use camera::CameraShake;
pub(crate) use flash::smoke as flash_smoke;
pub use tone::{FrontCamera, PICTURE_LAYER, TintState};
pub(crate) use weather::rain::Canvas as WeatherCanvas;
pub(crate) use weather::rain::smoke as weather_smoke;
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

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ScreenEffectsSet;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ScreenAdvance;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MapScreenReset;

#[derive(bevy::ecs::system::SystemParam)]
struct EffectPause<'w> {
    transition: crate::transitions::TransitionPause<'w>,
    scene: crate::world::ScenePause<'w>,
}

impl EffectPause<'_> {
    fn paused(&self) -> bool {
        self.transition.paused() || self.scene.screen_effects_paused()
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ScreenShake<'w> {
    fx: Option<Res<'w, Fx>>,
}

impl ScreenShake<'_> {
    pub(crate) fn offset(&self) -> Vec2 {
        self.fx.as_ref().map_or(Vec2::ZERO, |fx| fx.shake_offset)
    }
}

#[derive(Component)]
pub(crate) struct FlashOverlay;

/// Flash and shake state. The tint lives in the [`tone`] submodule.
#[derive(Resource, Default)]
pub(crate) struct Fx {
    flash: Option<Flashing>,
    shake: ShakeState,
    shake_offset: Vec2,
}

pub(crate) fn reset_transient(world: &mut World) {
    world.insert_resource(Fx::default());
    weather::rain::reset(world);
}

pub struct ScreenFxPlugin;

impl Plugin for ScreenFxPlugin {
    fn build(&self, app: &mut App) {
        crate::teleport::rebuild::register(
            app,
            crate::teleport::rebuild::Stage::Reset,
            clear_map_flash,
        );
        app.add_message::<ScreenEffect>()
            .add_message::<crate::world::MapRebuilt>()
            .add_message::<crate::world::MapEffectsReset>()
            .init_resource::<Fx>()
            .init_resource::<camera::Applied>()
            .init_resource::<flash::channel::Inbox>()
            .add_plugins(tone::ScreenTonePlugin)
            .add_plugins(weather::WeatherPlugin)
            .configure_sets(
                Update,
                (
                    ScreenAdvance
                        .after(crate::dialogue::MessageUpdate)
                        .after(crate::timer::ClockTick)
                        .before(crate::animation::AnimationSet::Advance)
                        .before(crate::interpreter::InterpreterStep),
                    ScreenEffectsSet
                        .after(crate::interpreter::InterpreterStep)
                        .after(crate::animation::AnimationSet::Advance),
                ),
            )
            .add_systems(Startup, flash::channel::spawn_overlay)
            .add_systems(
                Update,
                step_effects
                    .in_set(flash::channel::Advance)
                    .in_set(ScreenAdvance)
                    .after(crate::interpreter::ParallelStep)
                    .before(crate::animation::AnimationSet::Advance),
            )
            .add_systems(
                Update,
                flash::channel::scene_entry
                    .after(crate::battle::flow::BattleFlowSet)
                    .before(ScreenEffectsSet)
                    .before(crate::animation::AnimationSet::Start),
            )
            .add_systems(Update, flash::channel::receive.in_set(ScreenEffectsSet))
            .add_systems(
                PostUpdate,
                flash::channel::paint
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
        app.add_systems(
            Update,
            apply_camera_shake
                .in_set(ScreenShakeSet)
                .after(crate::player::CameraFollow)
                .after(ScreenAdvance)
                .before(crate::animation::AnimationSet::Advance)
                .before(crate::interpreter::InterpreterStep),
        );
    }
}

fn clear_map_flash(mut rebuilt: MessageReader<crate::world::MapEffectsReset>, mut fx: ResMut<Fx>) {
    if rebuilt.read().count() != 0 {
        fx.flash = None;
    }
}

/// Advance the screen before animations and foreground event commands.
fn step_effects(
    pause: EffectPause,
    time: Res<Time>,
    battle: Option<Res<crate::battle::Battle>>,
    mut pending: flash::channel::PendingEffects,
    mut commands: Commands,
    mut fx: ResMut<Fx>,
) {
    pending.apply(&mut fx, &mut commands);
    pending.scene_entry(battle.as_deref(), &mut fx, &mut commands);
    if !pause.paused() {
        let dt = time.delta_secs();
        step_flash(&mut fx, dt);
        step_shake(&mut fx, dt);
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

fn apply_camera_shake(
    mut shake: CameraShake,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    let point = shake.reproject(camera.translation.truncate());
    camera.translation.x = point.x;
    camera.translation.y = point.y;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_flash_plane_covers_both_scenes_below_animation_cells_and_windows() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, flash::channel::spawn_overlay);
        app.update();
        let (sprite, position, layers) = app
            .world_mut()
            .query_filtered::<(&Sprite, &Transform, &bevy::camera::visibility::RenderLayers), With<FlashOverlay>>()
            .single(app.world())
            .unwrap();
        assert_eq!(sprite.custom_size, Some(Vec2::new(320.0, 240.0)));
        assert_eq!(position.translation, Vec3::new(0.0, 0.0, 300.0));
        assert_eq!(*layers, crate::animation::overlay_layer());
    }

    #[test]
    fn scene_transition_holds_a_flash_until_scene_updates_resume() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ))
            .init_resource::<Fx>()
            .init_resource::<crate::screenfx::flash::channel::Inbox>()
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

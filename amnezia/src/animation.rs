//! RM2000 effect cells, sounds and flashes on the shared logical animation clock.
//! Each map/battle slot owns one cast. Map casts retain their character target
//! and refresh their placement after movement and camera shake, without restarting.
//! Overlay positions are measured from screen centre with y growing downward;
//! the animation definition supplies screen scope and head/centre/feet offsets.

mod cells;
mod map;
mod playback;
mod render;
#[cfg(test)]
mod save_tests;
pub(crate) mod scene;
pub(crate) mod smoke;

use crate::assets::{asset_root, load_ron};
use crate::audio::AudioRequest;
use crate::battle::BattleActive;
use crate::menu::MenuOpen;
use crate::player::Player;
use crate::shop::ShopOpen;
use crate::title::TitleActive;
use crate::world::{EventSprite, MainCamera};
use amnezia_data::{AnimationDef, AnimationTimingDef};
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use map::resolve_map_animation;
use playback::{start_animations, step_animations, track_active_animations};
use render::{FlashStamp, fade_flashes, spawn_screen_flash};

pub(crate) use map::flash_smoke as map_flash_smoke;
pub(crate) use map::smoke as map_smoke;
pub(crate) use playback::AnimationSet;
pub(crate) mod start;
pub(crate) use playback::reset_transient;
pub(crate) use playback::saved;
pub(crate) use render::flash_power_level;
pub(crate) use render::{FlashQuad as ScreenFlash, clear_screen_flash};
pub use render::{overlay_layer, overlay_translation};

/// Each data frame lasts two 60 Hz ticks (`battle_animation.cpp::GetRealFrame`),
/// independently of the flash envelope.
pub const FRAME_SECS: f32 = 1.0 / 30.0;

/// The flash channel `flash_scope` value that flashes the whole screen; `1`
/// flashes the target, `0` nothing.
const FLASH_SCOPE_SCREEN: u32 = 2;
const FLASH_SCOPE_TARGET: u32 = 1;

/// An [`AnimationDef::scope`] of `1` covers the whole screen: its cells draw once,
/// centred, instead of once per target. `0` draws at each target.
const SCOPE_SCREEN: u32 = 1;

/// The [`AnimationDef::position`] vertical anchors that shift the effect off the
/// target centre: `0` (head/up) lifts it by half the target height, `2`
/// (feet/down) drops it by half; `1` (centre) and anything else leave it centred
/// (EasyRPG `battle_animation.cpp` `CalculateOffset`).
const POSITION_UP: u32 = 0;
const POSITION_DOWN: u32 = 2;

/// The fixed target height RM2000 uses for a map animation (opcode 11210):
/// EasyRPG `BattleAnimationMap::DrawSingle` uses `character_height = 24`.
const MAP_CHARACTER_HEIGHT: f32 = 24.0;

/// Screen-scoped map effects use the centre-origin overlay's origin.
const MAP_SCREEN_CENTER: Vec2 = Vec2::ZERO;

/// Target centre in y-down screen offsets; pixel height determines head/feet anchors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimAnchor {
    pub pos: Vec2,
    pub height: f32,
}

/// One cast shares a sound timeline across all targets. Screen scope draws cells
/// once at `screen_center` but still flashes every target; `global` overrides
/// scope with 3×3 screen tiling (RM2000 opcode 11210).
#[derive(Message)]
pub struct PlayAnimation {
    pub slot: AnimationSlot,
    /// Keep map effects attached after their initial screen projection.
    pub map_target: Option<AnimTarget>,
    pub anim_id: u32,
    pub targets: Vec<AnimAnchor>,
    pub screen_center: Vec2,
    pub global: bool,
    /// Front-view party targets play only sound, truncated to 40 game frames.
    pub sound_only: bool,
}

/// The original keeps one map animation and one battle animation per target side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationSlot {
    Map,
    Party,
    Enemies,
}

/// Target tint matched by screen position: byte RGB and 0–31 power drive
/// [`render::flash_envelope`]. Front-view party targets have no sprite to tint.
#[derive(Message)]
pub struct BattlerFlash {
    pub pos: Vec2,
    pub rgb: [u8; 3],
    pub power: u32,
    pub age: u32,
}

/// Character reference with "this event" already resolved to a concrete event ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnimTarget {
    Hero,
    Event(u32),
}

/// Map opcode 11210 request. [`resolve_map_animation`] projects the character to
/// screen space, keeping camera queries out of the interpreter. `global` tiles 3×3.
#[derive(Message)]
pub struct ShowMapAnimation {
    pub anim_id: u32,
    pub target: AnimTarget,
    pub global: bool,
}

/// Every converted battle animation, loaded once at plugin build.
#[derive(Resource)]
pub struct AnimationLibrary(pub Vec<AnimationDef>);

/// Battle actions wait only for their own slots, never for a surviving map cast.
#[derive(Resource, Default, PartialEq, Eq)]
pub struct ActiveAnimations {
    pub total: usize,
    pub battle: usize,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        cells::register(app);
        map::flash::register(app);
        scene::register(app);
        saved::register(app);
        start::register(app);
        crate::teleport::rebuild::register(
            app,
            crate::teleport::rebuild::Stage::Reset,
            playback::clear_map_animations,
        );
        app.add_message::<PlayAnimation>()
            .add_message::<ShowMapAnimation>()
            .add_message::<crate::world::MapRebuilt>()
            .add_message::<BattlerFlash>()
            .insert_resource(AnimationLibrary(load_ron(&format!(
                "{}/animations.ron",
                asset_root()
            ))))
            .init_resource::<ActiveAnimations>()
            .add_systems(Startup, spawn_overlay_camera)
            .add_systems(
                Update,
                (
                    (fade_flashes, step_animations, track_active_animations)
                        .chain()
                        .in_set(AnimationSet::Advance)
                        .before(crate::interpreter::InterpreterStep)
                        .after(crate::teleport::MapTransfer),
                    debug_preview
                        .before(AnimationSet::Start)
                        .after(crate::interpreter::InterpreterStep)
                        .after(AnimationSet::Advance),
                ),
            )
            .add_systems(
                PostUpdate,
                (playback::follow_map_animations, track_active_animations)
                    .chain()
                    .after(crate::screenfx::ScreenShakeSet)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

/// Fixed screen-space camera above the world/pictures and below UI windows.
fn spawn_overlay_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 320.0,
                height: 240.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::default(),
        overlay_layer(),
    ));
}

/// Cell anchors apply scope and head/feet offsets; flash anchors stay at target centres.
fn draw_anchors(
    def: &AnimationDef,
    targets: &[AnimAnchor],
    screen_center: Vec2,
    global: bool,
) -> Vec<Vec2> {
    if global {
        return render::global_anchors(screen_center);
    }
    if def.scope == SCOPE_SCREEN {
        return vec![screen_center];
    }
    targets
        .iter()
        .map(|t| Vec2::new(t.pos.x, t.pos.y + position_offset(def.position, t.height)))
        .collect()
}

/// Timings use 1-based frames; `frame` is 0-based. Sound fires once per cast,
/// while target flashes fire per anchor, matching EasyRPG `BattleAnimation`.
fn fire_timings(
    commands: &mut Commands,
    audio: &mut MessageWriter<AudioRequest>,
    battler_flash: &mut MessageWriter<BattlerFlash>,
    def: &AnimationDef,
    frame: usize,
    flash_anchors: &[Vec2],
    flash_stamp: Option<FlashStamp>,
) {
    for timing in def.timings.iter().filter(|t| t.frame as usize == frame + 1) {
        emit_sound(audio, timing);
        if let Some(stamp) = flash_stamp {
            emit_flash(commands, battler_flash, timing, flash_anchors, stamp);
        }
    }
}

/// Animation timings carry their own volume/tempo, mapped like System sound effects.
fn emit_sound(audio: &mut MessageWriter<AudioRequest>, timing: &AnimationTimingDef) {
    if let Some(request) = AudioRequest::se(&timing.se_name, timing.se_volume, timing.se_tempo) {
        audio.write(request);
    }
}

/// Target flashes address battlers; screen flashes share the event flash plane.
fn emit_flash(
    commands: &mut Commands,
    battler_flash: &mut MessageWriter<BattlerFlash>,
    timing: &AnimationTimingDef,
    flash_anchors: &[Vec2],
    stamp: FlashStamp,
) {
    let rgb = [
        flash_channel(timing.flash_red),
        flash_channel(timing.flash_green),
        flash_channel(timing.flash_blue),
    ];
    let power = timing.flash_power;
    match timing.flash_scope {
        FLASH_SCOPE_SCREEN => spawn_screen_flash(commands, rgb, power, stamp),
        FLASH_SCOPE_TARGET => {
            for &pos in flash_anchors {
                battler_flash.write(BattlerFlash {
                    pos,
                    rgb,
                    power,
                    age: stamp.age,
                });
            }
        }
        _ => {}
    }
}

/// RPG_RT expands each 5-bit channel by shifting, not by normalizing to 255.
fn flash_channel(value: u32) -> u8 {
    (value.min(31) * 8) as u8
}

/// Y-down head/feet offset, matching EasyRPG `CalculateOffset`.
fn position_offset(position: u32, height: f32) -> f32 {
    match position {
        POSITION_UP => -height / 2.0,
        POSITION_DOWN => height / 2.0,
        _ => 0.0,
    }
}

/// Debug-only: F4 plays animation 1 ("Ron pusztakez") at screen centre so the
/// renderer can be verified on the map, but only while nothing else owns input.
fn debug_preview(
    keys: Res<ButtonInput<KeyCode>>,
    battle: Res<BattleActive>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    title: Res<TitleActive>,
    mut plays: MessageWriter<PlayAnimation>,
) {
    if !crate::debug::tools_enabled() || battle.0 || menu.0 || shop.0 || title.0 {
        return;
    }
    if keys.just_pressed(KeyCode::F4) {
        plays.write(PlayAnimation {
            slot: AnimationSlot::Map,
            map_target: None,
            anim_id: 1,
            targets: vec![AnimAnchor {
                pos: Vec2::ZERO,
                height: MAP_CHARACTER_HEIGHT,
            }],
            screen_center: MAP_SCREEN_CENTER,
            global: false,
            sound_only: false,
        });
    }
}

/// Convert world y-up positions to centre-relative RM2000 y-down pixels.
fn target_screen_offset(target: Vec2, camera: Vec2) -> Vec2 {
    Vec2::new(target.x - camera.x, camera.y - target.y)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod playback_tests;

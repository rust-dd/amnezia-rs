//! RM2000 effect cells, sounds and flashes on the shared logical animation clock.
//! Each map/battle slot owns one cast. Map casts retain their character target
//! and refresh their placement after movement and camera shake, without restarting.
//! Overlay positions are measured from screen centre with y growing downward;
//! the animation definition supplies screen scope and head/centre/feet offsets.

mod cells;
mod map;
mod playback;
mod render;
mod scene;
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
pub(crate) use playback::reset_transient;
pub(crate) use render::flash_power_level;
pub use render::{overlay_layer, overlay_translation};

/// Seconds each animation *data* frame is shown. RM2000 (and EasyRPG) advances
/// the animation once per 60 fps game-frame and shows each data frame for two of
/// them (`battle_animation.cpp`: `num_frames = GetRealFrames() * 2`,
/// `GetRealFrame() = frame / 2`), so a data frame lasts `2/60 = 1/30 s`. The
/// data cadence is independent of the flash envelope.
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

/// Where a screen-scope animation centres its cells on the map: EasyRPG
/// `BattleAnimationMap` draws a screen animation at the screen centre, which in
/// our centre-origin overlay is the origin.
const MAP_SCREEN_CENTER: Vec2 = Vec2::ZERO;

/// One target an animation plays on: `pos` its RM2000 screen offset from the
/// screen centre (y downward) — the point the target flash and the battler match
/// against — and `height` the target sprite's pixel height, from which the
/// [`AnimationDef::position`] anchor derives its vertical offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimAnchor {
    pub pos: Vec2,
    pub height: f32,
}

/// Play animation `anim_id` on `targets`. One cast is a single [`PlayAnimation`]:
/// its sound-effect timeline fires once, and its cells draw at each target's
/// [`AnimAnchor`] (a single-target scope-0 animation) — or, for a screen-scope
/// animation, once at `screen_center` (RM2000 screen offset from centre, y
/// downward). Target flashes still fire at every target regardless of scope,
/// matching EasyRPG (`battle_animation.cpp`). `global` (RM2000 map opcode 11210's
/// global flag) tiles the cells 3×3 across the screen instead, overriding scope.
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

/// A request to flash-tint a target battler sprite as an animation's target
/// flash fires: `pos` is the battler's RM2000 screen offset from centre (the same
/// point the animation plays on), `rgb` the flash colour in bytes, and `power` the
/// RM2000 flash strength (`0..=31`) that drives the stepped [`render::flash_envelope`].
/// `battle::scene` finds the battler at `pos` and drives its sprite colour over
/// the ~11-game-frame envelope. RM2000 front view draws no party sprites, so a
/// party-area target flash matches no battler and shows nothing.
#[derive(Message)]
pub struct BattlerFlash {
    pub pos: Vec2,
    pub rgb: [u8; 3],
    pub power: u32,
    pub age: u32,
}

/// The character a [`ShowMapAnimation`] plays on, already resolved from the
/// RM2000 char-ref by the interpreter: the hero, or a map event by id (a
/// this-event ref is resolved to a concrete id before it reaches here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimTarget {
    Hero,
    Event(u32),
}

/// The interpreter's request to play battle animation `anim_id` on a map
/// character (RM2000 `ShowBattleAnimation`, opcode 11210). The interpreter only
/// decodes the id, resolves the char-ref, and reads the `global` flag (params[3]);
/// [`resolve_map_animation`] looks the target's world position up against the main
/// camera and emits the screen-space [`PlayAnimation`], so the interpreter itself
/// needs no camera/transform queries. `global` tiles the animation 3×3 across the
/// screen (EasyRPG `BattleAnimationMap::DrawGlobal`).
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
        app.add_message::<PlayAnimation>()
            .add_message::<ShowMapAnimation>()
            .add_message::<crate::world::MapChanged>()
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
                    (
                        playback::clear_map_animations,
                        fade_flashes,
                        step_animations,
                        track_active_animations,
                    )
                        .chain()
                        .in_set(AnimationSet::Advance)
                        .after(crate::teleport::MapTransfer),
                    (
                        resolve_map_animation,
                        debug_preview,
                        start_animations,
                        track_active_animations,
                    )
                        .chain()
                        .in_set(AnimationSet::Start)
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

/// Spawn the fixed effect-overlay camera: a 2D camera at the origin with the same
/// fixed 320×240 scaling as the main camera, render `order` 2, and no clear. It
/// draws only [`render::OVERLAY_LAYER`], so it paints the effect sprites (and the
/// battle backdrop/battlers, which share the layer) over everything below it — the
/// toned world and the front camera's pictures — while the order-3 UI camera
/// composites every game window above it. It deliberately does not follow
/// the hero, which is what makes [`PlayAnimation`]'s `(x, y)` pure screen-space.
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

/// The screen points an animation's cells draw at: a `global` animation tiles its
/// cells 3×3 across the screen around `screen_center` (EasyRPG
/// `BattleAnimationMap::DrawGlobal`), overriding scope; otherwise a screen-scope
/// animation draws its cells once at `screen_center`, and a single-target one once
/// per target, each shifted vertically by the [`AnimationDef::position`] anchor
/// over that target's height (see [`position_offset`]). The flash anchors stay at
/// the un-shifted target centres, so a target flash still lands on the battler.
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

/// Fire every timing that lands on `frame` (0-based; timings store 1-based frame
/// numbers): emit its sound effect **once** for the whole cast, then its flash —
/// a single full-screen quad for a screen flash, or one target tint per anchor.
/// Firing the SE once (not once per target) matches EasyRPG, where a cast is a
/// single `BattleAnimation` whose timeline runs once for all its battlers.
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

/// Emit the timing's sound effect at its own volume/tempo (RM2000 stores a full
/// `Sound` per timing, not just a name). An empty or `(OFF)` name is silent
/// ([`AudioRequest::se`] returns `None`); otherwise the `0..=100` volume and
/// percent tempo map onto the request's logarithmic gain and playback speed, the
/// same mapping a System SE uses.
fn emit_sound(audio: &mut MessageWriter<AudioRequest>, timing: &AnimationTimingDef) {
    if let Some(request) = AudioRequest::se(&timing.se_name, timing.se_volume, timing.se_tempo) {
        audio.write(request);
    }
}

/// Emit the timing's flash. A screen flash spawns a single full-screen decaying
/// quad on the overlay (RM2000's animation screen flash is a full-screen tint),
/// once for the cast. A target flash publishes a [`BattlerFlash`] per anchor for
/// `battle::scene` to tint each target battler sprite — never a drawn box.
/// `ScreenEffect::Flash` is deliberately unused here: it is a main-camera overlay
/// that would hide behind the order-1 overlay backdrop during battle; it stays
/// reserved for the interpreter's map `FlashScreen` opcode.
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

/// The RM2000 screen-space y-offset (down positive) the [`AnimationDef::position`]
/// anchor applies over a target of pixel `height`, matching EasyRPG
/// `battle_animation.cpp` `CalculateOffset`: feet/down (`2`) drops the effect by
/// `height / 2`, head/up (`0`) lifts it by `height / 2`, and centre (`1`, or any
/// other value) leaves it on the target centre.
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

/// The RM2000 screen offset (from centre, y-down) at which a character at world
/// `target` appears when the main camera is centred at `camera`. The world and
/// overlay cameras share the fixed 320×240 projection (1 unit = 1 px), so the
/// on-screen offset is `target - camera`; RM2000 measures y downward while the
/// world is y-up, so the y component is negated. [`PlayAnimation`] flips it back
/// to world `(x, -y)` on the origin-fixed overlay, landing the animation on the
/// target.
fn target_screen_offset(target: Vec2, camera: Vec2) -> Vec2 {
    Vec2::new(target.x - camera.x, camera.y - target.y)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod playback_tests;

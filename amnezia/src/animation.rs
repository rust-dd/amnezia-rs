//! The battle effect-animation player: it plays a converted RM2000 animation
//! (from `animations.ron`) as on-screen sprites. A [`PlayAnimation`] message
//! names an animation id and a screen position; this plugin spawns a
//! [`LiveAnimation`] that steps its frames at a fixed rate, drawing each frame's
//! cells (see [`render`]) and firing that frame's sound-effect and flash
//! timings. It owns the renderer plus the fixed overlay camera (render order 2)
//! the effects draw on, which the battle backdrop and battlers ([`crate::battle`])
//! share, so effects composite over the map and land on the battlers by
//! construction; `battle` emits a [`PlayAnimation`] per physical hit, and the
//! interpreter's [`ShowMapAnimation`] (opcode 11210) projects a target character
//! to its screen position and plays one on the map.
//!
//! Positions are RM2000 screen coordinates measured from the screen centre
//! (`0,0` = centre), y growing downward, matching the source data. `scope`
//! (single vs screen) and `position` (head/centre/feet anchor) are the caller's
//! concern: it resolves them into the `(x, y)` it passes here.

mod cells;
mod render;
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
use cells::CellRenderer;
use render::{fade_flashes, next_frame, spawn_screen_flash};

pub use render::{flash_envelope, overlay_layer, overlay_translation};

/// Seconds each animation *data* frame is shown. RM2000 (and EasyRPG) advances
/// the animation once per 60 fps game-frame and shows each data frame for two of
/// them (`battle_animation.cpp`: `num_frames = GetRealFrames() * 2`,
/// `GetRealFrame() = frame / 2`), so a data frame lasts `2/60 = 1/30 s`. The
/// data cadence is independent of the flash envelope.
pub const FRAME_SECS: f32 = 1.0 / 30.0;

/// Seconds one 60 fps game-frame lasts: half a data frame. The flash envelope
/// ([`render::flash_envelope`]) counts these game-frames — EasyRPG holds a flash
/// over its `UpdateFlashGeneric` window of eleven game-frames (`delta_frames <=
/// 10`) — so the window stays decoupled from [`FRAME_SECS`]: halving the per-frame
/// step for the 15→30 fps fix must not reshape it.
const GAME_FRAME_SECS: f32 = FRAME_SECS / 2.0;

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
    pub anim_id: u32,
    pub targets: Vec<AnimAnchor>,
    pub screen_center: Vec2,
    pub global: bool,
}

/// A request to flash-tint a target battler sprite as an animation's target
/// flash fires: `pos` is the battler's RM2000 screen offset from centre (the same
/// point the animation plays on), `rgb` the flash colour (0..1), and `power` the
/// RM2000 flash strength (`0..=31`) that drives the stepped [`flash_envelope`].
/// `battle::scene` finds the battler at `pos` and drives its sprite colour over
/// the ~11-game-frame envelope. RM2000 front view draws no party sprites, so a
/// party-area target flash matches no battler and shows nothing.
#[derive(Message)]
pub struct BattlerFlash {
    pub pos: Vec2,
    pub rgb: [f32; 3],
    pub power: u32,
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

/// How many effect animations are live this frame. `battle::resolve_tick` reads
/// it to hold resolution while a strike/cast animation plays out before the
/// damage lands (during a fight only battle animations play, so any live
/// animation is the attack animation). Kept in step by [`track_active_animations`].
#[derive(Resource, Default)]
pub struct ActiveAnimations(pub usize);

/// A playing animation: which library entry it is, the screen points its cells
/// draw at (`draw_anchors` — one per target, or a single centred point for a
/// screen-scope animation), the target centres its flashes fire at
/// (`flash_anchors` — every target, so the battler-match points survive the
/// position offset baked into `draw_anchors`), the current frame, the per-frame
/// timer, and the cell entities of that frame (kept so they can be despawned when
/// the frame advances).
#[derive(Component)]
struct LiveAnimation {
    index: usize,
    draw_anchors: Vec<Vec2>,
    flash_anchors: Vec<Vec2>,
    frame: usize,
    timer: Timer,
    cells: Vec<Entity>,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        cells::register(app);
        app.add_message::<PlayAnimation>()
            .add_message::<ShowMapAnimation>()
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
                        resolve_map_animation,
                        debug_preview,
                        start_animations,
                        step_animations.run_if(crate::transitions::scene_running),
                    )
                        .chain(),
                    fade_flashes,
                    track_active_animations,
                ),
            );
    }
}

/// Spawn the fixed effect-overlay camera: a 2D camera at the origin with the same
/// fixed 320×240 scaling as the main camera, render `order` 2, and no clear. It
/// draws only [`render::OVERLAY_LAYER`], so it paints the effect sprites (and the
/// battle backdrop/battlers, which share the layer) over everything below it — the
/// toned world and the front camera's pictures and UI — while the order-3 HUD
/// camera composites the battle windows above it. It deliberately does not follow
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

/// Spawn a [`LiveAnimation`] for each [`PlayAnimation`], drawing its first frame
/// and firing that frame's timings immediately.
fn start_animations(
    mut commands: Commands,
    mut renderer: CellRenderer,
    library: Res<AnimationLibrary>,
    mut requests: MessageReader<PlayAnimation>,
    mut audio: MessageWriter<AudioRequest>,
    mut battler_flash: MessageWriter<BattlerFlash>,
) {
    for request in requests.read() {
        let Some(index) = library.0.iter().position(|a| a.id == request.anim_id) else {
            continue;
        };
        let def = &library.0[index];
        if def.frames.is_empty() {
            continue;
        }
        let draw_anchors =
            draw_anchors(def, &request.targets, request.screen_center, request.global);
        let flash_anchors: Vec<Vec2> = request.targets.iter().map(|t| t.pos).collect();
        let cells = spawn_cells_at(&mut commands, &mut renderer, def, 0, &draw_anchors);
        fire_timings(
            &mut commands,
            &mut audio,
            &mut battler_flash,
            def,
            0,
            &flash_anchors,
        );
        commands.spawn(LiveAnimation {
            index,
            draw_anchors,
            flash_anchors,
            frame: 0,
            timer: Timer::from_seconds(FRAME_SECS, TimerMode::Repeating),
            cells,
        });
    }
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

/// Spawn frame `frame`'s cells at every anchor in `bases`, returning all the cell
/// entities together so they despawn as one when the frame advances.
fn spawn_cells_at(
    commands: &mut Commands,
    renderer: &mut CellRenderer,
    def: &AnimationDef,
    frame: usize,
    bases: &[Vec2],
) -> Vec<Entity> {
    let mut cells = Vec::new();
    for &base in bases {
        cells.extend(renderer.spawn_frame(commands, def, frame, base));
    }
    cells
}

/// Advance every live animation one frame per timer tick: despawn the old
/// frame's cells, then draw the next frame and fire its timings, or despawn the
/// animation once its last frame has played.
fn step_animations(
    time: Res<Time>,
    mut commands: Commands,
    mut renderer: CellRenderer,
    library: Res<AnimationLibrary>,
    mut audio: MessageWriter<AudioRequest>,
    mut battler_flash: MessageWriter<BattlerFlash>,
    mut animations: Query<(Entity, &mut LiveAnimation)>,
) {
    for (entity, mut anim) in &mut animations {
        if !anim.timer.tick(time.delta()).just_finished() {
            continue;
        }
        for cell in anim.cells.drain(..) {
            commands.entity(cell).despawn();
        }
        let def = &library.0[anim.index];
        match next_frame(anim.frame, def.frames.len()) {
            Some(frame) => {
                anim.frame = frame;
                anim.cells =
                    spawn_cells_at(&mut commands, &mut renderer, def, frame, &anim.draw_anchors);
                fire_timings(
                    &mut commands,
                    &mut audio,
                    &mut battler_flash,
                    def,
                    frame,
                    &anim.flash_anchors,
                );
            }
            None => commands.entity(entity).despawn(),
        }
    }
}

/// Keep [`ActiveAnimations`] in step with the live [`LiveAnimation`] count, so the
/// battle resolver can tell whether a strike/cast animation is still playing. A
/// just-finished animation's despawn applies a frame later, so the count trails
/// its end by a frame — harmless for the hold, which only needs "has it gone".
fn track_active_animations(
    animations: Query<(), With<LiveAnimation>>,
    mut active: ResMut<ActiveAnimations>,
) {
    let count = animations.iter().count();
    if active.0 != count {
        active.0 = count;
    }
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
) {
    for timing in def.timings.iter().filter(|t| t.frame as usize == frame + 1) {
        emit_sound(audio, timing);
        emit_flash(commands, battler_flash, timing, flash_anchors);
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
) {
    let rgb = [
        flash_channel(timing.flash_red),
        flash_channel(timing.flash_green),
        flash_channel(timing.flash_blue),
    ];
    let power = timing.flash_power;
    match timing.flash_scope {
        FLASH_SCOPE_SCREEN => spawn_screen_flash(commands, rgb, power),
        FLASH_SCOPE_TARGET => {
            for &pos in flash_anchors {
                battler_flash.write(BattlerFlash { pos, rgb, power });
            }
        }
        _ => {}
    }
}

/// An RM2000 flash channel (0..=31) as a 0..1 component.
fn flash_channel(value: u32) -> f32 {
    (value as f32 / 31.0).clamp(0.0, 1.0)
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
            anim_id: 1,
            targets: vec![AnimAnchor {
                pos: Vec2::ZERO,
                height: MAP_CHARACTER_HEIGHT,
            }],
            screen_center: MAP_SCREEN_CENTER,
            global: false,
        });
    }
}

/// Resolve each interpreter [`ShowMapAnimation`] to a screen-space
/// [`PlayAnimation`]: find the target character's world position (the hero, or a
/// map event by id) and the main camera's, project the target onto the fixed
/// overlay with [`target_screen_offset`], and emit the play request. A target with
/// no live entity on the current map (e.g. an event id not on this map) is
/// dropped. This is where the camera/transform lookup lives, keeping the
/// interpreter's own system params clear of it.
fn resolve_map_animation(
    mut requests: MessageReader<ShowMapAnimation>,
    mut plays: MessageWriter<PlayAnimation>,
    camera: Query<&Transform, With<MainCamera>>,
    hero: Query<&Transform, With<Player>>,
    events: Query<(&EventSprite, &Transform)>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    let camera_pos = camera.translation.truncate();
    for request in requests.read() {
        let target_pos = match request.target {
            AnimTarget::Hero => hero.single().ok().map(|t| t.translation.truncate()),
            AnimTarget::Event(id) => events
                .iter()
                .find(|(sprite, _)| sprite.id == id)
                .map(|(_, transform)| transform.translation.truncate()),
        };
        let Some(target_pos) = target_pos else {
            continue;
        };
        let offset = target_screen_offset(target_pos, camera_pos);
        plays.write(PlayAnimation {
            anim_id: request.anim_id,
            targets: vec![AnimAnchor {
                pos: offset,
                height: MAP_CHARACTER_HEIGHT,
            }],
            screen_center: MAP_SCREEN_CENTER,
            global: request.global,
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

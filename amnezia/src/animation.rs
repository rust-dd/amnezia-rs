//! The battle effect-animation player: it plays a converted RM2000 animation
//! (from `animations.ron`) as on-screen sprites. A [`PlayAnimation`] message
//! names an animation id and a screen position; this plugin spawns a
//! [`LiveAnimation`] that steps its frames at a fixed rate, drawing each frame's
//! cells (see [`render`]) and firing that frame's sound-effect and flash
//! timings. It owns only the renderer; a follow-up wires it into per-hit battle
//! and the interpreter.
//!
//! Positions are RM2000 screen coordinates measured from the screen centre
//! (`0,0` = centre), y growing downward, matching the source data. `scope`
//! (single vs screen) and `position` (head/centre/feet anchor) are the caller's
//! concern: it resolves them into the `(x, y)` it passes here.

mod render;

use crate::assets::{asset_root, load_ron};
use crate::audio::AudioRequest;
use crate::battle::BattleActive;
use crate::menu::MenuOpen;
use crate::screenfx::{ScreenEffect, ScreenShakeSet};
use crate::shop::ShopOpen;
use crate::title::TitleActive;
use amnezia_data::{AnimationDef, AnimationTimingDef};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use render::{fade_flashes, next_frame, pin_to_screen, spawn_frame_cells, spawn_target_flash};

/// Seconds each animation frame is shown (RM2000 runs animations at ~15 fps).
pub const FRAME_SECS: f32 = 1.0 / 15.0;

/// A target flash decays over roughly three frames.
const FLASH_SECS: f32 = 3.0 * FRAME_SECS;

/// The RM2000 sentinel sound name meaning "no sound"; skipped like an empty name.
const SE_OFF: &str = "(OFF)";

/// The flash channel `flash_scope` value that flashes the whole screen; `1`
/// flashes the target, `0` nothing.
const FLASH_SCOPE_SCREEN: u32 = 2;
const FLASH_SCOPE_TARGET: u32 = 1;

/// Play animation `anim_id` centred at RM2000 screen offset `(x, y)` from the
/// screen centre (y downward).
#[derive(Message)]
pub struct PlayAnimation {
    pub anim_id: u32,
    pub x: f32,
    pub y: f32,
}

/// Every converted battle animation, loaded once at plugin build.
#[derive(Resource)]
pub struct AnimationLibrary(pub Vec<AnimationDef>);

/// A playing animation: which library entry it is, its screen base point, the
/// current frame, the per-frame timer, and the cell entities of that frame (kept
/// so they can be despawned when the frame advances).
#[derive(Component)]
struct LiveAnimation {
    index: usize,
    base: Vec2,
    frame: usize,
    timer: Timer,
    cells: Vec<Entity>,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayAnimation>()
            .insert_resource(AnimationLibrary(load_ron(&format!(
                "{}/animations.ron",
                asset_root()
            ))))
            .add_systems(
                Update,
                (
                    (debug_preview, start_animations, step_animations).chain(),
                    fade_flashes,
                ),
            )
            .add_systems(
                PostUpdate,
                pin_to_screen
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// Spawn a [`LiveAnimation`] for each [`PlayAnimation`], drawing its first frame
/// and firing that frame's timings immediately.
fn start_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    library: Res<AnimationLibrary>,
    mut requests: MessageReader<PlayAnimation>,
    mut audio: MessageWriter<AudioRequest>,
    mut screen: MessageWriter<ScreenEffect>,
) {
    for request in requests.read() {
        let Some(index) = library.0.iter().position(|a| a.id == request.anim_id) else {
            continue;
        };
        let def = &library.0[index];
        if def.frames.is_empty() {
            continue;
        }
        let base = Vec2::new(request.x, request.y);
        let cells = spawn_frame_cells(&mut commands, &asset_server, def, 0, base);
        fire_timings(&mut commands, &mut audio, &mut screen, def, 0, base);
        commands.spawn(LiveAnimation {
            index,
            base,
            frame: 0,
            timer: Timer::from_seconds(FRAME_SECS, TimerMode::Repeating),
            cells,
        });
    }
}

/// Advance every live animation one frame per timer tick: despawn the old
/// frame's cells, then draw the next frame and fire its timings, or despawn the
/// animation once its last frame has played.
fn step_animations(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    library: Res<AnimationLibrary>,
    mut audio: MessageWriter<AudioRequest>,
    mut screen: MessageWriter<ScreenEffect>,
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
                anim.cells = spawn_frame_cells(&mut commands, &asset_server, def, frame, anim.base);
                fire_timings(
                    &mut commands,
                    &mut audio,
                    &mut screen,
                    def,
                    frame,
                    anim.base,
                );
            }
            None => commands.entity(entity).despawn(),
        }
    }
}

/// Fire every timing that lands on `frame` (0-based; timings store 1-based frame
/// numbers): emit its sound effect and its flash.
fn fire_timings(
    commands: &mut Commands,
    audio: &mut MessageWriter<AudioRequest>,
    screen: &mut MessageWriter<ScreenEffect>,
    def: &AnimationDef,
    frame: usize,
    base: Vec2,
) {
    for timing in def.timings.iter().filter(|t| t.frame as usize == frame + 1) {
        emit_sound(audio, timing);
        emit_flash(commands, screen, timing, base);
    }
}

/// Emit the timing's sound effect. An empty or `(OFF)` name is silent; the audio
/// plugin resolves the name to a `wav` (or no-ops if none exists).
fn emit_sound(audio: &mut MessageWriter<AudioRequest>, timing: &AnimationTimingDef) {
    if timing.se_name.is_empty() || timing.se_name == SE_OFF {
        return;
    }
    audio.write(AudioRequest::Sound {
        name: timing.se_name.clone(),
        volume: 1.0,
        speed: 1.0,
    });
}

/// Emit the timing's flash: a screen flash reuses the [`ScreenEffect`] overlay;
/// a target flash spawns a local decaying quad at the animation base.
fn emit_flash(
    commands: &mut Commands,
    screen: &mut MessageWriter<ScreenEffect>,
    timing: &AnimationTimingDef,
    base: Vec2,
) {
    let rgb = [
        flash_channel(timing.flash_red),
        flash_channel(timing.flash_green),
        flash_channel(timing.flash_blue),
    ];
    match timing.flash_scope {
        FLASH_SCOPE_SCREEN => {
            screen.write(ScreenEffect::Flash {
                r: timing.flash_red as i32,
                g: timing.flash_green as i32,
                b: timing.flash_blue as i32,
                intensity: timing.flash_power as i32,
                secs: FLASH_SECS,
            });
        }
        FLASH_SCOPE_TARGET => {
            spawn_target_flash(
                commands,
                base,
                rgb,
                flash_channel(timing.flash_power),
                FLASH_SECS,
            );
        }
        _ => {}
    }
}

/// An RM2000 flash channel (0..=31) as a 0..1 component.
fn flash_channel(value: u32) -> f32 {
    (value as f32 / 31.0).clamp(0.0, 1.0)
}

/// Debug-only: F7 plays animation 1 ("Ron pusztakez") at screen centre so the
/// renderer can be verified on the map, but only while nothing else owns input.
fn debug_preview(
    keys: Res<ButtonInput<KeyCode>>,
    battle: Res<BattleActive>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    title: Res<TitleActive>,
    mut plays: MessageWriter<PlayAnimation>,
) {
    if battle.0 || menu.0 || shop.0 || title.0 {
        return;
    }
    if keys.just_pressed(KeyCode::F7) {
        plays.write(PlayAnimation {
            anim_id: 1,
            x: 0.0,
            y: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_channel_normalises_and_clamps_the_0_31_scale() {
        assert_eq!(flash_channel(0), 0.0);
        assert_eq!(flash_channel(31), 1.0);
        assert_eq!(flash_channel(62), 1.0);
        assert!((flash_channel(15) - 15.0 / 31.0).abs() < 1e-6);
    }
}

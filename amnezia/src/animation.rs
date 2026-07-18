//! The battle effect-animation player: it plays a converted RM2000 animation
//! (from `animations.ron`) as on-screen sprites. A [`PlayAnimation`] message
//! names an animation id and a screen position; this plugin spawns a
//! [`LiveAnimation`] that steps its frames at a fixed rate, drawing each frame's
//! cells (see [`render`]) and firing that frame's sound-effect and flash
//! timings. It owns the renderer plus the fixed overlay camera the effects draw
//! on, so they composite over the battle UI; `battle` emits a [`PlayAnimation`]
//! per physical hit, and the interpreter's [`ShowMapAnimation`] (opcode 11210)
//! projects a target character to its screen position and plays one on the map.
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
use crate::player::Player;
use crate::screenfx::ScreenEffect;
use crate::shop::ShopOpen;
use crate::title::TitleActive;
use crate::world::{EventSprite, MainCamera};
use amnezia_data::{AnimationDef, AnimationTimingDef};
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use render::{fade_flashes, next_frame, overlay_layer, spawn_frame_cells, spawn_target_flash};

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
/// decodes the id and resolves the char-ref; [`resolve_map_animation`] looks the
/// target's world position up against the main camera and emits the screen-space
/// [`PlayAnimation`], so the interpreter itself needs no camera/transform queries.
#[derive(Message)]
pub struct ShowMapAnimation {
    pub anim_id: u32,
    pub target: AnimTarget,
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
            .add_message::<ShowMapAnimation>()
            .insert_resource(AnimationLibrary(load_ron(&format!(
                "{}/animations.ron",
                asset_root()
            ))))
            .add_systems(Startup, spawn_overlay_camera)
            .add_systems(
                Update,
                (
                    (
                        resolve_map_animation,
                        debug_preview,
                        start_animations,
                        step_animations,
                    )
                        .chain(),
                    fade_flashes,
                ),
            );
    }
}

/// Spawn the fixed effect-overlay camera: a second 2D camera at the origin with
/// the same fixed 320×240 scaling as the main camera, a higher render `order`,
/// and no clear. It draws only [`render::OVERLAY_LAYER`], so it paints the
/// effect sprites over everything the main camera already rendered — the world,
/// the pictures, and the z-118 battle UI. It deliberately does not follow the
/// hero, which is what makes [`PlayAnimation`]'s `(x, y)` pure screen-space.
fn spawn_overlay_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
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
            x: offset.x,
            y: offset.y,
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
mod tests {
    use super::*;

    #[test]
    fn flash_channel_normalises_and_clamps_the_0_31_scale() {
        assert_eq!(flash_channel(0), 0.0);
        assert_eq!(flash_channel(31), 1.0);
        assert_eq!(flash_channel(62), 1.0);
        assert!((flash_channel(15) - 15.0 / 31.0).abs() < 1e-6);
    }

    #[test]
    fn target_screen_offset_centres_and_negates_y() {
        // A target on the camera centre maps to the screen centre.
        assert_eq!(
            target_screen_offset(Vec2::new(50.0, -20.0), Vec2::new(50.0, -20.0)),
            Vec2::ZERO
        );
        // A target +32 right and +16 *up* in world (y-up) reads as RM2000
        // `(32, -16)` — right, and above centre (RM2000 y grows downward).
        assert_eq!(
            target_screen_offset(Vec2::new(32.0, 16.0), Vec2::ZERO),
            Vec2::new(32.0, -16.0)
        );
        // A target below the camera has positive RM2000 y.
        assert_eq!(
            target_screen_offset(Vec2::new(0.0, -40.0), Vec2::ZERO),
            Vec2::new(0.0, 40.0)
        );
    }

    #[test]
    fn resolver_projects_hero_and_event_onto_screen() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ShowMapAnimation>();
        app.add_message::<PlayAnimation>();
        app.add_systems(Update, resolve_map_animation);

        // Main camera at world (100, 50). The hero sits 32 right / 16 up from it;
        // event 7 sits 10 left / 20 down.
        app.world_mut()
            .spawn((MainCamera, Transform::from_xyz(100.0, 50.0, 0.0)));
        app.world_mut().spawn((
            Player {
                tile_x: 0,
                tile_y: 0,
                dir: 0,
                frame: 0,
                charset: String::new(),
                index: 0,
            },
            Transform::from_xyz(132.0, 66.0, 3.0),
        ));
        app.world_mut().spawn((
            EventSprite {
                id: 7,
                tile_x: 0,
                tile_y: 0,
                dir: 0,
                frame: 0,
                charset: String::new(),
                index: 0,
            },
            Transform::from_xyz(90.0, 30.0, 3.0),
        ));

        app.world_mut().write_message(ShowMapAnimation {
            anim_id: 62,
            target: AnimTarget::Hero,
        });
        app.world_mut().write_message(ShowMapAnimation {
            anim_id: 63,
            target: AnimTarget::Event(7),
        });
        app.update();

        let messages = app.world().resource::<Messages<PlayAnimation>>();
        let mut cursor = messages.get_cursor();
        let plays: Vec<(u32, f32, f32)> = cursor
            .read(messages)
            .map(|p| (p.anim_id, p.x, p.y))
            .collect();
        // Hero: (132-100, 50-66) = (32, -16). Event 7: (90-100, 50-30) = (-10, 20).
        assert_eq!(plays, vec![(62, 32.0, -16.0), (63, -10.0, 20.0)]);
    }
}

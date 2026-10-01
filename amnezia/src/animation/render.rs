//! Shared overlay geometry and the original screen-flash envelope.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

pub(super) mod saved;

/// Isolates effects and battlers from the world camera for screen-space compositing.
pub(super) const OVERLAY_LAYER: usize = 1;

pub fn overlay_layer() -> RenderLayers {
    RenderLayers::layer(OVERLAY_LAYER)
}

/// The RM2000 screen extent, matching the fixed overlay projection.
const SCREEN_W: f32 = 320.0;
const SCREEN_H: f32 = 240.0;

/// Convert centre-relative y-down pixels to the fixed overlay's y-up world space.
pub fn overlay_translation(pos: Vec2, z: f32) -> Vec3 {
    Vec3::new(pos.x, -pos.y, z)
}

#[derive(Clone, Copy, Default)]
pub(super) struct FlashStamp {
    pub age: u32,
    pub frame: u32,
}

/// Screen flash age is measured in logical frames, including skipped render frames.
#[derive(Component)]
pub(crate) struct FlashQuad {
    elapsed: u32,
    last: u32,
    pub power: u32,
    rgb: [u8; 3],
    /// Live casts refresh the channel every update; legacy snapshots can fade independently.
    driven: bool,
}

/// EasyRPG `UpdateFlashGeneric` keeps a flash lit for ages 0–10 inclusive.
pub(super) const FLASH_LAST_FRAME: u32 = 10;

/// EasyRPG `CalculateFlashPower`: integer 0–31 strength, stepping every two ticks
/// rather than fading linearly. Full power holds a three-tick plateau.
pub(crate) fn flash_power_level(frames: u32, power: u32) -> u32 {
    let f = 7 - (frames as i32 + 1) / 2;
    (f * power as i32 / 6).clamp(0, 31) as u32
}

/// The original 5-bit strength expands to a byte with ×8, including at its peak.
pub fn flash_envelope(frame: u32, power: u32) -> Option<f32> {
    (frame <= FLASH_LAST_FRAME).then(|| (flash_power_level(frame, power) * 8) as f32 / 255.0)
}

/// EasyRPG `DrawGlobal` tiles 3×3 at screen-sized intervals; symmetry makes y-flipping unnecessary.
pub(super) fn global_anchors(center: Vec2) -> Vec<Vec2> {
    let mut anchors = Vec::with_capacity(9);
    for row in -1..=1 {
        for col in -1..=1 {
            anchors.push(Vec2::new(
                center.x + col as f32 * SCREEN_W,
                center.y + row as f32 * SCREEN_H,
            ));
        }
    }
    anchors
}

pub(super) fn spawn_screen_flash(
    commands: &mut Commands,
    rgb: [u8; 3],
    power: u32,
    stamp: FlashStamp,
) {
    write_screen_flash(commands, rgb, power, stamp, false);
}

pub(super) fn write_screen_flash(
    commands: &mut Commands,
    rgb: [u8; 3],
    power: u32,
    stamp: FlashStamp,
    driven: bool,
) {
    commands.queue(move |world: &mut World| {
        crate::screenfx::flash::channel::cancel_event_flash(world);
        let entity = crate::screenfx::flash::channel::overlay(world);
        let Some(alpha) = flash_envelope(stamp.age, power) else {
            clear_screen_flash(world);
            return;
        };
        world.entity_mut(entity).insert((
            Sprite::from_color(flash_color(rgb, alpha), Vec2::new(SCREEN_W, SCREEN_H)),
            FlashQuad {
                elapsed: stamp.age,
                last: stamp.frame,
                power,
                rgb,
                driven,
            },
        ));
    });
}

pub(crate) fn clear_screen_flash(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, With<FlashQuad>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.entity_mut(entity).remove::<FlashQuad>();
        if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
            sprite.color = Color::NONE;
        }
    }
}

pub(super) fn screen_timing(
    def: &amnezia_data::AnimationDef,
    tick: u32,
) -> Option<(&amnezia_data::AnimationTimingDef, u32)> {
    def.timings
        .iter()
        .enumerate()
        .filter(|(_, timing)| {
            timing.flash_scope == super::FLASH_SCOPE_SCREEN
                && timing.frame > 0
                && (timing.frame - 1) * 2 <= tick
        })
        .max_by_key(|(index, timing)| (timing.frame, *index))
        .map(|(_, timing)| (timing, tick - (timing.frame - 1) * 2))
}

fn flash_color(rgb: [u8; 3], alpha: f32) -> Color {
    let [r, g, b] = rgb.map(|v| v as f32 / 255.0);
    Color::srgba(r, g, b, alpha)
}

pub(super) fn screen_color(def: &amnezia_data::AnimationDef, tick: u32, duration: u32) -> [u8; 4] {
    if tick < duration
        && let Some((timing, age)) = screen_timing(def, tick)
        && age <= FLASH_LAST_FRAME
    {
        [
            super::flash_channel(timing.flash_red),
            super::flash_channel(timing.flash_green),
            super::flash_channel(timing.flash_blue),
            (flash_power_level(age, timing.flash_power) * 8) as u8,
        ]
    } else {
        [0; 4]
    }
}

/// Expire the previous cast update or advance a legacy standalone envelope.
pub(super) fn fade_flashes(
    transition: crate::transitions::TransitionPause,
    scene: super::scene::Scenes,
    frames: Res<crate::timing::GameFrames>,
    step: Option<Res<crate::timing::logical::Step>>,
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut FlashQuad, &mut Sprite)>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        let delta = if step.as_ref().is_some_and(|step| step.callback) {
            1
        } else {
            frames.frame.wrapping_sub(flash.last)
        };
        flash.last = frames.frame;
        if transition.paused() || scene.frozen() {
            continue;
        }
        if delta == 0 {
            continue;
        }
        if flash.driven {
            sprite.color = Color::NONE;
            commands.entity(entity).remove::<FlashQuad>();
            continue;
        }
        flash.elapsed = flash.elapsed.saturating_add(delta);
        match flash_envelope(flash.elapsed, flash.power) {
            Some(alpha) => {
                sprite.color = flash_color(flash.rgb, alpha);
            }
            None => {
                sprite.color = Color::NONE;
                commands.entity(entity).remove::<FlashQuad>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_translation_flips_y_and_keeps_x_and_z() {
        assert_eq!(
            overlay_translation(Vec2::new(0.0, 0.0), 510.0),
            Vec3::new(0.0, 0.0, 510.0)
        );
        assert_eq!(
            overlay_translation(Vec2::new(40.0, -20.0), 300.0),
            Vec3::new(40.0, 20.0, 300.0)
        );
    }

    #[test]
    fn spawn_screen_flash_makes_a_fullscreen_overlay_quad() {
        use bevy::ecs::world::CommandQueue;
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_screen_flash(&mut commands, [248; 3], 31, FlashStamp::default());
        }
        queue.apply(&mut world);
        let mut quads = world.query::<(&FlashQuad, &Sprite)>();
        let (flash, sprite) = quads.single(&world).expect("a screen flash quad");
        assert_eq!(sprite.custom_size, Some(Vec2::new(SCREEN_W, SCREEN_H)));
        assert_eq!(flash.power, 31);
        assert_eq!(sprite.color.alpha(), 248.0 / 255.0);
    }

    #[test]
    fn flash_power_level_matches_easyrpg_calculate_flash_power() {
        // Measured RM2000 envelope for a full-strength (31) flash over game-frames
        // 0..=10: a 3-frame plateau at 31, then a step down every two frames.
        let expected = [31, 31, 31, 25, 25, 20, 20, 15, 15, 10, 10];
        for (frames, &level) in expected.iter().enumerate() {
            assert_eq!(
                flash_power_level(frames as u32, 31),
                level,
                "frame {frames}"
            );
        }
        // A weaker flash (power 18) scales the same curve and never clamps:
        // f = 7,6,6,5,5,... times 18/6 = 3.
        assert_eq!(flash_power_level(0, 18), 21);
        assert_eq!(flash_power_level(1, 18), 18);
        assert_eq!(flash_power_level(3, 18), 15);
        assert_eq!(flash_power_level(0, 0), 0);
    }

    #[test]
    fn flash_envelope_holds_then_steps_then_ends() {
        assert_eq!(flash_envelope(0, 31), Some(248.0 / 255.0));
        assert_eq!(flash_envelope(3, 31), Some(200.0 / 255.0));
        assert_eq!(flash_envelope(10, 31), Some(80.0 / 255.0));
        assert_eq!(flash_envelope(11, 31), None);
    }

    #[test]
    fn subsequent_screen_flashes_replace_the_previous_color_without_stacking() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world
            .run_system_once(|mut commands: Commands| {
                spawn_screen_flash(&mut commands, [248; 3], 31, FlashStamp::default());
                spawn_screen_flash(
                    &mut commands,
                    [248, 0, 0],
                    18,
                    FlashStamp { age: 3, frame: 4 },
                );
            })
            .unwrap();
        let (flash, sprite) = world
            .query::<(&FlashQuad, &Sprite)>()
            .single(&world)
            .unwrap();
        assert_eq!(flash.elapsed, 3);
        assert_eq!(
            sprite.color,
            Color::srgba(248.0 / 255.0, 0.0, 0.0, 120.0 / 255.0)
        );
        world
            .run_system_once(|mut commands: Commands| {
                spawn_screen_flash(
                    &mut commands,
                    [0, 248, 0],
                    31,
                    FlashStamp { age: 11, frame: 20 },
                );
            })
            .unwrap();
        assert_eq!(world.query::<&FlashQuad>().iter(&world).count(), 0);
    }

    #[test]
    fn screen_flash_pauses_and_counts_wrapped_logical_frames() {
        use crate::timing::GameFrames;
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameFrames>()
            .add_systems(Update, fade_flashes);
        app.world_mut().resource_mut::<GameFrames>().frame = u32::MAX - 1;
        app.world_mut()
            .run_system_once(|mut commands: Commands| {
                spawn_screen_flash(
                    &mut commands,
                    [248; 3],
                    31,
                    FlashStamp {
                        age: 0,
                        frame: u32::MAX - 1,
                    },
                );
            })
            .unwrap();
        app.update();
        app.world_mut().resource_mut::<GameFrames>().frame = 1;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&FlashQuad>()
                .single(app.world())
                .unwrap()
                .elapsed,
            3
        );
        let mut transition = crate::transitions::Transition::default();
        transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
        app.insert_resource(transition);
        app.world_mut().resource_mut::<GameFrames>().frame = 80;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&FlashQuad>()
                .single(app.world())
                .unwrap()
                .elapsed,
            3
        );
        app.world_mut()
            .remove_resource::<crate::transitions::Transition>();
        app.world_mut().resource_mut::<GameFrames>().frame = 83;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&FlashQuad>()
                .single(app.world())
                .unwrap()
                .elapsed,
            6
        );
        app.world_mut().resource_mut::<GameFrames>().frame = 91;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&FlashQuad>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

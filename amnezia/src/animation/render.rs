//! Shared overlay geometry and the original screen-flash envelope.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

/// The render layer the effect-overlay camera draws. Cells, flash quads, and the
/// battle scene carry it so only that fixed, higher-`order` camera renders them —
/// painting over the layer-0 world instead of hiding behind it.
pub(super) const OVERLAY_LAYER: usize = 1;

/// A fresh [`RenderLayers`] on [`OVERLAY_LAYER`] (the type isn't `Copy`, so each
/// spawned sprite and the overlay camera take their own).
pub fn overlay_layer() -> RenderLayers {
    RenderLayers::layer(OVERLAY_LAYER)
}

/// The RM2000 screen extent, matching the fixed overlay projection.
const SCREEN_W: f32 = 320.0;
const SCREEN_H: f32 = 240.0;

/// The screen plane is below animation cells, which apply the flash to their
/// own pixels before opacity blending.
const SCREEN_FLASH_Z: f32 = 300.0;

/// The world translation of an overlay sprite at RM2000 screen offset `pos` from
/// centre (y downward) with depth `z`. The overlay camera sits at the origin, so
/// this is pure screen-space: `x` unchanged, `y` flipped for world y-up.
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
pub(super) struct FlashQuad {
    elapsed: u32,
    last: u32,
    pub power: u32,
    rgb: [u8; 3],
}

/// The last 60 fps game-frame a flash is lit: EasyRPG shows it while
/// `delta_frames <= 10` (`battle_animation.cpp` `UpdateFlashGeneric`), i.e.
/// game-frames `0..=10`, an 11-frame window.
pub(super) const FLASH_LAST_FRAME: u32 = 10;

/// EasyRPG `CalculateFlashPower` (`battle_animation.cpp`): the flash level
/// (`0..=31`) on game-frame `frames` after a flash of strength `power` (`0..=31`)
/// fires — `f = 7 - (frames + 1) / 2`, `level = min(f * power / 6, 31)`. The curve
/// is a plateau-then-step, not a linear fade: it holds near the peak for the first
/// ~3 frames (`level` pinned at 31 for a full-strength flash) then steps down
/// every two frames. Integer arithmetic throughout, matching the measured RM2000
/// values.
pub(crate) fn flash_power_level(frames: u32, power: u32) -> u32 {
    let f = 7 - (frames as i32 + 1) / 2;
    (f * power as i32 / 6).clamp(0, 31) as u32
}

/// The original 5-bit strength expands to a byte with ×8, including at its peak.
pub fn flash_envelope(frame: u32, power: u32) -> Option<f32> {
    (frame <= FLASH_LAST_FRAME).then(|| (flash_power_level(frame, power) * 8) as f32 / 255.0)
}

/// The nine draw points of a `global` map animation (RM2000 opcode 11210's global
/// flag): the animation tiled 3×3 around `center`, each copy one screen-width
/// across and one screen-height down from the last, matching EasyRPG
/// `BattleAnimationMap::DrawGlobal` (which draws over the screen-effects rect for
/// `x, y ∈ {-1, 0, 1}`). The tiling is symmetric, so the RM2000 y-down convention
/// need not be flipped here.
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
    // Game_Screen has one flash channel; later timings replace it, never stack.
    commands.queue(move |world: &mut World| {
        let existing = world
            .query_filtered::<Entity, With<FlashQuad>>()
            .iter(world)
            .next();
        let Some(alpha) = flash_envelope(stamp.age, power) else {
            if let Some(entity) = existing {
                world.despawn(entity);
            }
            return;
        };
        let components = (
            Sprite::from_color(flash_color(rgb, alpha), Vec2::new(SCREEN_W, SCREEN_H)),
            Transform::from_translation(overlay_translation(Vec2::ZERO, SCREEN_FLASH_Z)),
            overlay_layer(),
            FlashQuad {
                elapsed: stamp.age,
                last: stamp.frame,
                power,
                rgb,
            },
        );
        if let Some(entity) = existing {
            world.entity_mut(entity).insert(components);
        } else {
            world.spawn(components);
        }
    });
}

fn flash_color(rgb: [u8; 3], alpha: f32) -> Color {
    let [r, g, b] = rgb.map(|v| v as f32 / 255.0);
    Color::srgba(r, g, b, alpha)
}

/// Step each live flash quad's alpha along the RM2000 [`flash_envelope`],
/// despawning it once the ~11-game-frame window has passed.
pub(super) fn fade_flashes(
    transition: crate::transitions::TransitionPause,
    scene: super::scene::Scenes,
    frames: Res<crate::timing::GameFrames>,
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut FlashQuad, &mut Sprite)>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        let delta = frames.frame.wrapping_sub(flash.last);
        flash.last = frames.frame;
        if transition.paused() || scene.paused() {
            continue;
        }
        flash.elapsed = flash.elapsed.saturating_add(delta);
        match flash_envelope(flash.elapsed, flash.power) {
            Some(alpha) => {
                sprite.color = flash_color(flash.rgb, alpha);
            }
            None => {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_translation_flips_y_and_keeps_x_and_z() {
        // On the origin-fixed overlay camera an RM2000 offset (x, y-down) is
        // world (x, -y); z passes through unchanged.
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
        // A zero-strength flash stays dark.
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

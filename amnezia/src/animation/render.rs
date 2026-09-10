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

/// World z of a full-screen screen-flash quad: over the backdrop and battlers but
/// under the cells, so the effect that triggered the flash still paints over it.
const SCREEN_FLASH_Z: f32 = 300.0;

/// The world translation of an overlay sprite at RM2000 screen offset `pos` from
/// centre (y downward) with depth `z`. The overlay camera sits at the origin, so
/// this is pure screen-space: `x` unchanged, `y` flipped for world y-up.
pub fn overlay_translation(pos: Vec2, z: f32) -> Vec3 {
    Vec3::new(pos.x, -pos.y, z)
}

/// A flash quad following the RM2000 stepped envelope: `rgb` its colour (0..1),
/// `power` the RM2000 flash strength (`0..=31`), and `elapsed` the time since it
/// fired. Its alpha is [`flash_envelope`] of those, not a linear fade.
#[derive(Component)]
pub(super) struct FlashQuad {
    pub elapsed: f32,
    pub power: u32,
    pub rgb: [f32; 3],
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

/// The flash tint strength (0..1) at `elapsed` seconds into a flash of RM2000
/// strength `power` (`0..=31`), or `None` once the ~11-game-frame window has
/// passed (the flash is spent). Shared by the screen-flash quad and the battler
/// target tint so both follow the same stepped [`flash_power_level`] envelope; the
/// `0..=31` level normalises by 31 onto the 0..1 scale the sprite alpha and
/// `blend` use.
pub fn flash_envelope(elapsed: f32, power: u32) -> Option<f32> {
    let frame = (elapsed / super::GAME_FRAME_SECS) as u32;
    (frame <= FLASH_LAST_FRAME).then(|| flash_power_level(frame, power) as f32 / 31.0)
}

/// The frame after `frame`, or `None` once the last frame has played.
pub(super) fn next_frame(frame: usize, len: usize) -> Option<usize> {
    let next = frame + 1;
    (next < len).then_some(next)
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

/// Spawn a full-screen screen-flash quad of colour `rgb` and RM2000 strength
/// `power` (`0..=31`), whose alpha follows the stepped [`flash_envelope`]. It
/// covers the whole 320×240 overlay at [`SCREEN_FLASH_Z`]; RM2000's animation
/// screen flash is a full-screen tint, not a box on the target.
pub(super) fn spawn_screen_flash(commands: &mut Commands, rgb: [f32; 3], power: u32) {
    let alpha = flash_envelope(0.0, power).unwrap_or(0.0);
    commands.spawn((
        Sprite::from_color(
            Color::srgba(rgb[0], rgb[1], rgb[2], alpha),
            Vec2::new(SCREEN_W, SCREEN_H),
        ),
        Transform::from_translation(overlay_translation(Vec2::ZERO, SCREEN_FLASH_Z)),
        overlay_layer(),
        FlashQuad {
            elapsed: 0.0,
            power,
            rgb,
        },
    ));
}

/// Step each live flash quad's alpha along the RM2000 [`flash_envelope`],
/// despawning it once the ~11-game-frame window has passed.
pub(super) fn fade_flashes(
    transition: crate::transitions::TransitionPause,
    time: Res<Time>,
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut FlashQuad, &mut Sprite)>,
) {
    if transition.paused() {
        return;
    }
    let dt = time.delta_secs();
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.elapsed += dt;
        match flash_envelope(flash.elapsed, flash.power) {
            Some(alpha) => {
                sprite.color = Color::srgba(flash.rgb[0], flash.rgb[1], flash.rgb[2], alpha);
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
    fn next_frame_advances_then_finishes() {
        assert_eq!(next_frame(0, 58), Some(1));
        assert_eq!(next_frame(56, 58), Some(57));
        assert_eq!(next_frame(57, 58), None);
        assert_eq!(next_frame(0, 1), None);
        assert_eq!(next_frame(0, 0), None);
    }

    #[test]
    fn spawn_screen_flash_makes_a_fullscreen_overlay_quad() {
        use bevy::ecs::world::CommandQueue;
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_screen_flash(&mut commands, [1.0, 1.0, 1.0], 31);
        }
        queue.apply(&mut world);
        let mut quads = world.query::<(&FlashQuad, &Sprite)>();
        let (flash, sprite) = quads.single(&world).expect("a screen flash quad");
        assert_eq!(sprite.custom_size, Some(Vec2::new(SCREEN_W, SCREEN_H)));
        assert_eq!(flash.power, 31);
        // A full-strength flash starts at its plateau (level 31 → alpha 1.0).
        assert!((sprite.color.alpha() - 1.0).abs() < 1e-6);
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
        // Frame 0 (t=0) is the plateau; the envelope normalises the level by 31.
        let gf = super::super::GAME_FRAME_SECS;
        assert_eq!(flash_envelope(0.0, 31), Some(1.0));
        // Frame 3 has stepped down to level 25.
        assert_eq!(flash_envelope(3.5 * gf, 31), Some(25.0 / 31.0));
        // Frame 10 is the last lit frame (level 10); frame 11 is spent.
        assert_eq!(flash_envelope(10.5 * gf, 31), Some(10.0 / 31.0));
        assert_eq!(flash_envelope(11.5 * gf, 31), None);
    }
}

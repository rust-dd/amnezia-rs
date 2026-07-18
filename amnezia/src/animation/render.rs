//! Sprite rendering for the effect-animation player: turning an animation
//! frame's cells into overlay sprites, the local target-flash quad, and the
//! pure geometry/colour helpers the player and its tests share.
//!
//! The sprites draw on [`OVERLAY_LAYER`], rendered by the fixed effect-overlay
//! camera (see [`super`]) that sits at the origin with the same fixed 320×240
//! scaling mode as the main camera. So one world unit is one RM2000 pixel, a
//! 96×96 cell sub-rect draws at its native size, and screen-space is pure: an
//! RM2000 offset `(x, y)` from centre (y downward) is world `(x, -y)`, with no
//! camera-follow term. Because that camera has a higher `order` and no clear,
//! the effect composites over the map, the pictures, and the battle UI alike.

use crate::assets::resolve_png;
use amnezia_data::AnimationDef;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

/// The render layer the effect-overlay camera draws. Cells and flash quads carry
/// it so only that fixed, higher-`order` camera renders them — painting over the
/// layer-0 world and UI instead of hiding behind the battle UI.
pub(super) const OVERLAY_LAYER: usize = 1;

/// A fresh [`RenderLayers`] on [`OVERLAY_LAYER`] (the type isn't `Copy`, so each
/// spawned sprite and the overlay camera take their own).
pub(super) fn overlay_layer() -> RenderLayers {
    RenderLayers::layer(OVERLAY_LAYER)
}

/// A sheet is a 5-column, row-major grid of 96×96 cells (a `Battle` graphic is
/// 480 wide, so 5 columns; the row count varies).
const CELL: f32 = 96.0;
const COLS: u32 = 5;

/// World z of the animation cells: well above the map (z < 4) and pictures
/// (100..150), within the 2D camera's ±1000 range. Later cells in a frame add a
/// sliver so they draw over earlier ones, matching RM2000's paint order.
const CELL_Z: f32 = 510.0;

/// World z of a target-flash quad: just under the cells so the effect paints
/// over the flash it triggers.
const FLASH_Z: f32 = 505.0;

/// Edge length of a target-flash quad, roughly one battler. The real RM2000
/// flash tints the target sprite itself; a fixed quad is an approximation.
const FLASH_SIZE: f32 = 96.0;

/// The world translation of an overlay sprite at RM2000 screen offset `pos` from
/// centre (y downward) with depth `z`. The overlay camera sits at the origin, so
/// this is pure screen-space: `x` unchanged, `y` flipped for world y-up.
fn overlay_translation(pos: Vec2, z: f32) -> Vec3 {
    Vec3::new(pos.x, -pos.y, z)
}

/// A decaying target-flash quad: `rgb` its colour (0..1), `peak` the starting
/// alpha, fading linearly to 0 over `secs`.
#[derive(Component)]
pub(super) struct FlashQuad {
    pub elapsed: f32,
    pub secs: f32,
    pub peak: f32,
    pub rgb: [f32; 3],
}

/// The source sub-rect of `cell_id` on its sheet: origin `((id%5)*96,
/// (id/5)*96)`, size 96×96, in texture pixels.
pub(super) fn cell_rect(cell_id: u32) -> Rect {
    let col = (cell_id % COLS) as f32;
    let row = (cell_id / COLS) as f32;
    Rect::new(
        col * CELL,
        row * CELL,
        (col + 1.0) * CELL,
        (row + 1.0) * CELL,
    )
}

/// An RM2000 tone channel (0..=200, 100 = neutral) as a sprite-tint multiplier,
/// clamped to 0..2. Values above 100 brighten (subject to the render pipeline's
/// clamping); `tone_gray` saturation is not representable this way and is
/// ignored.
pub(super) fn tone_channel(tone: i32) -> f32 {
    (tone as f32 / 100.0).clamp(0.0, 2.0)
}

/// Sprite opacity from RM2000 `transparency` percent (0 opaque, 100 invisible).
pub(super) fn alpha_from_transparency(transparency: u32) -> f32 {
    (1.0 - transparency as f32 / 100.0).clamp(0.0, 1.0)
}

/// The frame after `frame`, or `None` once the last frame has played.
pub(super) fn next_frame(frame: usize, len: usize) -> Option<usize> {
    let next = frame + 1;
    (next < len).then_some(next)
}

/// Spawn one sprite per cell of `def`'s `frame`, anchored at `base` (the
/// animation's RM2000 screen offset from centre). Returns the cell entities so
/// the caller can despawn them when the frame advances.
pub(super) fn spawn_frame_cells(
    commands: &mut Commands,
    asset_server: &AssetServer,
    def: &AnimationDef,
    frame: usize,
    base: Vec2,
) -> Vec<Entity> {
    let image = asset_server.load(resolve_png("Battle", &def.animation_name));
    let mut cells = Vec::new();
    for (i, cell) in def.frames[frame].cells.iter().enumerate() {
        let color = Color::srgba(
            tone_channel(cell.tone_red),
            tone_channel(cell.tone_green),
            tone_channel(cell.tone_blue),
            alpha_from_transparency(cell.transparency),
        );
        let pos = Vec2::new(base.x + cell.x as f32, base.y + cell.y as f32);
        let entity = commands
            .spawn((
                Sprite {
                    image: image.clone(),
                    rect: Some(cell_rect(cell.cell_id)),
                    color,
                    ..default()
                },
                Transform {
                    translation: overlay_translation(pos, CELL_Z + i as f32 * 0.1),
                    scale: Vec3::splat(cell.scale as f32 / 100.0),
                    ..default()
                },
                overlay_layer(),
            ))
            .id();
        cells.push(entity);
    }
    cells
}

/// Spawn a target-flash quad of colour `rgb` and starting alpha `peak` at
/// `base`, decaying over `secs`.
pub(super) fn spawn_target_flash(
    commands: &mut Commands,
    base: Vec2,
    rgb: [f32; 3],
    peak: f32,
    secs: f32,
) {
    commands.spawn((
        Sprite::from_color(
            Color::srgba(rgb[0], rgb[1], rgb[2], peak),
            Vec2::splat(FLASH_SIZE),
        ),
        Transform::from_translation(overlay_translation(base, FLASH_Z)),
        overlay_layer(),
        FlashQuad {
            elapsed: 0.0,
            secs,
            peak,
            rgb,
        },
    ));
}

/// Decay each live target flash, repainting its alpha and despawning it once
/// spent.
pub(super) fn fade_flashes(
    time: Res<Time>,
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut FlashQuad, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.elapsed += dt;
        if flash.elapsed >= flash.secs {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = flash.peak * (1.0 - flash.elapsed / flash.secs);
        sprite.color = Color::srgba(flash.rgb[0], flash.rgb[1], flash.rgb[2], alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_rect_indexes_a_5_column_row_major_grid() {
        assert_eq!(cell_rect(0), Rect::new(0.0, 0.0, 96.0, 96.0));
        assert_eq!(cell_rect(4), Rect::new(384.0, 0.0, 480.0, 96.0));
        assert_eq!(cell_rect(5), Rect::new(0.0, 96.0, 96.0, 192.0));
        assert_eq!(cell_rect(12), Rect::new(192.0, 192.0, 288.0, 288.0));
    }

    #[test]
    fn tone_channel_maps_neutral_and_clamps_to_0_2() {
        assert_eq!(tone_channel(100), 1.0);
        assert_eq!(tone_channel(0), 0.0);
        assert_eq!(tone_channel(50), 0.5);
        assert_eq!(tone_channel(200), 2.0);
        assert_eq!(tone_channel(300), 2.0);
        assert_eq!(tone_channel(-50), 0.0);
    }

    #[test]
    fn alpha_is_the_complement_of_transparency() {
        assert_eq!(alpha_from_transparency(0), 1.0);
        assert_eq!(alpha_from_transparency(100), 0.0);
        assert_eq!(alpha_from_transparency(50), 0.5);
        assert_eq!(alpha_from_transparency(150), 0.0);
    }

    #[test]
    fn overlay_translation_flips_y_and_keeps_x_and_z() {
        // On the origin-fixed overlay camera an RM2000 offset (x, y-down) is
        // world (x, -y); z passes through unchanged.
        assert_eq!(
            overlay_translation(Vec2::new(0.0, 0.0), 510.0),
            Vec3::new(0.0, 0.0, 510.0)
        );
        assert_eq!(
            overlay_translation(Vec2::new(40.0, -20.0), 505.0),
            Vec3::new(40.0, 20.0, 505.0)
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
}

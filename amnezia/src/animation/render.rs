//! Sprite rendering for the effect-animation player: turning an animation
//! frame's cells into camera-pinned world sprites, the local target-flash quad,
//! and the pure geometry/colour helpers the player and its tests share.
//!
//! The world camera uses a fixed 320×240 scaling mode, so one world unit is one
//! RM2000 pixel and a 96×96 cell sub-rect draws at its native size. Like
//! `picture.rs`, every sprite is centre-anchored and re-pinned to the (shaken)
//! camera each frame via [`ScreenAnchored`], so the effect stays fixed to the
//! screen instead of scrolling with the map.

use crate::assets::resolve_png;
use amnezia_data::AnimationDef;
use bevy::prelude::*;

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

/// A screen-space overlay pinned to the camera centre every frame. `pos` is the
/// RM2000 screen offset from centre (y grows downward, as in the source data);
/// `z` the world depth to sit at.
#[derive(Component)]
pub(super) struct ScreenAnchored {
    pub pos: Vec2,
    pub z: f32,
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
        let entity = commands
            .spawn((
                Sprite {
                    image: image.clone(),
                    rect: Some(cell_rect(cell.cell_id)),
                    color,
                    ..default()
                },
                Transform::from_scale(Vec3::splat(cell.scale as f32 / 100.0)),
                ScreenAnchored {
                    pos: Vec2::new(base.x + cell.x as f32, base.y + cell.y as f32),
                    z: CELL_Z + i as f32 * 0.1,
                },
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
        Transform::default(),
        ScreenAnchored {
            pos: base,
            z: FLASH_Z,
        },
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

/// Pin every [`ScreenAnchored`] sprite to the (shaken) camera centre plus its
/// screen offset. RM2000 y grows downward, so it flips against world y. Runs in
/// `PostUpdate` after the shake so effects shake with the view.
pub(super) fn pin_to_screen(
    cameras: Query<&Transform, (With<Camera2d>, Without<ScreenAnchored>)>,
    mut anchored: Query<(&ScreenAnchored, &mut Transform), Without<Camera2d>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let center = camera.translation.truncate();
    for (anchor, mut transform) in &mut anchored {
        transform.translation = (center + Vec2::new(anchor.pos.x, -anchor.pos.y)).extend(anchor.z);
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
    fn next_frame_advances_then_finishes() {
        assert_eq!(next_frame(0, 58), Some(1));
        assert_eq!(next_frame(56, 58), Some(57));
        assert_eq!(next_frame(57, 58), None);
        assert_eq!(next_frame(0, 1), None);
        assert_eq!(next_frame(0, 0), None);
    }
}

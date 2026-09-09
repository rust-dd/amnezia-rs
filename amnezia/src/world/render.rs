//! Tile and NPC spawning helpers for [`super::load_map`]: a single tile sprite,
//! the four quarter sprites of an assembled lower-layer autotile, and an event's
//! active-page NPC sprite. Every spawned entity is tagged [`MapScene`] so a
//! teleport can despawn the whole scene at once.

use super::MapScene;
use super::water::WaterQuarter;
use crate::tiles;
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct MapTile(pub Vec2);

pub(super) fn spawn_tile(
    commands: &mut Commands,
    chipset: &Handle<Image>,
    source: (f32, f32),
    index: i32,
    width: i32,
    offset: (f32, f32),
    z: f32,
) -> Entity {
    let world_x = (index % width) as f32 * tiles::TILE - offset.0 + tiles::TILE / 2.0;
    let world_y = offset.1 - (index / width) as f32 * tiles::TILE - tiles::TILE / 2.0;
    commands
        .spawn((
            Sprite {
                image: chipset.clone(),
                rect: Some(Rect::new(
                    source.0,
                    source.1,
                    source.0 + tiles::TILE,
                    source.1 + tiles::TILE,
                )),
                custom_size: Some(Vec2::splat(tiles::TILE)),
                ..default()
            },
            Transform::from_xyz(world_x, world_y, z),
            MapTile(Vec2::new(world_x, world_y)),
            MapScene,
        ))
        .id()
}

/// Spawn the four 8×8 quarter sprites of an assembled lower-layer autotile at
/// map cell `index`, each drawing its own chipset sub-rect at its offset within
/// the tile so the shape's edges and corners compose correctly. Quarters of an
/// animated `BLOCK_A`/`BLOCK_B` water tile are tagged [`WaterQuarter`] so
/// [`super::water::animate_water`] can scroll their source column.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_lower_quarters(
    commands: &mut Commands,
    chipset: &Handle<Image>,
    quarters: &[tiles::Quarter; 4],
    id: u16,
    index: i32,
    width: i32,
    offset: (f32, f32),
    z: f32,
) {
    let left = (index % width) as f32 * tiles::TILE - offset.0;
    let top = offset.1 - (index / width) as f32 * tiles::TILE;
    for (quarter, q) in quarters.iter().enumerate() {
        let world_x = left + q.dst.0 + tiles::QUARTER / 2.0;
        let world_y = top - q.dst.1 - tiles::QUARTER / 2.0;
        let mut entity = commands.spawn((
            Sprite {
                image: chipset.clone(),
                rect: Some(Rect::new(
                    q.src.0,
                    q.src.1,
                    q.src.0 + tiles::QUARTER,
                    q.src.1 + tiles::QUARTER,
                )),
                custom_size: Some(Vec2::splat(tiles::QUARTER)),
                ..default()
            },
            Transform::from_xyz(world_x, world_y, z),
            MapTile(Vec2::new(world_x, world_y)),
            MapScene,
        ));
        if tiles::is_ab_water(id) {
            entity.insert(WaterQuarter { id, quarter });
        }
    }
}

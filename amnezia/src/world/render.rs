//! Tile and NPC spawning helpers for [`super::load_map`]: a single tile sprite,
//! the four quarter sprites of an assembled lower-layer autotile, and an event's
//! active-page NPC sprite. Every spawned entity is tagged [`MapScene`] so a
//! teleport can despawn the whole scene at once.

use super::water::WaterQuarter;
use super::{EventSprite, MapScene, MoveQueue};
use crate::assets::resolve_png;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN};
use amnezia_data::Event;
use bevy::prelude::*;

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
            MapScene,
        ));
        if tiles::is_ab_water(id) {
            entity.insert(WaterQuarter { id, quarter });
        }
    }
}

/// Spawn an NPC sprite for an event's active page graphic, if it has one. The
/// active page is chosen per the current switches/variables at load time.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_event_npc(
    commands: &mut Commands,
    asset_server: &AssetServer,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
    event: &Event,
    offset: (f32, f32),
) {
    let Some(page) = active_page(event, switches, variables, party, inventory) else {
        return;
    };
    if page.graphic_name.is_empty() {
        return;
    }
    let image = asset_server.load(resolve_png("CharSet", &page.graphic_name));
    let (sx, sy) = tiles::charset_source(page.graphic_index, DIR_DOWN, 1);
    let world_x = event.x as f32 * tiles::TILE - offset.0 + tiles::TILE / 2.0;
    let world_y = offset.1 - event.y as f32 * tiles::TILE - tiles::TILE / 2.0 + CHAR_Y_OFFSET;
    commands.spawn((
        Sprite {
            image,
            rect: Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H)),
            custom_size: Some(Vec2::new(tiles::CHAR_W, tiles::CHAR_H)),
            ..default()
        },
        Transform::from_xyz(world_x, world_y, tiles::character_z(event.y as i32)),
        EventSprite {
            id: event.id,
            tile_x: event.x as i32,
            tile_y: event.y as i32,
            dir: DIR_DOWN,
            frame: 1,
            charset: page.graphic_name.clone(),
            index: page.graphic_index,
        },
        MoveQueue::default(),
        MapScene,
    ));
}

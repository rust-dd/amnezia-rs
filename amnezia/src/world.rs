//! World setup: loads the start map, renders its tile layers and event NPCs,
//! spawns the player, and stores the map data + events as resources.

use crate::assets::{load_ron, resolve_png, ASSET_ROOT};
use crate::player::spawn_player;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN};
use amnezia_data::{Chipset, Event, Map};
use bevy::prelude::*;

const START_MAP: &str = "map_0001";

/// The active map's geometry, tile layers, and passability, for movement.
#[derive(Resource)]
pub struct MapData {
    pub width: i32,
    pub height: i32,
    offset_x: f32,
    offset_y: f32,
    lower: Vec<u16>,
    upper: Vec<u16>,
    passages_down: Vec<u8>,
    passages_up: Vec<u8>,
}

impl MapData {
    /// World-space center of a tile.
    pub fn tile_center(&self, tile_x: i32, tile_y: i32) -> (f32, f32) {
        let x = tile_x as f32 * tiles::TILE - self.offset_x + tiles::TILE / 2.0;
        let y = self.offset_y - tile_y as f32 * tiles::TILE - tiles::TILE / 2.0;
        (x, y)
    }

    /// Whether the hero can stand on tile `(x, y)`.
    pub fn passable(&self, x: i32, y: i32) -> bool {
        let idx = (y * self.width + x) as usize;
        tiles::passable(self.lower[idx], self.upper[idx], &self.passages_down, &self.passages_up)
    }
}

/// The active map's events, for interaction lookups.
#[derive(Resource, Default)]
pub struct MapEvents {
    pub events: Vec<Event>,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let map: Map = load_ron(&format!("{ASSET_ROOT}/maps/{START_MAP}.ron"));
    let chipsets: Vec<Chipset> = load_ron(&format!("{ASSET_ROOT}/chipsets.ron"));
    let entry = chipsets.into_iter().find(|c| c.id == map.chipset_id);
    let (graphic, passages_down, passages_up) = match entry {
        Some(c) => (c.graphic, c.passages_down, c.passages_up),
        None => (String::new(), vec![0x0F; 162], vec![0x0F; 144]),
    };
    let chipset = asset_server.load(resolve_png("ChipSet", &graphic));

    let width = map.width as i32;
    let offset_x = map.width as f32 * tiles::TILE / 2.0;
    let offset_y = map.height as f32 * tiles::TILE / 2.0;

    for (index, &id) in map.lower.iter().enumerate() {
        let (sx, sy) = tiles::lower_source(id);
        spawn_tile(&mut commands, &chipset, (sx, sy), index as i32, width, (offset_x, offset_y), 0.0);
    }
    for (index, &id) in map.upper.iter().enumerate() {
        if let Some((sx, sy)) = tiles::upper_source(id) {
            spawn_tile(&mut commands, &chipset, (sx, sy), index as i32, width, (offset_x, offset_y), 1.0);
        }
    }

    for event in &map.events {
        spawn_event_npc(&mut commands, &asset_server, event, (offset_x, offset_y));
    }

    let start = (width / 2, map.height as i32 / 2);
    spawn_player(&mut commands, &asset_server, start, (offset_x, offset_y));

    commands.insert_resource(MapData {
        width,
        height: map.height as i32,
        offset_x,
        offset_y,
        lower: map.lower,
        upper: map.upper,
        passages_down,
        passages_up,
    });
    commands.insert_resource(MapEvents { events: map.events });
}

fn spawn_tile(
    commands: &mut Commands,
    chipset: &Handle<Image>,
    source: (f32, f32),
    index: i32,
    width: i32,
    offset: (f32, f32),
    z: f32,
) {
    let world_x = (index % width) as f32 * tiles::TILE - offset.0 + tiles::TILE / 2.0;
    let world_y = offset.1 - (index / width) as f32 * tiles::TILE - tiles::TILE / 2.0;
    commands.spawn((
        Sprite {
            image: chipset.clone(),
            rect: Some(Rect::new(source.0, source.1, source.0 + tiles::TILE, source.1 + tiles::TILE)),
            custom_size: Some(Vec2::splat(tiles::TILE)),
            ..default()
        },
        Transform::from_xyz(world_x, world_y, z),
    ));
}

/// Spawn an NPC sprite for an event's active page graphic, if it has one.
fn spawn_event_npc(
    commands: &mut Commands,
    asset_server: &AssetServer,
    event: &Event,
    offset: (f32, f32),
) {
    let Some(page) = event.pages.last() else {
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
        Transform::from_xyz(world_x, world_y, 2.0),
    ));
}

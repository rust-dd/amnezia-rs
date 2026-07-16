//! World setup and map loading: renders a map's tile layers and event NPCs,
//! stores its data + events as resources, and can reload for a teleport. The
//! player entity persists across map changes; scene entities are tagged
//! [`MapScene`] so a teleport can despawn them.

use crate::assets::{load_ron, resolve_png, ASSET_ROOT};
use crate::player::spawn_player;
use crate::state::{active_page, Switches, Variables};
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN};
use amnezia_data::{Chipset, Event, Map, Start};
use bevy::prelude::*;

/// Temporary developer start override, used until the event interpreter can run
/// the intro map's autorun cutscene. The faithful start (`start.ron`, the intro
/// map) is a black "Black"-chipset scene that only comes alive with the
/// cutscene, so for now we drop the hero onto an open, walkable tile in the
/// village (map_0001) to keep the world testable. Set to `None` to use the
/// faithful LMT start once the intro cutscene runs.
const DEV_START: Option<Start> = Some(Start { map_id: 1, x: 20, y: 13 });

/// Tag for entities belonging to the current map (tiles, NPCs); despawned on a
/// teleport. The player is deliberately untagged so it persists.
#[derive(Component)]
pub struct MapScene;

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

/// The active map's events, for interaction and touch lookups.
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    switches: Res<Switches>,
    variables: Res<Variables>,
) {
    commands.spawn(Camera2d);
    let start: Start = match DEV_START {
        Some(dev) => dev,
        None => load_ron(&format!("{ASSET_ROOT}/start.ron")),
    };
    let (data, events) =
        load_map(&mut commands, &asset_server, &switches, &variables, start.map_id);
    spawn_player(&mut commands, &asset_server, (start.x as i32, start.y as i32), &data);
    commands.insert_resource(data);
    commands.insert_resource(events);
}

/// Load map `map_id` into the world: spawn its tile layers and event NPCs
/// (tagged [`MapScene`]) and return fresh [`MapData`]/[`MapEvents`]. Each event's
/// sprite is its active page's graphic (per the current switches/variables). The
/// caller installs the resources — as `Commands` on first load, or `ResMut`
/// overwrite on a teleport so they take effect the same frame the hero moves.
pub fn load_map(
    commands: &mut Commands,
    asset_server: &AssetServer,
    switches: &Switches,
    variables: &Variables,
    map_id: u32,
) -> (MapData, MapEvents) {
    let map: Map = load_ron(&format!("{ASSET_ROOT}/maps/map_{map_id:04}.ron"));
    let chipsets: Vec<Chipset> = load_ron(&format!("{ASSET_ROOT}/chipsets.ron"));
    let entry = chipsets.into_iter().find(|c| c.id == map.chipset_id);
    let (graphic, passages_down, passages_up) = match entry {
        Some(c) => (c.graphic, c.passages_down, c.passages_up),
        None => (String::new(), vec![0x0F; 162], vec![0x0F; 144]),
    };
    let chipset = asset_server.load(resolve_png("ChipSet", &graphic));

    let width = map.width as i32;
    let height = map.height as i32;
    let offset = (map.width as f32 * tiles::TILE / 2.0, map.height as f32 * tiles::TILE / 2.0);

    for (index, &id) in map.lower.iter().enumerate() {
        spawn_tile(commands, &chipset, tiles::lower_source(id), index as i32, width, offset, 0.0);
    }
    for (index, &id) in map.upper.iter().enumerate() {
        if let Some(source) = tiles::upper_source(id) {
            spawn_tile(commands, &chipset, source, index as i32, width, offset, 1.0);
        }
    }
    for event in &map.events {
        spawn_event_npc(commands, asset_server, switches, variables, event, offset);
    }

    let data = MapData {
        width,
        height,
        offset_x: offset.0,
        offset_y: offset.1,
        lower: map.lower,
        upper: map.upper,
        passages_down,
        passages_up,
    };
    (data, MapEvents { events: map.events })
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
        MapScene,
    ));
}

/// Spawn an NPC sprite for an event's active page graphic, if it has one. The
/// active page is chosen per the current switches/variables at load time.
fn spawn_event_npc(
    commands: &mut Commands,
    asset_server: &AssetServer,
    switches: &Switches,
    variables: &Variables,
    event: &Event,
    offset: (f32, f32),
) {
    let Some(page) = active_page(event, switches, variables) else {
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
        MapScene,
    ));
}

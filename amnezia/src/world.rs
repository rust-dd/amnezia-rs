//! Map loading and rendering. Transfers replace [`MapScene`] entities but retain the player.

use crate::assets::{asset_root, load_ron, resolve_png};
use crate::player::spawn_player;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::tiles;
use amnezia_data::{Chipset, Event, Map, Start};
use bevy::prelude::*;

mod autonomy;
mod bush;
mod cameras;
mod character_animation;
pub(crate) mod collision;
mod movement;
pub(crate) mod overlap_smoke;
mod pages;
pub(crate) mod passage_smoke;
mod relocation;
mod render;
mod route;
pub(crate) mod saved;
mod scene_pause;
pub(crate) mod scene_smoke;
mod screen;
pub(crate) mod stop_clock;
mod terrain;
pub(crate) mod terrain_smoke;
#[cfg(test)]
pub(crate) mod test_support;
mod topology;
mod touch;
mod triggers;
pub(crate) mod update;
mod water;
pub(crate) use water::smoke as water_smoke;

pub use autonomy::AutoMove;
pub(crate) use autonomy::MoveGuards;
pub(crate) use bush::BushBottom;
pub(crate) use cameras::{HudCamera, setup as setup_cameras};
pub use movement::{Character, MoveQueue, RouteAction, walk};
pub(crate) use movement::{ScrollStep, dir_delta, step_secs_for_speed};
use relocation::apply_relocate;
pub use route::RouteStepper;
#[cfg(test)]
pub(crate) use route::drive as drive_route;
pub(crate) use route::{
    Attempt as RouteAttempt, Boundary as RouteBoundary, Progress as RouteProgress, StepEffect,
    Turn as RouteTurn,
};
pub(crate) use scene_pause::ScenePause;
pub(crate) use screen::MapScreen;
pub(crate) use touch::TouchEvents;
pub(crate) use triggers::{EventTriggers, finish_foreground};

/// Developer override; `None` uses the LMT start from `start.ron`.
const DEV_START: Option<Start> = None;

/// Tag for entities belonging to the current map (tiles, NPCs); despawned on a
/// teleport. The player is deliberately untagged so it persists.
#[derive(Component)]
pub struct MapScene;

/// The followed, shaken layer-0 camera. World bitmaps carry their own tone;
/// pictures and UI stay untinted on their separate cameras.
#[derive(Component)]
pub struct MainCamera;

/// Live NPC state; [`walk`] owns movement and [`update_event_sprites`] refreshes appearance.
#[derive(Component, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventSprite {
    pub id: u32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub dir: u32,
    pub frame: u32,
    pub charset: String,
    pub index: u32,
    /// The active page's draw layer (0 below the hero, 1 same, 2 above), which
    /// places the sprite's y-sorted Z relative to the hero via
    /// [`crate::tiles::character_z_layer`].
    pub layer: u32,
}

/// The active map's geometry, tile layers, and passability, for movement.
#[derive(Resource)]
pub struct MapData {
    pub scroll_type: u32,
    pub panorama: Option<amnezia_data::PanoramaDef>,
    pub map_id: u32,
    pub width: i32,
    pub height: i32,
    offset_x: f32,
    offset_y: f32,
    lower: Vec<u16>,
    upper: Vec<u16>,
    passages_down: Vec<u8>,
    passages_up: Vec<u8>,
    terrain_data: Vec<u16>,
    terrains: Vec<amnezia_data::TerrainDef>,
}

impl MapData {
    /// World-space center of a tile.
    pub fn tile_center(&self, tile_x: i32, tile_y: i32) -> (f32, f32) {
        let x = tile_x as f32 * tiles::TILE - self.offset_x + tiles::TILE / 2.0;
        let y = self.offset_y - tile_y as f32 * tiles::TILE - tiles::TILE / 2.0;
        (x, y)
    }

    /// Whether tile `(x, y)` is passable from any direction — a non-directional
    /// standability test, used by the debug overlay. Movement additionally
    /// checks both tile edges and live events through `MapCollision`.
    pub fn passable(&self, x: i32, y: i32) -> bool {
        self.passable_dir(x, y, tiles::PASS_ALL)
    }

    /// Whether the cell at `(x, y)` permits passage in the direction(s) `bit`,
    /// combining both layers. Looping axes wrap; other map boundaries block.
    fn passable_dir(&self, x: i32, y: i32, bit: u8) -> bool {
        let (x, y) = self.normalize_tile(x, y);
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return false;
        }
        let idx = (y * self.width + x) as usize;
        tiles::passable(
            self.lower[idx],
            self.upper[idx],
            &self.passages_down,
            &self.passages_up,
            bit,
        )
    }

    /// Whether a character can move from `(fx, fy)` to the adjacent `(tx, ty)`,
    /// per RM2000's `Game_Map::MakeWay`: the tile being left must permit exit
    /// toward the move, and the tile being entered must permit entry from the
    /// opposite side. Both checks combine each tile's lower and upper passability.
    #[cfg(test)]
    pub fn can_move(&self, fx: i32, fy: i32, tx: i32, ty: i32) -> bool {
        let bit_from = tiles::passable_mask(fx, fy, tx, ty);
        let bit_to = tiles::passable_mask(tx, ty, fx, fy);
        self.passable_dir(fx, fy, bit_from) && self.passable_dir(tx, ty, bit_to)
    }

    /// Whether the tile `(x, y)` is a counter (its upper tile carries the counter
    /// bit): an action event one tile beyond can be talked to across it.
    pub fn is_counter(&self, x: i32, y: i32) -> bool {
        let (x, y) = self.normalize_tile(x, y);
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return false;
        }
        let idx = (y * self.width + x) as usize;
        tiles::is_counter(self.upper[idx], &self.passages_up)
    }
}

#[cfg(test)]
impl MapData {
    pub(crate) fn set_counter_for_test(&mut self, x: i32, y: i32) {
        self.upper[(y * self.width + x) as usize] = 10001;
        self.passages_up[1] |= 0x40;
    }

    /// Build a bare, fully passable `MapData` of the given tile dimensions for
    /// headless tests: offsets centered as on a real load, all tiles empty.
    pub(crate) fn for_test(width: i32, height: i32) -> MapData {
        MapData {
            scroll_type: 0,
            panorama: None,
            map_id: 0,
            width,
            height,
            offset_x: width as f32 * tiles::TILE / 2.0,
            offset_y: height as f32 * tiles::TILE / 2.0,
            lower: vec![0; (width * height) as usize],
            upper: vec![10000; (width * height) as usize],
            passages_down: vec![0x0F; 162],
            passages_up: vec![0x0F; 144],
            terrain_data: Vec::new(),
            terrains: vec![amnezia_data::TerrainDef { id: 1, ..default() }],
        }
    }
}

/// The active map's events, for interaction and touch lookups.
#[derive(Resource, Default)]
pub struct MapEvents {
    pub events: Vec<Event>,
}

/// Opcode 10860 relocation, applied to both logical collision state and the rendered sprite.
#[derive(Message)]
pub struct RelocateEvent {
    pub event_id: u32,
    pub x: u32,
    pub y: u32,
}

/// Arrival at a transfer destination, including same-map repositioning.
#[derive(Message)]
pub struct MapChanged;

/// A rebuilt map scene, including loading a saved map, but not same-map repositioning.
#[derive(Message)]
pub(crate) struct MapRebuilt;

/// Ordinary map transfers clear transient effects; quick vehicle transfers do not.
#[derive(Message)]
pub(crate) struct MapEffectsReset;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<water::WaterStyle>()
            .init_resource::<crate::timing::GameFrames>()
            .init_resource::<crate::timing::SceneFrames>()
            .init_resource::<TouchEvents>()
            .add_message::<RelocateEvent>()
            .add_message::<MapChanged>()
            .add_systems(Startup, (cameras::setup, setup))
            .add_systems(
                PostUpdate,
                (topology::wrap_scene, bush::update)
                    .chain()
                    .after(crate::legacy_colors::world::WorldColors)
                    .after(crate::vehicles::VehicleDisplay)
                    .after(crate::screenfx::ScreenShakeSet)
                    .before(bevy::camera::visibility::VisibilitySystems::CalculateBounds)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
        app.add_systems(PostUpdate, water::animate_water);
        update::register(app);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) {
    let start: Start = match DEV_START {
        Some(dev) => dev,
        None => load_ron(&format!("{}/start.ron", asset_root())),
    };
    let (data, events) = load_map(
        &mut commands,
        &asset_server,
        &switches,
        &variables,
        &party,
        &inventory,
        start.map_id,
    );
    spawn_player(
        &mut commands,
        &asset_server,
        (start.x as i32, start.y as i32),
        &data,
    );
    commands.insert_resource(data);
    commands.insert_resource(events);
}

/// Spawn the active map and return its resources. Transfer callers install them
/// immediately so collision state changes in the same frame as the hero's position.
#[allow(clippy::too_many_arguments)]
pub fn load_map(
    commands: &mut Commands,
    asset_server: &AssetServer,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
    map_id: u32,
) -> (MapData, MapEvents) {
    let map: Map = load_ron(&format!("{}/maps/map_{map_id:04}.ron", asset_root()));
    let chipsets: Vec<Chipset> = load_ron(&format!("{}/chipsets.ron", asset_root()));
    let entry = chipsets.into_iter().find(|c| c.id == map.chipset_id);
    commands.insert_resource(
        entry
            .as_ref()
            .map(water::WaterStyle::from_chipset)
            .unwrap_or_default(),
    );
    let (graphic, passages_down, passages_up, terrain_data) = match entry {
        Some(c) => (c.graphic, c.passages_down, c.passages_up, c.terrain_data),
        None => (String::new(), vec![0x0F; 162], vec![0x0F; 144], Vec::new()),
    };
    let chipset = asset_server.load(resolve_png("ChipSet", &graphic));
    commands.insert_resource(pages::EventTileset(chipset.clone()));

    let width = map.width as i32;
    let height = map.height as i32;
    let offset = (
        map.width as f32 * tiles::TILE / 2.0,
        map.height as f32 * tiles::TILE / 2.0,
    );

    for (index, &id) in map.lower.iter().enumerate() {
        let z = if tiles::above_hero_lower(id, &passages_down) {
            tiles::Z_TILE_ABOVE
        } else {
            tiles::Z_GROUND
        };
        match tiles::lower_render(id) {
            tiles::LowerRender::Whole { src } => {
                let tile =
                    render::spawn_tile(commands, &chipset, src, index as i32, width, offset, z);
                if tiles::is_block_c(id) {
                    commands.entity(tile).insert(water::WaterCell { id });
                }
            }
            tiles::LowerRender::Quarters(quarters) => {
                render::spawn_lower_quarters(
                    commands,
                    &chipset,
                    &quarters,
                    id,
                    index as i32,
                    width,
                    offset,
                    z,
                );
            }
        }
    }
    for (index, &id) in map.upper.iter().enumerate() {
        if let Some(source) = tiles::upper_source(id) {
            let z = if tiles::above_hero(id, &passages_up) {
                tiles::Z_TILE_ABOVE
            } else {
                tiles::Z_UPPER
            };
            render::spawn_tile(commands, &chipset, source, index as i32, width, offset, z);
        }
    }
    for event in &map.events {
        pages::spawn_event(
            commands,
            asset_server,
            (switches, variables, party, inventory),
            event,
            offset,
            &chipset,
        );
    }

    let data = MapData {
        scroll_type: map.scroll_type,
        panorama: map.panorama,
        map_id,
        width,
        height,
        offset_x: offset.0,
        offset_y: offset.1,
        lower: map.lower,
        upper: map.upper,
        passages_down,
        passages_up,
        terrain_data,
        terrains: load_ron(&format!("{}/terrains.ron", asset_root())),
    };
    (data, MapEvents { events: map.events })
}

/// Refresh changed NPC graphics and visibility. [`walk`] owns active-step placement.
#[allow(clippy::type_complexity)]
fn update_event_sprites(
    asset_server: Res<AssetServer>,
    data: Res<MapData>,
    tileset: Option<Res<pages::EventTileset>>,
    mut sprites: Query<
        (
            &EventSprite,
            &MoveQueue,
            Option<&RouteStepper>,
            &mut Sprite,
            &mut Visibility,
            &mut Transform,
        ),
        Changed<EventSprite>,
    >,
) {
    let Some(tileset) = tileset else {
        return;
    };
    for (event, queue, route, mut sprite, mut visibility, mut transform) in &mut sprites {
        let (mut graphic, visible) = pages::graphic(event, &tileset.0, &asset_server);
        graphic.color = sprite.color;
        *sprite = graphic;
        *visibility = if route.is_some_and(|route| !route.page_present()) {
            Visibility::Hidden
        } else {
            visible
        };
        if queue.busy() {
            continue;
        }
        let (x, y) = data.tile_center(event.tile_x, event.tile_y);
        transform.translation = Vec3::new(x, y + event.y_offset(), event.draw_z(event.tile_y));
    }
}

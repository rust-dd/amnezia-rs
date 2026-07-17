//! World setup and map loading: renders a map's tile layers and event NPCs,
//! stores its data + events as resources, and can reload for a teleport. The
//! player entity persists across map changes; scene entities are tagged
//! [`MapScene`] so a teleport can despawn them.

use crate::assets::{ASSET_ROOT, load_ron, resolve_png};
use crate::player::spawn_player;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN};
use amnezia_data::{Chipset, Event, Map, Start};
use bevy::prelude::*;

mod movement;

pub use movement::{Character, MoveQueue, RouteAction, decode_route, walk};

/// Developer start override. `None` uses the faithful LMT start (`start.ron`,
/// the intro map_0005), whose autorun cutscene the interpreter now runs; set it
/// to `Some(Start { .. })` to drop the hero onto a specific map/tile for testing
/// instead.
const DEV_START: Option<Start> = None;

/// Tag for entities belonging to the current map (tiles, NPCs); despawned on a
/// teleport. The player is deliberately untagged so it persists.
#[derive(Component)]
pub struct MapScene;

/// A rendered event NPC: its event id, live tile position, and current
/// facing/frame/graphic. A running `MoveEvent` enqueues route steps that
/// [`walk`] tweens across tiles (updating `tile_x`/`tile_y`), while
/// [`update_event_sprites`] reflects facing/frame/graphic changes onto the
/// sprite when the NPC is not mid-step.
#[derive(Component)]
pub struct EventSprite {
    pub id: u32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub dir: u32,
    pub frame: u32,
    pub charset: String,
    pub index: u32,
}

/// The active map's geometry, tile layers, and passability, for movement.
#[derive(Resource)]
pub struct MapData {
    pub map_id: u32,
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
        tiles::passable(
            self.lower[idx],
            self.upper[idx],
            &self.passages_down,
            &self.passages_up,
        )
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
        app.init_resource::<WaterAnim>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (walk::<EventSprite>, update_event_sprites).chain(),
                    animate_water,
                ),
            );
    }
}

/// Marks one 8×8 quarter of an animated `BLOCK_A`/`BLOCK_B` water tile so
/// [`animate_water`] can re-sample its scrolling source column each frame.
#[derive(Component)]
struct WaterQuarter {
    id: u16,
    quarter: usize,
}

/// Marks a whole-cell `BLOCK_C` animated tile (waterfalls) for [`animate_water`].
#[derive(Component)]
struct WaterCell {
    id: u16,
}

/// The shared water-animation clock: each timer tick advances `step`, which
/// drives the `BLOCK_A`/`BLOCK_B` column cycle and the `BLOCK_C` row cycle.
#[derive(Resource)]
struct WaterAnim {
    timer: Timer,
    step: u32,
}

impl Default for WaterAnim {
    fn default() -> Self {
        // RM2000 water animates gently; ~0.4 s per step gives the classic cadence
        // (we do not parse the chipset's `animation_speed` database field).
        Self {
            timer: Timer::from_seconds(0.4, TimerMode::Repeating),
            step: 0,
        }
    }
}

/// Advance the water clock and re-point every animated water sprite at its
/// current frame: `BLOCK_A`/`BLOCK_B` quarters scroll one chipset column per step
/// ([`tiles::WATER_FRAMES`]); `BLOCK_C` cells step down one animation row.
fn animate_water(
    time: Res<Time>,
    mut anim: ResMut<WaterAnim>,
    mut quarters: Query<(&WaterQuarter, &mut Sprite), Without<WaterCell>>,
    mut cells: Query<(&WaterCell, &mut Sprite), Without<WaterQuarter>>,
) {
    if !anim.timer.tick(time.delta()).just_finished() {
        return;
    }
    anim.step = anim.step.wrapping_add(1);
    let ab_frame = tiles::WATER_FRAMES[(anim.step % tiles::WATER_FRAMES.len() as u32) as usize];
    for (water, mut sprite) in &mut quarters {
        let src = tiles::water_quarters(water.id, ab_frame)[water.quarter].src;
        sprite.rect = Some(Rect::new(
            src.0,
            src.1,
            src.0 + tiles::QUARTER,
            src.1 + tiles::QUARTER,
        ));
    }
    let c_frame = (anim.step % u32::from(tiles::BLOCK_C_FRAMES)) as u16;
    for (water, mut sprite) in &mut cells {
        let src = tiles::block_c_source(water.id, c_frame);
        sprite.rect = Some(Rect::new(
            src.0,
            src.1,
            src.0 + tiles::TILE,
            src.1 + tiles::TILE,
        ));
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
    commands.spawn(Camera2d);
    let start: Start = match DEV_START {
        Some(dev) => dev,
        None => load_ron(&format!("{ASSET_ROOT}/start.ron")),
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

/// Load map `map_id` into the world: spawn its tile layers and event NPCs
/// (tagged [`MapScene`]) and return fresh [`MapData`]/[`MapEvents`]. Each event's
/// sprite is its active page's graphic (per the current switches/variables). The
/// caller installs the resources — as `Commands` on first load, or `ResMut`
/// overwrite on a teleport so they take effect the same frame the hero moves.
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
    let offset = (
        map.width as f32 * tiles::TILE / 2.0,
        map.height as f32 * tiles::TILE / 2.0,
    );

    for (index, &id) in map.lower.iter().enumerate() {
        // A star-flagged lower tile (roof/wall top/treetop painted on the ground
        // layer) draws above the hero (z 4), the same rule the upper layer uses;
        // ordinary ground stays at z 0 below everything.
        let z = if tiles::above_hero_lower(id, &passages_down) {
            4.0
        } else {
            0.0
        };
        match tiles::lower_render(id) {
            tiles::LowerRender::Whole { src } => {
                let tile = spawn_tile(commands, &chipset, src, index as i32, width, offset, z);
                if tiles::is_block_c(id) {
                    commands.entity(tile).insert(WaterCell { id });
                }
            }
            tiles::LowerRender::Quarters(quarters) => {
                spawn_lower_quarters(
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
            // "Above hero" upper tiles (roof/tree/tall-object tops) draw over the
            // hero (z 4 > player z 3) so the hero walks behind them; ordinary
            // upper tiles stay below the hero at z 1.
            let z = if tiles::above_hero(id, &passages_up) {
                4.0
            } else {
                1.0
            };
            spawn_tile(commands, &chipset, source, index as i32, width, offset, z);
        }
    }
    for event in &map.events {
        spawn_event_npc(
            commands,
            asset_server,
            switches,
            variables,
            party,
            inventory,
            event,
            offset,
        );
    }

    let data = MapData {
        map_id,
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
/// [`animate_water`] can scroll their source column.
#[allow(clippy::too_many_arguments)]
fn spawn_lower_quarters(
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
fn spawn_event_npc(
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

/// Re-render event NPCs whose facing/frame/graphic changed, reflecting the new
/// charset sub-rect (and charset image) onto the sprite. Skips NPCs mid-step:
/// [`walk`] owns their rendering while a move tweens.
fn update_event_sprites(
    asset_server: Res<AssetServer>,
    mut sprites: Query<(&EventSprite, &MoveQueue, &mut Sprite), Changed<EventSprite>>,
) {
    for (event, queue, mut sprite) in &mut sprites {
        if event.charset.is_empty() || queue.busy() {
            continue;
        }
        sprite.image = asset_server.load(resolve_png("CharSet", &event.charset));
        let (sx, sy) = tiles::charset_source(event.index, event.dir, event.frame);
        sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
    }
}

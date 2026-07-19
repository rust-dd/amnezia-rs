//! World setup and map loading: renders a map's tile layers and event NPCs,
//! stores its data + events as resources, and can reload for a teleport. The
//! player entity persists across map changes; scene entities are tagged
//! [`MapScene`] so a teleport can despawn them.

use crate::assets::{asset_root, load_ron, resolve_png};
use crate::player::spawn_player;
use crate::screenfx::{FrontCamera, PICTURE_LAYER, ScreenTone};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::tiles::{self, CHAR_Y_OFFSET};
use amnezia_data::{Chipset, Event, Map, Start};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

mod autonomy;
mod movement;
mod render;
mod water;

pub use autonomy::AutoMove;
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

/// The main world camera: it follows the hero (`player::camera_follow`), takes
/// the screen shake, and draws the map on the default render layer 0, where its
/// [`crate::screenfx::ScreenTone`] post-process tints it. Pictures and the UI
/// render untinted on the [`crate::screenfx::FrontCamera`] above it. The overlay
/// camera in [`crate::animation`] and the front camera both lack this marker, so
/// the follow, shake, and picture-pinning systems keep matching exactly one
/// camera.
#[derive(Component)]
pub struct MainCamera;

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

#[cfg(test)]
impl MapData {
    /// Build a bare, fully passable `MapData` of the given tile dimensions for
    /// headless tests: offsets centered as on a real load, all tiles empty.
    pub(crate) fn for_test(width: i32, height: i32) -> MapData {
        MapData {
            map_id: 0,
            width,
            height,
            offset_x: width as f32 * tiles::TILE / 2.0,
            offset_y: height as f32 * tiles::TILE / 2.0,
            lower: vec![0; (width * height) as usize],
            upper: vec![10000; (width * height) as usize],
            passages_down: vec![0x0F; 162],
            passages_up: vec![0x0F; 144],
        }
    }
}

/// The active map's events, for interaction and touch lookups.
#[derive(Resource, Default)]
pub struct MapEvents {
    pub events: Vec<Event>,
}

/// A request to teleport an event to a tile (RM2000 opcode 10860). The
/// interpreter resolves the target id and concrete coordinates and writes one
/// per `ChangeEventLocation`; `apply_relocate` moves both the logical
/// [`MapEvents`] entry (collision/touch) and, if present, the [`EventSprite`].
#[derive(Message)]
pub struct RelocateEvent {
    pub event_id: u32,
    pub x: u32,
    pub y: u32,
}

/// Emitted once when the active map is replaced — a teleport/transfer or a
/// save-load, both routed through the fade's `swap_map`. Pictures clear on it,
/// matching RPG Maker 2000's transfer default (it erases pictures on transfer).
#[derive(Message)]
pub struct MapChanged;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<water::WaterAnim>()
            .add_message::<RelocateEvent>()
            .add_message::<MapChanged>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (
                        autonomy::autonomous_movement,
                        walk::<EventSprite>,
                        update_event_sprites,
                    )
                        .chain(),
                    water::animate_water,
                    apply_relocate,
                ),
            );
    }
}

/// The fixed 320×240 orthographic projection shared by the world and front
/// cameras — RM2000's native screen. Maps larger than this scroll; the whole view
/// scales to fill the (4:3) window, so tiles are pixel-perfect with no gray margin
/// around a small map.
fn fixed_projection() -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: bevy::camera::ScalingMode::Fixed {
            width: 320.0,
            height: 240.0,
        },
        ..OrthographicProjection::default_2d()
    })
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) {
    // The main camera renders the world on layer 0; its ScreenTone post-process
    // tints only what it draws. Pictures and the UI move to the front camera, so
    // they stay untinted and above the tone.
    commands.spawn((
        Camera2d,
        MainCamera,
        fixed_projection(),
        ScreenTone::default(),
    ));
    // The front camera composites pictures (PICTURE_LAYER) and the UI over the
    // toned world, and owns the default UI camera. `sync_front_camera` keeps its
    // transform matched to the main camera each frame so pictures stay in place.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        fixed_projection(),
        IsDefaultUiCamera,
        RenderLayers::layer(PICTURE_LAYER),
        FrontCamera,
    ));
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
    let map: Map = load_ron(&format!("{}/maps/map_{map_id:04}.ron", asset_root()));
    let chipsets: Vec<Chipset> = load_ron(&format!("{}/chipsets.ron", asset_root()));
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
            // "Above hero" upper tiles (roof/tree/tall-object tops) draw over the
            // hero (z 4 > player z 3) so the hero walks behind them; ordinary
            // upper tiles stay below the hero at z 1.
            let z = if tiles::above_hero(id, &passages_up) {
                4.0
            } else {
                1.0
            };
            render::spawn_tile(commands, &chipset, source, index as i32, width, offset, z);
        }
    }
    for event in &map.events {
        render::spawn_event_npc(
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

/// Teleport events per each [`RelocateEvent`]: move the logical [`MapEvents`]
/// entry (so collision/touch use the new tile) and, for a graphic-bearing event,
/// snap its [`EventSprite`] tile and transform to the target tile center,
/// cancelling any in-flight move by resetting its [`MoveQueue`]. A graphic-less
/// event has no sprite, so updating only the logical position is correct.
fn apply_relocate(
    mut reader: MessageReader<RelocateEvent>,
    data: Res<MapData>,
    mut map_events: ResMut<MapEvents>,
    mut sprites: Query<(&mut EventSprite, &mut Transform, &mut MoveQueue)>,
) {
    for msg in reader.read() {
        if let Some(event) = map_events.events.iter_mut().find(|e| e.id == msg.event_id) {
            event.x = msg.x;
            event.y = msg.y;
        }
        if let Some((mut sprite, mut transform, mut queue)) =
            sprites.iter_mut().find(|(s, _, _)| s.id == msg.event_id)
        {
            sprite.tile_x = msg.x as i32;
            sprite.tile_y = msg.y as i32;
            *queue = MoveQueue::default();
            let (wx, wy) = data.tile_center(msg.x as i32, msg.y as i32);
            transform.translation =
                Vec3::new(wx, wy + CHAR_Y_OFFSET, tiles::character_z(msg.y as i32));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::DIR_DOWN;

    #[test]
    fn relocate_moves_event_logical_and_visual() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<RelocateEvent>();
        app.add_systems(Update, apply_relocate);
        app.insert_resource(MapData::for_test(10, 10));
        app.insert_resource(MapEvents {
            events: vec![Event {
                id: 5,
                x: 0,
                y: 0,
                name: String::new(),
                pages: Vec::new(),
            }],
        });
        let entity = app
            .world_mut()
            .spawn((
                EventSprite {
                    id: 5,
                    tile_x: 0,
                    tile_y: 0,
                    dir: DIR_DOWN,
                    frame: 1,
                    charset: "C".into(),
                    index: 0,
                },
                Transform::default(),
                MoveQueue::default(),
            ))
            .id();

        app.world_mut().write_message(RelocateEvent {
            event_id: 5,
            x: 3,
            y: 4,
        });
        app.update();

        let sprite = app.world().entity(entity).get::<EventSprite>().unwrap();
        assert_eq!(sprite.tile_x, 3);
        assert_eq!(sprite.tile_y, 4);
        let events = app.world().resource::<MapEvents>();
        let ev = events.events.iter().find(|e| e.id == 5).unwrap();
        assert_eq!(ev.x, 3);
        assert_eq!(ev.y, 4);
    }
}

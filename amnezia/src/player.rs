//! The player hero: spawning, grid movement (blocked by passability and solid
//! events), player-touch event triggers, walk animation, and camera follow.

use crate::assets::resolve_png;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::state::{active_page, Switches, Variables};
use crate::teleport::Fade;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::{MapData, MapEvents};
use amnezia_data::EventPage;
use bevy::prelude::*;

const PLAYER_CHARSET: &str = "Chara1";
const PLAYER_INDEX: u32 = 0;

/// The hero the player controls, tracked in tile coordinates.
#[derive(Component)]
pub struct Player {
    pub tile_x: i32,
    pub tile_y: i32,
    pub dir: u32,
    pub frame: u32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (move_player, update_player_sprite, camera_follow).chain());
    }
}

/// Spawn the hero at `start` (tile) positioned via the map's geometry.
pub fn spawn_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    start: (i32, i32),
    data: &MapData,
) {
    let image = asset_server.load(resolve_png("CharSet", PLAYER_CHARSET));
    let (sx, sy) = tiles::charset_source(PLAYER_INDEX, DIR_DOWN, 1);
    let (world_x, world_y) = data.tile_center(start.0, start.1);
    commands.spawn((
        Player { tile_x: start.0, tile_y: start.1, dir: DIR_DOWN, frame: 1 },
        Sprite {
            image,
            rect: Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H)),
            custom_size: Some(Vec2::new(tiles::CHAR_W, tiles::CHAR_H)),
            ..default()
        },
        Transform::from_xyz(world_x, world_y + CHAR_Y_OFFSET, 3.0),
    ));
}

/// The tile directly in front of the player, given its facing direction.
pub fn facing_tile(player: &Player) -> (i32, i32) {
    let (dx, dy) = match player.dir {
        DIR_UP => (0, -1),
        DIR_RIGHT => (1, 0),
        DIR_DOWN => (0, 1),
        _ => (-1, 0),
    };
    (player.tile_x + dx, player.tile_y + dy)
}

#[allow(clippy::too_many_arguments)]
fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<MapData>,
    dialogue: Res<Dialogue>,
    fade: Res<Fade>,
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    mut running: ResMut<RunningEvent>,
    mut players: Query<&mut Player>,
) {
    if dialogue.active || fade.busy() || running.active() {
        return;
    }
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    let step = if keys.just_pressed(KeyCode::ArrowUp) {
        Some((0, -1, DIR_UP))
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        Some((0, 1, DIR_DOWN))
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        Some((-1, 0, DIR_LEFT))
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some((1, 0, DIR_RIGHT))
    } else {
        None
    };
    if let Some((dx, dy, dir)) = step {
        player.dir = dir;
        let (nx, ny) = (player.tile_x + dx, player.tile_y + dy);
        if nx < 0 || ny < 0 || nx >= data.width || ny >= data.height {
            return;
        }
        let blocked =
            !data.passable(nx, ny) || event_blocks_at(&map_events, &switches, &variables, nx, ny);
        if !blocked {
            player.tile_x = nx;
            player.tile_y = ny;
            player.frame = (player.frame + 1) % 3;
        }
        // A player-touch event fires on the attempt to enter its tile — after a
        // passable step onto it, or in place at a solid one (RM2000 doors/exits
        // are solid). It fires only on input, never on the interpreter's own
        // actions, so there's no re-trigger loop.
        if let Some(page) = touch_page_at(&map_events, &switches, &variables, nx, ny) {
            running.start(page.commands.clone());
        }
    }
}

/// Whether a same-layer event occupies tile `(x, y)` and blocks the player.
/// RM2000 events with `layer == 1` are solid (graphic or not); other layers
/// don't block. The active page (per current switches/variables) decides.
fn event_blocks_at(
    map_events: &MapEvents,
    switches: &Switches,
    variables: &Variables,
    x: i32,
    y: i32,
) -> bool {
    map_events.events.iter().any(|e| {
        e.x as i32 == x
            && e.y as i32 == y
            && active_page(e, switches, variables).is_some_and(|p| p.layer == 1)
    })
}

/// The active page of a player-touch event (trigger 1 or 2) on tile `(x, y)`,
/// if any — the command list the interpreter should run on contact.
fn touch_page_at<'a>(
    map_events: &'a MapEvents,
    switches: &Switches,
    variables: &Variables,
    x: i32,
    y: i32,
) -> Option<&'a EventPage> {
    map_events
        .events
        .iter()
        .filter(|e| e.x as i32 == x && e.y as i32 == y)
        .find_map(|e| {
            active_page(e, switches, variables).filter(|p| p.trigger == 1 || p.trigger == 2)
        })
}

fn update_player_sprite(
    data: Res<MapData>,
    mut players: Query<(&Player, &mut Sprite, &mut Transform), Changed<Player>>,
) {
    for (player, mut sprite, mut transform) in &mut players {
        let (sx, sy) = tiles::charset_source(PLAYER_INDEX, player.dir, player.frame);
        sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
        let (world_x, world_y) = data.tile_center(player.tile_x, player.tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
    }
}

fn camera_follow(
    players: Query<&Transform, With<Player>>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    camera.translation.x = player.translation.x;
    camera.translation.y = player.translation.y;
}

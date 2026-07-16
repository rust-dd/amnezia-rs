//! The player hero: spawning, grid movement (blocked by passability), walk
//! animation, and camera follow.

use crate::assets::resolve_png;
use crate::dialogue::Dialogue;
use crate::events::touch_teleport;
use crate::teleport::PendingTeleport;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::{MapData, MapEvents};
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

/// Spawn the hero at `start` (tile) given the map's world offset.
pub fn spawn_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    start: (i32, i32),
    offset: (f32, f32),
) {
    let image = asset_server.load(resolve_png("CharSet", PLAYER_CHARSET));
    let (sx, sy) = tiles::charset_source(PLAYER_INDEX, DIR_DOWN, 1);
    let world_x = start.0 as f32 * tiles::TILE - offset.0 + tiles::TILE / 2.0;
    let world_y = offset.1 - start.1 as f32 * tiles::TILE - tiles::TILE / 2.0 + CHAR_Y_OFFSET;
    commands.spawn((
        Player { tile_x: start.0, tile_y: start.1, dir: DIR_DOWN, frame: 1 },
        Sprite {
            image,
            rect: Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H)),
            custom_size: Some(Vec2::new(tiles::CHAR_W, tiles::CHAR_H)),
            ..default()
        },
        Transform::from_xyz(world_x, world_y, 3.0),
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
    map_events: Res<MapEvents>,
    mut pending: ResMut<PendingTeleport>,
    mut players: Query<&mut Player>,
) {
    if dialogue.active {
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
        if nx >= 0 && ny >= 0 && nx < data.width && ny < data.height && data.passable(nx, ny) {
            player.tile_x = nx;
            player.tile_y = ny;
            player.frame = (player.frame + 1) % 3;
            queue_touch_teleport(&map_events, &mut pending, nx, ny);
        }
    }
}

/// If the tile stepped onto holds a touch-triggered event that teleports, queue
/// it. Only fires on an actual step, so a teleport landing never re-triggers.
fn queue_touch_teleport(map_events: &MapEvents, pending: &mut PendingTeleport, x: i32, y: i32) {
    for event in &map_events.events {
        if event.x as i32 == x
            && event.y as i32 == y
            && let Some(target) = touch_teleport(event)
        {
            pending.0 = Some(target);
        }
    }
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

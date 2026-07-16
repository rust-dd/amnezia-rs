//! Map transfer: when a teleport is requested, despawn the current map's scene
//! entities, load the destination map, and move the (persistent) hero to the
//! target tile.

use crate::player::Player;
use crate::tiles::CHAR_Y_OFFSET;
use crate::world::{load_map, MapData, MapEvents, MapScene};
use bevy::prelude::*;

/// A pending teleport `(map_id, x, y)`, set by an interaction or a touch, and
/// consumed by [`apply_teleport`]. At most one is queued at a time.
#[derive(Resource, Default)]
pub struct PendingTeleport(pub Option<(u32, u32, u32)>);

pub struct TeleportPlugin;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingTeleport>().add_systems(Update, apply_teleport);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_teleport(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingTeleport>,
    mut map_data: ResMut<MapData>,
    mut map_events: ResMut<MapEvents>,
    scene: Query<Entity, With<MapScene>>,
    mut players: Query<(&mut Player, &mut Transform)>,
) {
    let Some((map_id, x, y)) = pending.0.take() else {
        return;
    };
    for entity in &scene {
        commands.entity(entity).despawn();
    }
    let (data, events) = load_map(&mut commands, &asset_server, map_id);
    let (tile_x, tile_y) = (x as i32, y as i32);
    if let Ok((mut player, mut transform)) = players.single_mut() {
        player.tile_x = tile_x;
        player.tile_y = tile_y;
        let (world_x, world_y) = data.tile_center(tile_x, tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
    }
    *map_data = data;
    *map_events = events;
}

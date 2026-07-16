//! Map transfer: when a teleport is requested, despawn the current map's scene
//! entities, load the destination map, and move the (persistent) hero to the
//! target tile.

use crate::player::Player;
use crate::world::{load_map, MapScene};
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

fn apply_teleport(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingTeleport>,
    scene: Query<Entity, With<MapScene>>,
    mut players: Query<&mut Player>,
) {
    let Some((map_id, x, y)) = pending.0.take() else {
        return;
    };
    for entity in &scene {
        commands.entity(entity).despawn();
    }
    load_map(&mut commands, &asset_server, map_id);
    if let Ok(mut player) = players.single_mut() {
        player.tile_x = x as i32;
        player.tile_y = y as i32;
    }
}

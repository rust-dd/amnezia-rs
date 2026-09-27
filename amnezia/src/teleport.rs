//! Map transfers share the event transition state, preserving explicit erasure.

use crate::player::{CameraPan, Player};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::tiles::CHAR_Y_OFFSET;
use crate::transitions::{Kind, TransitionIo};
use crate::world::{
    Character, MapChanged, MapData, MapEvents, MapRebuilt, MapScene, MoveQueue, RouteStepper,
    load_map,
};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct PendingTeleport(pub Option<(u32, u32, u32)>, bool);

impl PendingTeleport {
    /// Rebuild the destination even when a save or new game uses the current map.
    pub fn reload(&mut self, map_id: u32, x: u32, y: u32) {
        self.0 = Some((map_id, x, y));
        self.1 = true;
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum Phase {
    #[default]
    Idle,
    Out,
    Prepare,
    In,
}

/// Map-transfer progress, including rebuilding the scene between transitions.
#[derive(Resource, Default)]
pub struct Fade {
    phase: Phase,
    target: Option<(u32, u32, u32)>,
    reload: bool,
}

impl Fade {
    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }
}

pub struct TeleportPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MapTransfer;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingTeleport>()
            .init_resource::<Fade>()
            .add_message::<MapRebuilt>()
            .add_systems(
                Update,
                drive_fade
                    .in_set(MapTransfer)
                    .before(crate::interpreter::InterpreterStep),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_fade(
    mut transition: TransitionIo,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut pending: ResMut<PendingTeleport>,
    mut fade: ResMut<Fade>,
    mut map_data: ResMut<MapData>,
    mut map_events: ResMut<MapEvents>,
    mut pan: ResMut<CameraPan>,
    mut map_changed: MessageWriter<MapChanged>,
    mut map_rebuilt: MessageWriter<MapRebuilt>,
    scene: Query<Entity, With<MapScene>>,
    mut players: Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
) {
    if transition.state.busy() {
        return;
    }
    let now = transition.frames.frame;
    let center = IVec2::new(160, 120);
    match fade.phase {
        Phase::Idle => {
            let Some(target) = pending.0.take() else {
                return;
            };
            fade.target = Some(target);
            fade.reload = std::mem::take(&mut pending.1);
            let kind = if fade.reload {
                Kind::Fade
            } else {
                transition.kind(0)
            };
            if !transition.state.erased() {
                transition.state.start(kind, true, now, center);
            }
            fade.phase = Phase::Out;
        }
        Phase::Out => {
            if let Some((map_id, x, y)) = fade.target.take() {
                let rebuilt = swap_map(
                    &mut commands,
                    &asset_server,
                    &switches,
                    &variables,
                    &party,
                    &inventory,
                    &mut map_data,
                    &mut map_events,
                    &mut pan,
                    &scene,
                    &mut players,
                    map_id,
                    x,
                    y,
                    fade.reload,
                );
                commands.queue(move |world: &mut World| {
                    if let Some(mut vehicles) =
                        world.get_resource_mut::<crate::vehicles::Vehicles>()
                        && let Some(index) = vehicles.save.riding
                    {
                        vehicles.set_location(index, map_id, x, y);
                    }
                    if let Some(mut calling) = world.get_resource_mut::<crate::menu::Calling>() {
                        calling.cancel();
                    }
                });
                if rebuilt {
                    map_rebuilt.write(MapRebuilt);
                }
                map_changed.write(MapChanged);
            }
            fade.phase = Phase::Prepare;
        }
        Phase::Prepare => {
            if !transition.state.event_erased {
                let kind = if fade.reload {
                    Kind::Fade
                } else {
                    transition.kind(1)
                };
                transition.state.start(kind, false, now, center);
            }
            fade.phase = Phase::In;
        }
        Phase::In => fade.phase = Phase::Idle,
    }
}

#[allow(clippy::too_many_arguments)]
fn swap_map(
    commands: &mut Commands,
    asset_server: &AssetServer,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
    map_data: &mut MapData,
    map_events: &mut MapEvents,
    pan: &mut CameraPan,
    scene: &Query<Entity, With<MapScene>>,
    players: &mut Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
    map_id: u32,
    x: u32,
    y: u32,
    reload: bool,
) -> bool {
    let (tile_x, tile_y) = (x as i32, y as i32);
    if map_id == map_data.map_id && !reload {
        reposition_hero(players, map_data, tile_x, tile_y);
        pan.recenter(false);
        return false;
    }
    for entity in scene {
        commands.entity(entity).despawn();
    }
    let (data, events) = load_map(
        commands,
        asset_server,
        switches,
        variables,
        party,
        inventory,
        map_id,
    );
    reposition_hero(players, &data, tile_x, tile_y);
    if map_id != map_data.map_id
        && let Ok((mut player, _, _, mut route)) = players.single_mut()
    {
        route.animation.reset(&mut *player);
    }
    *map_data = data;
    *map_events = events;
    pan.recenter(true);
    true
}

/// Move the persistent hero to tile `(tile_x, tile_y)` on `data`: update its
/// logical tile and snap its transform to the tile center. Facing is retained —
/// the teleport target carries no direction, matching RM2000's "retain heading".
fn reposition_hero(
    players: &mut Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
    data: &MapData,
    tile_x: i32,
    tile_y: i32,
) {
    if let Ok((mut player, mut transform, mut queue, _)) = players.single_mut() {
        queue.relocate(player.tile());
        player.tile_x = tile_x;
        player.tile_y = tile_y;
        let (world_x, world_y) = data.tile_center(tile_x, tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
        transform.translation.z = crate::tiles::character_z(tile_y);
    }
}

#[cfg(test)]
mod transition_tests;

#[cfg(test)]
mod relocation_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// Reposition the single hero to tile (4, 6) — the same-map teleport path.
    fn run_reposition(
        data: Res<MapData>,
        mut players: Query<(
            &mut Player,
            &mut Transform,
            &mut MoveQueue,
            &mut RouteStepper,
        )>,
    ) {
        reposition_hero(&mut players, &data, 4, 6);
    }

    /// A same-map teleport repositions the hero without tearing down the scene:
    /// `reposition_hero` moves the hero's tile and transform and never despawns
    /// the `MapScene` entities (it holds no `Commands`), so the map's generation
    /// is unchanged.
    #[test]
    fn same_map_reposition_moves_hero_and_keeps_scene() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(MapData::for_test(10, 10));
        let scene = (0..3)
            .map(|_| app.world_mut().spawn(MapScene).id())
            .collect::<Vec<_>>();
        let hero = app
            .world_mut()
            .spawn((
                Player {
                    tile_x: 1,
                    tile_y: 1,
                    dir: crate::tiles::DIR_DOWN,
                    frame: 1,
                    charset: "Chara1".into(),
                    index: 0,
                },
                Transform::default(),
                MoveQueue::default(),
                RouteStepper::default(),
            ))
            .id();

        app.add_systems(Update, run_reposition);
        app.update();

        let player = app.world().entity(hero).get::<Player>().unwrap();
        assert_eq!((player.tile_x, player.tile_y), (4, 6));
        let (cx, cy) = app.world().resource::<MapData>().tile_center(4, 6);
        let transform = app.world().entity(hero).get::<Transform>().unwrap();
        assert_eq!(transform.translation.x, cx);
        assert_eq!(transform.translation.y, cy + CHAR_Y_OFFSET);
        for entity in scene {
            assert!(app.world().get_entity(entity).is_ok());
        }
    }
}

//! Map transfers share event transitions; occupied vehicle relocation skips them.

use crate::world::{MapEffectsReset, MapRebuilt};
use bevy::prelude::*;

mod flow;
pub(crate) mod normal_smoke;
pub(crate) mod rebuild;
pub(crate) mod reservation_smoke;
mod scene;
pub(crate) mod smoke;
#[cfg(test)]
use crate::{
    player::{CameraPan, Player},
    state::{Inventory, Party, Switches, Variables},
    tiles::CHAR_Y_OFFSET,
    world::{Character, MapChanged, MapData, MapEvents, MapScene, MoveQueue, RouteStepper},
};
#[cfg(test)]
use scene::reposition_hero;

#[derive(Resource, Default)]
pub struct PendingTeleport(
    pub Option<(u32, u32, u32)>,
    bool,
    Option<(u32, u32, u32)>,
    bool,
);

impl PendingTeleport {
    /// Rebuild the destination even when a save or new game uses the current map.
    pub fn reload(&mut self, map_id: u32, x: u32, y: u32) {
        self.0 = Some((map_id, x, y));
        self.1 = true;
        self.3 = false;
    }

    pub(crate) fn reserve(&mut self, target: (u32, u32, u32), foreground: bool) {
        self.0 = Some(target);
        self.1 = false;
        self.3 = foreground;
    }

    pub(crate) fn quick(&mut self, map_id: u32, x: u32, y: u32) {
        self.2 = Some((map_id, x, y));
    }

    pub(crate) fn reloading(&self) -> bool {
        self.0.is_some() && self.1
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum Phase {
    #[default]
    Idle,
    Out,
    Parallel,
    In,
    Foreground,
}

/// Map-transfer progress, including rebuilding the scene between transitions.
#[derive(Resource, Default)]
pub struct Fade {
    phase: Phase,
    target: Option<(u32, u32, u32)>,
    reload: bool,
    default_show: bool,
    foreground: bool,
}

impl Fade {
    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }
}

pub struct TeleportPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MapTransfer;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TransferCommit;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingTeleport>()
            .init_resource::<Fade>()
            .add_message::<MapRebuilt>()
            .add_message::<MapEffectsReset>()
            .add_systems(
                Update,
                flow::drive
                    .in_set(MapTransfer)
                    .before(crate::interpreter::InterpreterStep),
            );
        crate::timing::logical::post(app, || {
            flow::begin_pending
                .in_set(TransferCommit)
                .after(crate::panorama::PanoramaAdvance)
        });
    }
}

pub(crate) fn flush_quick(world: &mut World) {
    let target = world
        .get_resource_mut::<PendingTeleport>()
        .and_then(|mut pending| pending.2.take());
    let Some(target) = target else {
        return;
    };
    crate::picture::apply_pending(world);
    world.run_system_cached_with(scene::quick, target).unwrap();
    if let Some(mut pool) = world.get_resource_mut::<crate::interpreter::ParallelPool>() {
        pool.enter_map(Some(target.0));
    }
    crate::player::relocate_camera(world);
    crate::map_bgm::flush(world);
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

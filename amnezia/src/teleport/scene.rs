use crate::player::{CameraPan, Player};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::tiles::CHAR_Y_OFFSET;
use crate::world::{
    Character, MapChanged, MapData, MapEffectsReset, MapEvents, MapRebuilt, MapScene, MoveQueue,
    RouteStepper, load_map,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(super) struct Scene<'w, 's> {
    commands: Commands<'w, 's>,
    server: Res<'w, AssetServer>,
    switches: Res<'w, Switches>,
    variables: Res<'w, Variables>,
    party: Res<'w, Party>,
    inventory: Res<'w, Inventory>,
    data: ResMut<'w, MapData>,
    events: ResMut<'w, MapEvents>,
    pan: ResMut<'w, CameraPan>,
    vehicles: Option<ResMut<'w, crate::vehicles::Vehicles>>,
    calling: Option<ResMut<'w, crate::menu::Calling>>,
    changed: MessageWriter<'w, MapChanged>,
    rebuilt: MessageWriter<'w, MapRebuilt>,
    effects: MessageWriter<'w, MapEffectsReset>,
    scene: Query<'w, 's, Entity, With<MapScene>>,
    heroes: Query<
        'w,
        's,
        (
            &'static mut Player,
            &'static mut Transform,
            &'static mut MoveQueue,
            &'static mut RouteStepper,
        ),
    >,
}

impl Scene<'_, '_> {
    pub(super) fn perform(&mut self, target: (u32, u32, u32), reload: bool, quick: bool) {
        let (map_id, x, y) = target;
        let changed = map_id != self.data.map_id;
        if changed || reload {
            for entity in &self.scene {
                self.commands.entity(entity).despawn();
            }
            let (data, events) = load_map(
                &mut self.commands,
                &self.server,
                &self.switches,
                &self.variables,
                &self.party,
                &self.inventory,
                map_id,
            );
            if changed && let Ok((mut hero, _, _, mut route)) = self.heroes.single_mut() {
                route.animation.reset(&mut *hero);
            }
            *self.data = data;
            *self.events = events;
            if quick {
                self.pan.recenter_quick();
            } else {
                self.pan.recenter(true);
                self.effects.write(MapEffectsReset);
            }
            self.rebuilt.write(MapRebuilt);
        } else {
            self.pan.recenter(false);
        }
        reposition_hero(&mut self.heroes, &self.data, x as i32, y as i32);
        if let Some(vehicles) = &mut self.vehicles
            && let Some(index) = vehicles.save.riding
        {
            vehicles.set_location(index, map_id, x, y);
        }
        if let Some(calling) = &mut self.calling {
            calling.cancel();
        }
        self.changed.write(MapChanged);
    }
}

pub(super) fn quick(In(target): In<(u32, u32, u32)>, mut scene: Scene) {
    scene.perform(target, false, true);
}

/// Move the persistent hero to tile `(tile_x, tile_y)` on `data`: update its
/// logical tile and snap its transform to the tile center. Facing is retained —
/// the teleport target carries no direction, matching RM2000's "retain heading".
pub(super) fn reposition_hero(
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

use super::*;
use crate::player::Player;
use crate::world::{Character, TouchEvents};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub(crate) fn character(
    world: &mut World,
    id: u32,
    from: (i32, i32),
    delta: (i32, i32),
    jumping: bool,
) -> bool {
    let to = (from.0 + delta.0, from.1 + delta.1);
    if jumping && delta == (0, 0) {
        return true;
    }
    if !jumping && delta.0 != 0 && delta.1 != 0 {
        let vertical = (from.0, to.1);
        let horizontal = (to.0, from.1);
        return (leg(world, id, from, vertical, false) && leg(world, id, vertical, to, false))
            || (leg(world, id, from, horizontal, false) && leg(world, id, horizontal, to, false));
    }
    leg(world, id, from, to, jumping)
}

fn leg(world: &mut World, id: u32, from: (i32, i32), to: (i32, i32), jumping: bool) -> bool {
    let passage = match world
        .run_system_cached_with(enter, (id, from, to, jumping))
        .unwrap()
    {
        Entry::Blocked => return false,
        Entry::Clear => return true,
        Entry::Check(passage) => passage,
    };
    let mut ids = world
        .resource::<MapEvents>()
        .events
        .iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for other in ids {
        let at_destination = other != id
            && world.resource::<MapEvents>().events.iter().any(|event| {
                event.id == other && (event.x as i32, event.y as i32) == passage.destination
            });
        if !at_destination {
            continue;
        }
        if world.contains_resource::<crate::interpreter::ParallelPool>() {
            // The collision caller ignores nested asynchronous requests.
            crate::interpreter::update_map_event(world, other);
        }
        if world
            .run_system_cached_with(blocks, (id, other, passage))
            .unwrap()
        {
            return false;
        }
    }
    if id != 0
        && !world
            .get_resource::<crate::vehicles::Vehicles>()
            .is_some_and(|vehicles| vehicles.riding())
        && world
            .query::<&Player>()
            .single(world)
            .is_ok_and(|hero| hero.tile() == passage.destination)
    {
        crate::player::update::early(world);
    }
    if world
        .run_system_cached_with(hero_blocks, (id, passage))
        .unwrap()
    {
        return false;
    }
    for index in 0..3 {
        if id == 10002 + index as u32 || (id == 0 && index == 2) {
            continue;
        }
        let map_id = world.resource::<MapData>().map_id;
        let at_destination = world
            .get_resource::<crate::vehicles::Vehicles>()
            .is_some_and(|vehicles| {
                let vehicle = &vehicles.save.vehicles[index];
                vehicle.definition.map_id == map_id && vehicle.tile() == passage.destination
            });
        if !at_destination {
            continue;
        }
        crate::vehicles::early(world, index);
        if world
            .run_system_cached_with(vehicle_blocks, (id, index, passage))
            .unwrap()
        {
            return false;
        }
    }
    world
        .run_system_cached_with(finish_tile, (id, passage))
        .unwrap()
}

#[derive(SystemParam)]
struct Context<'w, 's> {
    data: Res<'w, MapData>,
    events: Res<'w, MapEvents>,
    switches: Res<'w, Switches>,
    variables: Res<'w, Variables>,
    party: Res<'w, Party>,
    inventory: Res<'w, Inventory>,
    characters: Query<'w, 's, (&'static EventSprite, Option<&'static RouteStepper>)>,
    players: Query<'w, 's, (&'static Player, Option<&'static RouteStepper>)>,
    vehicles: Option<Res<'w, crate::vehicles::Vehicles>>,
}

impl Context<'_, '_> {
    fn with<T>(
        &self,
        id: u32,
        f: impl FnOnce(&MapCollision, Mover, Option<(i32, i32)>) -> T,
    ) -> Option<T> {
        let hero = self.players.single().ok();
        let mover = if id == 0 {
            let (_, route) = hero?;
            Mover::hero(route.is_some_and(RouteStepper::through))
        } else if let Some(index) = vehicle_index(id) {
            Mover::vehicle(index, self.vehicles.as_ref()?.route_through(index))
        } else {
            let (character, route) = self.characters.iter().find(|(event, _)| event.id == id)?;
            Mover::event(character, route.is_some_and(RouteStepper::through))
        };
        let mut bodies = CollisionBodies::from_events(self.characters.iter());
        bodies.hero_through = hero
            .and_then(|(_, route)| route)
            .is_some_and(RouteStepper::through);
        bodies.include_vehicles(self.vehicles.as_deref(), self.data.map_id);
        let collision = MapCollision::new(
            &self.data,
            &self.events,
            (
                &self.switches,
                &self.variables,
                &self.party,
                &self.inventory,
            ),
            &bodies,
        );
        Some(f(&collision, mover, hero.map(|(hero, _)| hero.tile())))
    }
}

#[allow(clippy::type_complexity)]
fn enter(
    In((id, from, to, jumping)): In<(u32, (i32, i32), (i32, i32), bool)>,
    context: Context,
) -> Entry {
    context
        .with(id, |collision, mover, _| {
            collision.enter(from, to, mover, jumping)
        })
        .unwrap_or(Entry::Blocked)
}

fn blocks(In((id, other, passage)): In<(u32, u32, Passage)>, context: Context) -> bool {
    context
        .with(id, |collision, mover, _| {
            collision
                .events
                .events
                .iter()
                .find(|event| event.id == other)
                .is_some_and(|event| {
                    collision.event_blocks(mover, event, passage.destination, passage.self_conflict)
                })
        })
        .unwrap_or(false)
}

fn hero_blocks(In((id, passage)): In<(u32, Passage)>, context: Context) -> bool {
    context
        .with(id, |collision, mover, hero| {
            collision.hero_blocks(mover, hero, passage)
        })
        .unwrap_or(false)
}

fn vehicle_blocks(In((id, index, passage)): In<(u32, usize, Passage)>, context: Context) -> bool {
    context
        .with(id, |collision, mover, _| {
            collision.vehicle_blocks(mover, index, passage)
        })
        .unwrap_or(false)
}

fn finish_tile(In((id, passage)): In<(u32, Passage)>, context: Context) -> bool {
    context
        .with(id, |collision, mover, _| {
            collision.finish_tile(mover, passage)
        })
        .unwrap_or(false)
}

pub(in crate::world) fn failed_walk(world: &mut World, id: u32) {
    if world.contains_resource::<TouchEvents>() {
        world.run_system_cached_with(contact, id).unwrap();
        world
            .run_system_cached_with(crate::world::touch::trigger_event, (Some(id), false))
            .unwrap();
    }
}

fn contact(In(id): In<u32>, context: Context, mut touches: ResMut<TouchEvents>) {
    let Some((character, route)) = context.characters.iter().find(|(event, _)| event.id == id)
    else {
        return;
    };
    let direction = route.map_or(character.dir, |route| route.direction(character));
    // The original failure hook computes a front tile only for cardinal directions.
    let delta = if direction < 4 {
        crate::world::dir_delta(direction)
    } else {
        (0, 0)
    };
    let front = context
        .data
        .normalize_tile(character.tile_x + delta.0, character.tile_y + delta.1);
    if character.layer == 1
        && context
            .players
            .single()
            .is_ok_and(|(hero, _)| hero.tile() == front)
    {
        touches.0.push(id);
    }
}

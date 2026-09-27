use super::*;
mod route;

#[derive(Resource, Default)]
pub(super) struct Updates(pub(super) [bool; 3]);

pub(crate) fn begin_update(world: &mut World) {
    let Some(data) = world.get_resource::<MapData>() else {
        return;
    };
    let Some(vehicles) = world.get_resource::<Vehicles>() else {
        return;
    };
    let active = std::array::from_fn::<_, 3, _>(|index| {
        vehicles.save.vehicles[index].definition.map_id == data.map_id
    });
    if let Some(mut updates) = world.get_resource_mut::<Updates>() {
        for (processed, active) in updates.0.iter_mut().zip(active) {
            if active {
                *processed = false;
            }
        }
    }
}

fn claim(world: &mut World, index: usize) -> bool {
    world
        .get_resource_mut::<Updates>()
        .is_none_or(|mut updates| !std::mem::replace(&mut updates.0[index], true))
}

pub(crate) fn early(world: &mut World, index: usize) {
    if world.contains_resource::<Updates>()
        && world.run_system_cached_with(eligible, index).unwrap()
    {
        character(world, index);
    }
}

fn eligible(
    In(index): In<usize>,
    data: Res<MapData>,
    vehicles: Res<Vehicles>,
    guards: MoveGuards,
) -> bool {
    !guards.forced_route_paused() && vehicles.save.vehicles[index].definition.map_id == data.map_id
}

pub(super) fn advance(world: &mut World) {
    if !world.run_system_cached(begin).unwrap() {
        return;
    };
    for index in 0..3 {
        character(world, index);
    }
}

fn character(world: &mut World, index: usize) {
    if world.resource::<Vehicles>().save.vehicles[index]
        .definition
        .map_id
        != world.resource::<MapData>().map_id
        || !claim(world, index)
    {
        return;
    }
    route::advance(world, index);
    world.run_system_cached_with(finish, index).unwrap();
}

fn begin(guards: MoveGuards) -> bool {
    !guards.forced_route_paused()
}

fn finish(
    In(index): In<usize>,
    time: Res<Time>,
    data: Res<MapData>,
    stops: crate::world::stop_clock::StopGates,
    mut vehicles: ResMut<Vehicles>,
) {
    let vehicles = &mut *vehicles;
    let vehicle = &mut vehicles.save.vehicles[index];
    let motion = &mut vehicles.motion[index];
    if vehicle.definition.map_id != data.map_id {
        return;
    }
    let moving = motion.queue.busy();
    motion
        .queue
        .use_character_motion(vehicle.speed, motion.route.direction(vehicle));
    if let Some(pixel) = motion.queue.advance(vehicle, &data, time.delta_secs()) {
        motion.pixel = Some(pixel);
    }
    if moving && !motion.queue.busy() {
        motion.route.settle_movement();
    }
    motion
        .route
        .advance_stop_clock(moving, stops.advances(None));
    let walking = motion.route.stop_count() == 0;
    let animated = !motion.queue.jumping() && (index != 2 || vehicles.save.riding == Some(2));
    motion
        .route
        .animation
        .advance_vehicle(vehicle, animated, walking, time.delta_secs());
}

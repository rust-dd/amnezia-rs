use super::*;
use crate::world::{RouteAttempt, RouteBoundary, RouteProgress, RouteTurn};

pub(super) fn advance(world: &mut World, index: usize) {
    let mut turn = None;
    while let Some((current, boundary)) = world
        .run_system_cached_with(prepare, (index, turn))
        .unwrap()
    {
        match boundary {
            RouteBoundary::Ready(RouteProgress::Refresh) => {
                crate::world::update::refresh_route_switch(world);
                turn = Some(current);
            }
            RouteBoundary::Attempt(attempt) => {
                let success = crate::world::collision::make_way(
                    world,
                    10002 + index as u32,
                    attempt.origin,
                    attempt.delta,
                    attempt.jumping,
                );
                let (current, done) = world
                    .run_system_cached_with(resolve, (index, current, attempt, success))
                    .unwrap();
                if done {
                    break;
                }
                turn = Some(current);
            }
            RouteBoundary::Ready(RouteProgress::Done) => break,
            RouteBoundary::Ready(RouteProgress::Move(..)) => unreachable!(),
        }
    }
}

fn prepare(
    In((index, turn)): In<(usize, Option<RouteTurn>)>,
    data: Res<MapData>,
    mut vehicles: ResMut<Vehicles>,
    mut switches: ResMut<Switches>,
    mut audio: MessageWriter<AudioRequest>,
    players: Query<&Player>,
) -> Option<(RouteTurn, RouteBoundary)> {
    let vehicles = &mut *vehicles;
    let vehicle = &mut vehicles.save.vehicles[index];
    let motion = &mut vehicles.motion[index];
    if vehicle.definition.map_id != data.map_id
        || motion.queue.busy()
        || (turn.is_none() && (motion.route.settle_movement() || !motion.route.active()))
        || motion.route.stop_active()
    {
        return None;
    }
    let origin = vehicle.tile();
    let target = players.single().map_or(origin, Character::tile);
    let delta = data.tile_delta(origin, target);
    let mut turn = turn.unwrap_or_else(|| RouteTurn::new(&mut motion.route));
    let mut effects = Vec::new();
    let boundary = motion.route.prepare_turn(
        vehicle,
        (origin.0 + delta.0, origin.1 + delta.1),
        &mut effects,
        &mut turn,
    );
    vehicle.speed = motion.route.speed();
    if let RouteBoundary::Attempt(attempt) = &boundary {
        motion.queue.set_jump_attempt(attempt.jumping);
    }
    for effect in effects {
        match effect {
            StepEffect::Switch(id, on) => switches.set(id, on),
            StepEffect::Sound { name, params } => {
                audio.write(AudioRequest::play_sound(&name, &params));
            }
            StepEffect::Transparency(level) => {
                motion.alpha = crate::tiles::character_alpha(level);
            }
        }
    }
    Some((turn, boundary))
}

fn resolve(
    In((index, mut turn, attempt, success)): In<(usize, RouteTurn, RouteAttempt, bool)>,
    data: Res<MapData>,
    mut vehicles: ResMut<Vehicles>,
) -> (RouteTurn, bool) {
    let vehicles = &mut *vehicles;
    let vehicle = &mut vehicles.save.vehicles[index];
    let motion = &mut vehicles.motion[index];
    motion.queue.set_jump_attempt(false);
    let progress = motion
        .route
        .resolve_turn(vehicle, attempt, success, &mut turn);
    vehicle.speed = motion.route.speed();
    let done = progress.is_some();
    if let Some(RouteProgress::Move(action, seconds)) = progress {
        motion.queue.set_step_secs(seconds);
        motion
            .queue
            .begin_from(vehicle, &data, attempt.origin, action);
    }
    (turn, done)
}

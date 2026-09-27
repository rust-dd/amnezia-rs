use super::*;
use crate::world::{RouteTurn, drive_route_part};

pub(super) fn advance(world: &mut World) {
    let Some(transitioning) = world.run_system_cached(begin).unwrap() else {
        return;
    };
    for index in 0..3 {
        if index != 2 || !transitioning {
            let mut turn = None;
            loop {
                turn = world.run_system_cached_with(part, (index, turn)).unwrap();
                if turn.is_none() {
                    break;
                }
                crate::world::update::refresh_route_switch(world);
            }
        }
        world
            .run_system_cached_with(finish, (index, transitioning))
            .unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
fn begin(
    time: Res<Time>,
    data: Res<MapData>,
    guards: MoveGuards,
    mut vehicles: ResMut<Vehicles>,
    switches: Res<Switches>,
    mut audio: MessageWriter<AudioRequest>,
    map_events: Res<MapEvents>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) -> Option<bool> {
    if guards.forced_route_paused() {
        return None;
    }
    let transitioning = vehicles.airship_transitioning();
    if vehicles.advance_flight(time.delta_secs(), &data, |x, y| {
        map_events.events.iter().any(|event| {
            (event.x as i32, event.y as i32) == (x, y)
                && active_page(event, &switches, &variables, &party, &inventory).is_some()
        })
    }) {
        audio.write(
            vehicles
                .save
                .before_music
                .as_ref()
                .map_or(AudioRequest::StopBgm, |music| music.replay()),
        );
    }
    Some(transitioning)
}

fn part(
    In((index, turn)): In<(usize, Option<RouteTurn>)>,
    data: Res<MapData>,
    mut vehicles: ResMut<Vehicles>,
    mut switches: ResMut<Switches>,
    mut audio: MessageWriter<AudioRequest>,
) -> Option<RouteTurn> {
    let vehicles = &mut *vehicles;
    let vehicle = &mut vehicles.save.vehicles[index];
    let motion = &mut vehicles.motion[index];
    if vehicle.definition.map_id != data.map_id {
        return None;
    }
    let (x, y) = vehicle.tile();
    let routed = motion.route.active();
    let (driven, turn) = drive_route_part(
        vehicle,
        &mut motion.queue,
        &mut motion.route,
        (x, y),
        |_, dx, dy, _, through| {
            data.contains_tile(x + dx, y + dy)
                && (through || index != 2 || data.airship_passable(x + dx, y + dy))
        },
        turn,
    );
    if routed {
        vehicle.speed = motion.route.speed();
    }
    for effect in driven.effects {
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
    turn
}

fn finish(
    In((index, transitioning)): In<(usize, bool)>,
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
    if index == 2 && transitioning {
        motion.route.animation.advance_vehicle(
            vehicle,
            vehicles.save.riding == Some(2),
            false,
            time.delta_secs(),
        );
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

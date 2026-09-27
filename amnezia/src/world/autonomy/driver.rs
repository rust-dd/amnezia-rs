use super::decisions::Decision;
use super::*;
use crate::world::Character;

struct Turn {
    previous: u32,
    move_type: u32,
    reverse: Option<u32>,
    origin: (i32, i32),
    delta: (i32, i32),
}

pub(in crate::world) fn advance_event(world: &mut World, target: Option<u32>) {
    let mut ids = world
        .query::<&EventSprite>()
        .iter(world)
        .map(|event| event.id)
        .filter(|id| target.is_none_or(|target| target == *id))
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for id in ids {
        let mut turn = world.run_system_cached_with(prepare, id).unwrap();
        while let Some(current) = turn {
            let moved =
                crate::world::collision::make_way(world, id, current.origin, current.delta, false);
            if !moved {
                crate::world::collision::failed_walk(world, id);
            }
            turn = world
                .run_system_cached_with(finish, (id, current, moved))
                .unwrap();
        }
    }
}

fn prepare(
    In(id): In<u32>,
    data: Res<MapData>,
    guards: MoveGuards,
    players: Query<&Player>,
    cameras: Query<&Transform, With<crate::world::MainCamera>>,
    mut movers: Query<(
        &mut EventSprite,
        &MoveQueue,
        &mut AutoMove,
        &mut RouteStepper,
    )>,
) -> Option<Turn> {
    let hero = players.single().ok()?.tile();
    let (mut character, queue, mut auto, mut route) =
        movers.iter_mut().find(|(event, ..)| event.id == id)?;
    if guards.autonomous_paused(id)
        || !matches!(auto.move_type, 1..=5)
        || queue.busy()
        || route.active()
        || route.stop_active()
    {
        return None;
    }
    auto.frequency = route.frequency();
    auto.speed = route.speed();
    let origin = character.tile();
    let camera = cameras
        .single()
        .map_or(Vec2::ZERO, |transform| transform.translation.truncate());
    let visible =
        decisions::visible(data.screen_position(queue.render_position(&*character, &data), camera));
    let previous = route.direction(&*character);
    let decision = auto.decide(
        previous,
        data.tile_delta(origin, hero),
        visible,
        route.stop_maximum(),
    );
    let (direction, reverse) = match decision {
        Decision::Idle(count) => {
            route.set_stop_count(count);
            return None;
        }
        Decision::Move(direction) => (direction, None),
        Decision::Cycle(direction) => (direction, Some(super::reverse(direction))),
    };
    route.set_direction(&mut *character, direction);
    Some(Turn {
        previous,
        move_type: auto.move_type,
        reverse,
        origin,
        delta: dir_delta(direction),
    })
}

fn finish(
    In((id, mut turn, moved)): In<(u32, Turn, bool)>,
    data: Res<MapData>,
    mut events: ResMut<MapEvents>,
    running: Res<RunningEvent>,
    mut movers: Query<(
        &mut EventSprite,
        &mut MoveQueue,
        &mut AutoMove,
        &mut RouteStepper,
    )>,
) -> Option<Turn> {
    let (mut character, mut queue, mut auto, mut route) =
        movers.iter_mut().find(|(event, ..)| event.id == id)?;
    if !moved
        && route.stop_count() >= route.stop_maximum() + 20
        && let Some(direction) = turn.reverse.take()
    {
        route.set_direction(&mut *character, direction);
        turn.origin = character.tile();
        turn.delta = dir_delta(direction);
        return Some(turn);
    }
    if moved {
        let (dx, dy) = turn.delta;
        let face = character.dir;
        queue.set_step_secs(step_secs_for_speed(route.speed()));
        queue.begin_from(
            &mut *character,
            &data,
            turn.origin,
            RouteAction::Step { dx, dy, face },
        );
        if let Some(event) = events.events.iter_mut().find(|event| event.id == id) {
            event.x = character.tile_x as u32;
            event.y = character.tile_y as u32;
        }
    } else if running.event_waiting(id) || route.stop_count() >= route.stop_maximum() + 60 {
        route.set_stop_count(0);
    } else {
        route.restore_retry_direction(&mut *character, turn.previous);
    }
    auto.set_stop_maximum_for(turn.move_type, &mut route);
    None
}

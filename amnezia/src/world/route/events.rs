use super::*;
use stepper::{Attempt, Boundary, Progress};

pub(in crate::world) fn route_event(world: &mut World, target: Option<u32>, forced: Option<bool>) {
    let mut ids = world
        .query::<&EventSprite>()
        .iter(world)
        .map(|event| event.id)
        .filter(|&id| target.is_none_or(|target| target == id))
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for id in ids {
        let mut turn = None;
        while let Some((current, boundary)) = world
            .run_system_cached_with(part, (id, forced, turn))
            .unwrap()
        {
            match boundary {
                Boundary::Ready(Progress::Refresh) => {
                    crate::world::update::refresh_route_switch(world);
                    turn = Some(current);
                }
                Boundary::Attempt(attempt) => {
                    let success = crate::world::collision::make_way(
                        world,
                        id,
                        attempt.origin,
                        attempt.delta,
                        attempt.jumping,
                    );
                    if !success && !attempt.jumping {
                        crate::world::collision::failed_walk(world, id);
                    }
                    let (current, done) = world
                        .run_system_cached_with(resolve, (id, current, attempt, success))
                        .unwrap();
                    if done {
                        break;
                    }
                    turn = Some(current);
                }
                Boundary::Ready(Progress::Done) => break,
                Boundary::Ready(Progress::Move(..)) => unreachable!(),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn part(
    In((target, forced, turn)): In<(u32, Option<bool>, Option<Turn>)>,
    data: Res<MapData>,
    mut switches: ResMut<Switches>,
    guards: crate::world::MoveGuards,
    mut audio: MessageWriter<AudioRequest>,
    players: Query<&Player>,
    mut movers: Query<(
        &mut EventSprite,
        &mut MoveQueue,
        &mut RouteStepper,
        &mut Sprite,
    )>,
) -> Option<(Turn, Boundary)> {
    if turn.is_none() && guards.forced_route_paused() {
        return None;
    }
    let (mut character, mut queue, mut stepper, mut sprite) =
        movers.iter_mut().find(|(event, ..)| event.id == target)?;
    if queue.busy() {
        return None;
    }
    if turn.is_none()
        && (forced.is_some_and(|forced| forced != stepper.forced())
            || (!stepper.forced() && guards.autonomous_paused(target))
            || stepper.settle_movement()
            || !stepper.active())
    {
        return None;
    }
    if stepper.stop_active() {
        return None;
    }
    let hero = players.single().map_or((-1, -1), Character::tile);
    let (x, y) = character.tile();
    let delta = data.tile_delta((x, y), hero);
    let mut turn = turn.unwrap_or_else(|| Turn::new(&mut stepper));
    let mut effects = Vec::new();
    let boundary = stepper.prepare_turn(
        &mut *character,
        (x + delta.0, y + delta.1),
        &mut effects,
        &mut turn,
    );
    if let Boundary::Attempt(attempt) = &boundary {
        queue.set_jump_attempt(attempt.jumping);
    }
    apply_effects(effects, &mut switches, &mut audio, &mut sprite);
    Some((turn, boundary))
}

fn resolve(
    In((id, mut turn, attempt, success)): In<(u32, Turn, Attempt, bool)>,
    data: Res<MapData>,
    mut events: ResMut<MapEvents>,
    mut movers: Query<(&mut EventSprite, &mut MoveQueue, &mut RouteStepper)>,
) -> (Turn, bool) {
    let Some((mut character, mut queue, mut stepper)) =
        movers.iter_mut().find(|(event, ..)| event.id == id)
    else {
        return (turn, true);
    };
    queue.set_jump_attempt(false);
    let progress = stepper.resolve_turn(&mut *character, attempt, success, &mut turn);
    let done = progress.is_some();
    if let Some(Progress::Move(action, seconds)) = progress {
        queue.set_step_secs(seconds);
        queue.begin_from(&mut *character, &data, attempt.origin, action);
        if let Some(event) = events.events.iter_mut().find(|event| event.id == id) {
            event.x = character.tile_x.max(0) as u32;
            event.y = character.tile_y.max(0) as u32;
        }
    }
    (turn, done)
}

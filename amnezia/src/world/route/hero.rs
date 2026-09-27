use super::*;
use stepper::{Attempt, Boundary, Progress};

pub(in crate::world) fn route(world: &mut World) {
    let mut turn = None;
    while let Some((current, boundary)) = world.run_system_cached_with(part, turn).unwrap() {
        match boundary {
            Boundary::Ready(Progress::Refresh) => {
                crate::world::update::refresh_route_switch(world);
                turn = Some(current);
            }
            Boundary::Attempt(attempt) => {
                let success = crate::world::collision::make_way(
                    world,
                    0,
                    attempt.origin,
                    attempt.delta,
                    attempt.jumping,
                );
                let (current, done) = world
                    .run_system_cached_with(resolve, (current, attempt, success))
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

#[allow(clippy::type_complexity)]
fn part(
    In(turn): In<Option<Turn>>,
    mut switches: ResMut<Switches>,
    guards: crate::world::MoveGuards,
    mut audio: MessageWriter<AudioRequest>,
    mut heroes: Query<
        (&mut Player, &mut MoveQueue, &mut RouteStepper, &mut Sprite),
        Without<EventSprite>,
    >,
    mut vehicles: Option<ResMut<crate::vehicles::Vehicles>>,
) -> Option<(Turn, Boundary)> {
    if turn.is_none()
        && (guards.forced_route_paused() || vehicles.as_ref().is_some_and(|v| v.blocks_movement()))
    {
        return None;
    }
    let (mut hero, mut queue, mut stepper, mut sprite) = heroes.single_mut().ok()?;
    if queue.busy()
        || (turn.is_none() && (stepper.settle_movement() || !stepper.active()))
        || stepper.stop_active()
    {
        return None;
    }
    let position = hero.tile();
    let mut turn = turn.unwrap_or_else(|| Turn::new(&mut stepper));
    let mut effects = Vec::new();
    let mut character = crate::vehicles::rider::Graphic {
        hero: &mut hero,
        vehicles: vehicles.as_deref_mut(),
    };
    let boundary = stepper.prepare_turn(&mut character, position, &mut effects, &mut turn);
    if let Boundary::Attempt(attempt) = &boundary {
        queue.set_jump_attempt(attempt.jumping);
    }
    apply_effects(effects, &mut switches, &mut audio, &mut sprite);
    Some((turn, boundary))
}

fn resolve(
    In((mut turn, attempt, success)): In<(Turn, Attempt, bool)>,
    data: Res<MapData>,
    mut heroes: Query<(&mut Player, &mut MoveQueue, &mut RouteStepper)>,
) -> (Turn, bool) {
    let Ok((mut hero, mut queue, mut stepper)) = heroes.single_mut() else {
        return (turn, true);
    };
    queue.set_jump_attempt(false);
    let progress = stepper.resolve_turn(&mut *hero, attempt, success, &mut turn);
    let done = progress.is_some();
    if let Some(Progress::Move(action, seconds)) = progress {
        queue.set_step_secs(seconds);
        queue.begin_from(&mut *hero, &data, attempt.origin, action);
    }
    (turn, done)
}

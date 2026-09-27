use super::*;

struct Attempt {
    origin: (i32, i32),
    delta: (i32, i32),
}

pub(super) fn move_player(world: &mut World) {
    let Some(attempt) = world.run_system_cached(prepare).unwrap() else {
        return;
    };
    let success = crate::world::collision::make_way(world, 0, attempt.origin, attempt.delta, false);
    world
        .run_system_cached_with(resolve, (attempt, success))
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn prepare(
    keys: Res<ButtonInput<KeyCode>>,
    prompts: crate::dialogue::InputPrompts,
    mut triggers: EventTriggers,
    dialogue: Res<Dialogue>,
    scene: ScenePause,
    mut running: ResMut<RunningEvent>,
    mut phase: ResMut<InputPhase>,
    mut players: Query<(&mut Player, &mut MoveQueue, &mut RouteStepper), Without<EventSprite>>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut calling: Option<ResMut<crate::menu::Calling>>,
) -> Option<Attempt> {
    phase.blocked = true;
    let (mut player, queue, mut stepper) = players.single_mut().ok()?;
    if scene.paused()
        || queue.busy()
        || vehicles
            .as_ref()
            .is_some_and(|vehicles| vehicles.blocks_movement())
    {
        return None;
    }
    if running.active() {
        if let Some(calling) = &mut calling {
            calling.cancel();
        }
        return None;
    }
    if dialogue.active || prompts.active() || stepper.active() {
        return None;
    }
    if calling.as_mut().is_some_and(|calling| calling.consume()) {
        stepper.animation.reset(&mut *player);
        return None;
    }
    if !scene.airship() {
        let hero = player.tile();
        triggers.queue_at(&mut running, hero, false, &[2], hero, false);
    }
    if running.waiting() {
        return None;
    }
    phase.blocked = false;
    let direction = [
        (KeyCode::ArrowUp, DIR_UP),
        (KeyCode::ArrowDown, DIR_DOWN),
        (KeyCode::ArrowLeft, DIR_LEFT),
        (KeyCode::ArrowRight, DIR_RIGHT),
    ]
    .into_iter()
    .find_map(|(key, dir)| keys.pressed(key).then_some(dir))?;
    stepper.set_direction(&mut *player, direction);
    Some(Attempt {
        origin: player.tile(),
        delta: crate::world::dir_delta(direction),
    })
}

fn resolve(
    In((attempt, success)): In<(Attempt, bool)>,
    mut triggers: EventTriggers,
    mut running: ResMut<RunningEvent>,
    mut players: Query<(&mut Player, &mut MoveQueue, &RouteStepper), Without<EventSprite>>,
    steps: Option<ResMut<crate::conditions::FieldSteps>>,
) {
    let Ok((mut player, mut queue, stepper)) = players.single_mut() else {
        return;
    };
    if success {
        queue.set_step_secs(crate::world::step_secs_for_speed(stepper.speed()));
        let face = player.dir;
        queue.begin_from(
            &mut *player,
            &triggers.data,
            attempt.origin,
            RouteAction::Step {
                dx: attempt.delta.0,
                dy: attempt.delta.1,
                face,
            },
        );
    } else if !queue.busy() {
        let direction = stepper.direction(&*player);
        let delta = if direction < 4 {
            crate::world::dir_delta(direction)
        } else {
            (0, 0)
        };
        let front = triggers
            .data
            .normalize_tile(player.tile_x + delta.0, player.tile_y + delta.1);
        triggers.queue_at(&mut running, front, true, &[1, 2], player.tile(), false);
    }
    if queue.busy()
        && let Some(mut steps) = steps
    {
        steps.record();
    }
}

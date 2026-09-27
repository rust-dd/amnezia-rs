use super::decisions::Decision;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn advance_event(
    In(target): In<Option<u32>>,
    data: Res<MapData>,
    mut map_events: ResMut<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    guards: MoveGuards,
    mut touches: Option<ResMut<crate::world::TouchEvents>>,
    players: Query<(&Player, Option<&RouteStepper>), Without<EventSprite>>,
    cameras: Query<&Transform, With<crate::world::MainCamera>>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut movers: Query<
        (
            &mut EventSprite,
            &mut MoveQueue,
            &mut AutoMove,
            &mut RouteStepper,
        ),
        Without<Player>,
    >,
) {
    let Ok((player, hero_route)) = players.single() else {
        return;
    };
    let (px, py) = (player.tile_x, player.tile_y);
    let camera = cameras
        .single()
        .map_or(Vec2::ZERO, |transform| transform.translation.truncate());
    let mut bodies = CollisionBodies::from_events(
        movers
            .iter()
            .map(|(event, _, _, route)| (event, Some(route))),
    );
    bodies.hero_through = hero_route.is_some_and(RouteStepper::through);
    bodies.include_vehicles(vehicles.as_deref(), data.map_id);
    for (mut sprite, mut queue, mut auto, mut stepper) in &mut movers {
        if target.is_some_and(|id| id != sprite.id) {
            continue;
        }
        if guards.autonomous_paused(sprite.id)
            || !matches!(auto.move_type, 1..=5)
            || queue.busy()
            || stepper.active()
        {
            continue;
        }
        auto.frequency = stepper.frequency();
        auto.speed = stepper.speed();
        if stepper.stop_active() {
            continue;
        }

        let (ex, ey) = (sprite.tile_x, sprite.tile_y);
        let self_id = sprite.id;
        let hero_delta = data.tile_delta((ex, ey), (px, py));
        let previous = stepper.direction(&*sprite);
        let visible = decisions::visible(
            data.screen_position(queue.render_position(&*sprite, &data), camera),
        );
        let decision = auto.decide(previous, hero_delta, visible, stepper.stop_maximum());
        let direction = match decision {
            Decision::Move(direction) | Decision::Cycle(direction) => direction,
            Decision::Idle(count) => {
                stepper.set_stop_count(count);
                continue;
            }
        };
        let mut waiting = guards.running.event_waiting(self_id);
        let contact = (!guards.running.active())
            .then(|| {
                let event = map_events.events.iter().find(|event| event.id == self_id)?;
                let page =
                    crate::state::active_page(event, &switches, &variables, &party, &inventory)?;
                (page.trigger == 2 && page.layer == 1).then_some(!page.commands.is_empty())
            })
            .flatten();
        let moved = {
            let collision = MapCollision::new(
                &data,
                &map_events,
                (&switches, &variables, &party, &inventory),
                &bodies,
            );
            let mut attempt = |dir: u32, sprite: &mut EventSprite, stepper: &mut RouteStepper| {
                stepper.set_direction(sprite, dir);
                let (dx, dy) = dir_delta(dir);
                let (nx, ny) = (ex + dx, ey + dy);
                let through = stepper.through();
                let moved = collision.can_move(
                    (ex, ey),
                    (nx, ny),
                    Mover::event(sprite, through),
                    Some((px, py)),
                    false,
                );
                let front = if dir < 4 { (nx, ny) } else { (ex, ey) };
                if !moved
                    && data.normalize_tile(front.0, front.1) == (px, py)
                    && let Some(has_commands) = contact
                {
                    stepper.set_stop_count(0);
                    waiting |= has_commands;
                    if let Some(touches) = touches.as_mut() {
                        touches.0.push(self_id);
                    }
                }
                moved
            };
            attempt(direction, &mut sprite, &mut stepper)
                || (matches!(decision, Decision::Cycle(_))
                    && stepper.stop_count() >= stepper.stop_maximum() + 20
                    && attempt(reverse(direction), &mut sprite, &mut stepper))
        };
        if moved {
            let (dx, dy) = dir_delta(stepper.direction(&*sprite));
            let (nx, ny) = data.normalize_tile(ex + dx, ey + dy);
            if let Some(event) = map_events.events.iter_mut().find(|e| e.id == self_id) {
                event.x = nx as u32;
                event.y = ny as u32;
            }
            queue.set_step_secs(step_secs_for_speed(auto.speed));
            queue.enqueue_route([RouteAction::Step {
                dx,
                dy,
                face: sprite.dir,
            }]);
        } else if waiting || stepper.stop_count() >= stepper.stop_maximum() + 60 {
            stepper.set_stop_count(0);
        } else {
            stepper.restore_retry_direction(&mut *sprite, previous);
        }
        auto.set_stop_maximum(&mut stepper);
    }
}

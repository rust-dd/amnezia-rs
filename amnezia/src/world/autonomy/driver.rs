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
            || auto.move_type == 0
            || auto.move_type == 6
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
        let rand_dir = next_rand(&mut auto.rng) % 4;
        let hero_delta = data.tile_delta((ex, ey), (px, py));
        let touched = std::cell::Cell::new(false);
        let decision = {
            let collision = MapCollision::new(
                &data,
                &map_events,
                (&switches, &variables, &party, &inventory),
                &bodies,
            );
            let passable = |dir: u32| {
                let (dx, dy) = dir_delta(dir);
                let (nx, ny) = (ex + dx, ey + dy);
                let destination = data.normalize_tile(nx, ny);
                let through = stepper.through();
                if !through && !bodies.hero_through && sprite.layer == 1 && destination == (px, py)
                {
                    touched.set(true);
                }
                collision.can_move(
                    (ex, ey),
                    (nx, ny),
                    Mover::event(&sprite, through),
                    Some((px, py)),
                    false,
                )
            };
            decide(
                auto.move_type,
                stepper.direction(&*sprite),
                ex,
                ey,
                ex + hero_delta.0,
                ey + hero_delta.1,
                rand_dir,
                passable,
            )
        };

        if touched.get()
            && let Some(touches) = touches.as_mut()
        {
            touches.0.push(self_id);
        }

        match decision {
            Decision::Step(dir) => {
                let (dx, dy) = dir_delta(dir);
                let (nx, ny) = data.normalize_tile(ex + dx, ey + dy);
                if let Some(event) = map_events.events.iter_mut().find(|e| e.id == self_id) {
                    event.x = nx as u32;
                    event.y = ny as u32;
                }
                stepper.set_direction(&mut *sprite, dir);
                queue.set_step_secs(step_secs_for_speed(auto.speed));
                queue.enqueue_route([RouteAction::Step {
                    dx,
                    dy,
                    face: sprite.dir,
                }]);
            }
            Decision::Face(dir) => {
                stepper.set_direction(&mut *sprite, dir);
            }
            Decision::Idle => {}
        }
        auto.set_stop_maximum(&mut stepper);
    }
}

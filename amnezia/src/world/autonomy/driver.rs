use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn advance_event(
    In(target): In<Option<u32>>,
    time: Res<Time>,
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
            Option<&mut RouteStepper>,
        ),
        Without<Player>,
    >,
) {
    let Ok((player, hero_route)) = players.single() else {
        return;
    };
    let (px, py) = (player.tile_x, player.tile_y);
    let dt = time.delta_secs();
    let mut bodies =
        CollisionBodies::from_events(movers.iter().map(|(event, _, _, route)| (event, route)));
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
            || stepper.as_ref().is_some_and(|route| route.active())
        {
            continue;
        }
        if let Some(route) = stepper.as_mut() {
            auto.frequency = route.frequency();
            auto.speed = route.speed();
            if route.take_autonomy_reset() {
                auto.timer = stop_frames(auto.frequency) as f32 / FPS;
            }
        }
        auto.timer -= dt;
        if auto.timer > 0.0 {
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
                let through = stepper.as_ref().is_some_and(|route| route.through());
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
                stepper
                    .as_ref()
                    .map_or(sprite.dir, |route| route.direction(&*sprite)),
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
                if let Some(route) = stepper.as_mut() {
                    route.set_direction(&mut *sprite, dir);
                } else {
                    sprite.dir = dir;
                }
                queue.set_step_secs(step_secs_for_speed(auto.speed));
                queue.enqueue_route([RouteAction::Step {
                    dx,
                    dy,
                    face: sprite.dir,
                }]);
            }
            Decision::Face(dir) => {
                if let Some(route) = stepper.as_mut() {
                    route.set_direction(&mut *sprite, dir);
                } else {
                    sprite.dir = dir;
                }
            }
            Decision::Idle => {}
        }
        auto.timer = auto.next_delay();
    }
}

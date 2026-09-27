use super::*;

pub(in crate::world) fn route_event(
    In((target, forced)): In<(Option<u32>, Option<bool>)>,
    world: &mut World,
) {
    let mut ids = world
        .query::<&EventSprite>()
        .iter(world)
        .map(|event| event.id)
        .filter(|&id| target.is_none_or(|target| target == id))
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for id in ids {
        let mut turn = None;
        loop {
            turn = world
                .run_system_cached_with(part, (id, forced, turn))
                .unwrap();
            if turn.is_none() {
                break;
            }
            crate::world::update::refresh_route_switch(world);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn part(
    In((target, forced, turn)): In<(u32, Option<bool>, Option<Turn>)>,
    data: Res<MapData>,
    mut map_events: ResMut<MapEvents>,
    mut switches: ResMut<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    guards: crate::world::MoveGuards,
    mut touches: Option<ResMut<crate::world::TouchEvents>>,
    mut audio: MessageWriter<AudioRequest>,
    players: Query<(&Player, Option<&RouteStepper>), Without<EventSprite>>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut movers: Query<
        (
            &mut EventSprite,
            &mut MoveQueue,
            &mut RouteStepper,
            &mut Sprite,
        ),
        Without<Player>,
    >,
) -> Option<Turn> {
    if turn.is_none() && guards.forced_route_paused() {
        return None;
    }
    let hero = players
        .single()
        .map(|(p, _)| (p.tile_x, p.tile_y))
        .unwrap_or((-1, -1));
    let mut bodies = CollisionBodies::from_events(
        movers
            .iter()
            .map(|(event, _, route, _)| (event, Some(route))),
    );
    bodies.hero_through = players
        .single()
        .ok()
        .and_then(|(_, route)| route)
        .is_some_and(RouteStepper::through);
    bodies.include_vehicles(vehicles.as_deref(), data.map_id);
    for (mut sprite_c, mut queue, mut stepper, mut sprite) in &mut movers {
        if target != sprite_c.id
            || (turn.is_none() && forced.is_some_and(|forced| forced != stepper.forced()))
        {
            continue;
        }
        if turn.is_none() && !stepper.forced() && guards.autonomous_paused(sprite_c.id) {
            return None;
        }
        let (ex, ey) = (sprite_c.tile_x, sprite_c.tile_y);
        let self_id = sprite_c.id;
        let layer = sprite_c.layer;
        let delta = data.tile_delta((ex, ey), hero);
        let near_hero = (ex + delta.0, ey + delta.1);
        let touched = std::cell::Cell::new(false);
        let (driven, turn) = {
            let collision = MapCollision::new(
                &data,
                &map_events,
                (&switches, &variables, &party, &inventory),
                &bodies,
            );
            let can_step =
                |character: &EventSprite, dx: i32, dy: i32, jumping: bool, through: bool| {
                    let passable = collision.can_move(
                        (ex, ey),
                        (ex + dx, ey + dy),
                        Mover::event(character, through),
                        Some(hero),
                        jumping,
                    );
                    // The failure hook uses cardinal-only front coordinates, even for diagonals.
                    let front = if dx != 0 && dy != 0 {
                        (ex, ey)
                    } else {
                        (ex + dx, ey + dy)
                    };
                    if !passable
                        && !jumping
                        && layer == 1
                        && data.normalize_tile(front.0, front.1) == hero
                    {
                        touched.set(true);
                    }
                    passable
                };
            drive_part(
                &mut *sprite_c,
                &mut queue,
                &mut stepper,
                near_hero,
                can_step,
                turn,
            )
        };
        bodies.update(&sprite_c, &stepper);
        if touched.get()
            && let Some(touches) = touches.as_mut()
        {
            touches.0.push(self_id);
        }
        if let Some((dx, dy)) = driven.moved
            && let Some(event) = map_events.events.iter_mut().find(|e| e.id == self_id)
        {
            let (x, y) = data.normalize_tile(ex + dx, ey + dy);
            event.x = x.max(0) as u32;
            event.y = y.max(0) as u32;
        }
        apply_effects(driven.effects, &mut switches, &mut audio, &mut sprite);
        return turn;
    }
    None
}

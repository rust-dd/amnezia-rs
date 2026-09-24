use super::*;

#[derive(Resource, Default)]
struct InFlight(bool);

pub(super) fn register(app: &mut App) {
    app.init_resource::<InFlight>().add_systems(
        Update,
        (remember, walk::<Player>, trigger)
            .chain()
            .in_set(PlayerStep)
            .after(crate::vehicles::VehicleInput)
            .before(crate::dialogue::MessageUpdate),
    );
}

fn remember(
    scene: ScenePause,
    players: Query<&MoveQueue, With<Player>>,
    mut moving: ResMut<InFlight>,
) {
    moving.0 = !scene.paused() && players.single().is_ok_and(MoveQueue::busy);
}

#[allow(clippy::too_many_arguments)]
fn trigger(
    moving: Res<InFlight>,
    scene: ScenePause,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut running: ResMut<RunningEvent>,
    players: Query<(&Player, &MoveQueue, &RouteStepper)>,
) {
    if !moving.0 || scene.paused() || scene.riding() || running.active() {
        return;
    }
    let Ok((hero, queue, route)) = players.single() else {
        return;
    };
    if queue.busy() || route.forced() {
        return;
    }
    if let Some((id, page)) = touch_page_at(
        &events,
        &switches,
        &variables,
        &party,
        &inventory,
        hero.tile_x,
        hero.tile_y,
        false,
    ) {
        running.start(id, page.commands.clone());
    }
}

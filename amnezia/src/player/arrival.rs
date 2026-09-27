use super::*;

#[derive(Resource, Default)]
struct InFlight(bool);

pub(super) fn register(app: &mut App) {
    app.init_resource::<InFlight>()
        .init_resource::<camera::MotionScroll>();
    update::character(app, || {
        (
            remember,
            camera::prepare_scroll,
            walk::<Player>,
            camera::apply_scroll,
            trigger,
        )
            .chain()
            .in_set(PlayerStep)
            .after(crate::vehicles::VehicleInput)
            .before(crate::dialogue::MessageUpdate)
    });
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
    mut triggers: EventTriggers,
    mut running: ResMut<RunningEvent>,
    players: Query<(&Player, &MoveQueue, &RouteStepper)>,
) {
    if !moving.0 || scene.paused() || scene.airship() {
        return;
    }
    let Ok((hero, queue, route)) = players.single() else {
        return;
    };
    if queue.busy() || route.forced() {
        return;
    }
    let hero = (hero.tile_x, hero.tile_y);
    triggers.queue_at(&mut running, hero, false, &[1, 2], hero, false);
}

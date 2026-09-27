use super::*;
use crate::player::Player;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct HeroState {
    pub(crate) frame: u32,
    pub(crate) motion: MotionState,
    pub(crate) route: RouteStepper,
}

impl HeroState {
    pub(super) fn capture(hero: &Player, motion: &MoveQueue, route: &RouteStepper) -> Self {
        Self {
            frame: hero.frame,
            motion: motion.snapshot(),
            route: route.clone(),
        }
    }

    pub(crate) fn valid(&self) -> bool {
        self.frame < 4 && self.motion.valid() && self.route.valid()
    }
}

pub(crate) fn snapshot(world: &mut World) -> Option<HeroState> {
    world
        .query::<(&Player, &MoveQueue, &RouteStepper)>()
        .single(world)
        .ok()
        .map(|(hero, queue, route)| HeroState::capture(hero, queue, route))
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    state: HeroState,
}

pub(crate) fn prepare(world: &mut World, map_id: u32, state: Option<HeroState>) {
    world.remove_resource::<Pending>();
    if let Some(state) = state {
        world.insert_resource(Pending { map_id, state });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        restore
            .in_set(RestoreCharacters)
            .after(crate::teleport::MapTransfer)
            .before(crate::interpreter::InterpreterStep),
    );
}

fn restore(
    mut commands: Commands,
    mut changes: MessageReader<MapRebuilt>,
    pending: Option<Res<Pending>>,
    data: Option<Res<MapData>>,
    mut players: Query<(
        &mut Player,
        &mut MoveQueue,
        &mut RouteStepper,
        &mut Transform,
    )>,
) {
    if changes.read().count() == 0 {
        return;
    }
    let (Some(pending), Some(data)) = (pending, data) else {
        return;
    };
    if data.map_id != pending.map_id {
        return;
    }
    let Ok((mut player, mut queue, mut route, mut transform)) = players.single_mut() else {
        return;
    };
    player.frame = pending.state.frame;
    *queue = pending.state.motion.clone().into_queue();
    *route = pending.state.route.clone();
    route.restore_stop_clock(None);
    let point = queue.render_position(&*player, &data);
    transform.translation = Vec3::new(
        point.x,
        point.y + player.y_offset(),
        player.draw_z(player.tile_y),
    );
    commands.remove_resource::<Pending>();
}

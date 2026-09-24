use super::{Dialogue, InputPrompts, MessagePause};
use crate::interpreter::{InterpreterStep, RunningEvent};
use crate::player::Player;
use crate::world::{EventTriggers, MoveQueue, RouteStepper, ScenePause, dir_delta};
use bevy::prelude::*;

pub(super) fn register(app: &mut App) {
    // RPG_RT checks player actions before resuming the foreground interpreter.
    app.add_systems(
        Update,
        update
            .after(crate::menu::MenuInput)
            .after(crate::player::PlayerInput)
            .after(crate::vehicles::VehicleInput)
            .before(crate::player::PlayerStep)
            .before(InterpreterStep)
            .before(super::MessageUpdate),
    );
}

/// Facing action pages share the hero's layer and can be reached across three
/// counter tiles. Pages above or below the hero activate on the occupied tile.
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    keys: Res<ButtonInput<KeyCode>>,
    prompts: InputPrompts,
    scene: ScenePause,
    pause: MessagePause,
    mut triggers: EventTriggers,
    input: Option<Res<crate::player::InputPhase>>,
    dialogue: Res<Dialogue>,
    mut running: ResMut<RunningEvent>,
    players: Query<(&Player, Option<&MoveQueue>, Option<&RouteStepper>)>,
) {
    if !(keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter))
        || pause.paused()
        || scene.paused()
        || prompts.active()
        || dialogue.active
        || running.active()
        || input
            .as_ref()
            .map_or_else(|| running.waiting(), |input| input.blocked)
        || scene.vehicles.as_ref().is_some_and(|v| v.blocks_action())
    {
        return;
    }
    let Ok((player, queue, route)) = players.single() else {
        return;
    };
    if queue.is_some_and(MoveQueue::busy) || route.is_some_and(RouteStepper::active) {
        return;
    }
    let (x, y, _) = scene
        .vehicles
        .as_ref()
        .map_or((player.tile_x, player.tile_y, player.dir), |vehicles| {
            vehicles.hero_position((player.tile_x, player.tile_y, player.dir))
        });
    let hero = (x, y);
    let direction = scene
        .vehicles
        .as_ref()
        .and_then(|vehicles| vehicles.rider_direction())
        .unwrap_or_else(|| route.map_or(player.dir, |route| route.direction(player)));
    let (dx, dy) = dir_delta(direction);
    let (mut tx, mut ty) = triggers.data.normalize_tile(hero.0 + dx, hero.1 + dy);
    triggers.queue_at(&mut running, (tx, ty), true, &[1, 2], hero, true);
    triggers.queue_at(&mut running, hero, false, &[0], hero, true);
    for hop in 0..=3 {
        if triggers.queue_at(&mut running, (tx, ty), true, &[0], hero, true) {
            break;
        }
        if hop == 3 || !triggers.data.is_counter(tx, ty) {
            break;
        }
        (tx, ty) = triggers.data.normalize_tile(tx + dx, ty + dy);
    }
}

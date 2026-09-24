use super::{Dialogue, InputPrompts, MessagePause};
use crate::interpreter::{InterpreterStep, RunningEvent};
use crate::player::{Player, facing_tile};
use crate::world::{EventTriggers, MoveQueue, RouteStepper, ScenePause};
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
    triggers: EventTriggers,
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
    let (fx, fy) = facing_tile(player);
    let (dx, dy) = (fx - player.tile_x, fy - player.tile_y);
    let data = &triggers.data;
    let (mut tx, mut ty) = data.normalize_tile(fx, fy);
    triggers.queue_at(&mut running, (tx, ty), true, &[1, 2]);
    triggers.queue_at(&mut running, (player.tile_x, player.tile_y), false, &[0]);
    for hop in 0..=3 {
        if triggers.queue_at(&mut running, (tx, ty), true, &[0]) {
            break;
        }
        if hop == 3 || !data.is_counter(tx, ty) {
            break;
        }
        (tx, ty) = data.normalize_tile(tx + dx, ty + dy);
    }
}

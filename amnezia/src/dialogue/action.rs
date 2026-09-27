use super::{Dialogue, InputPrompts, MessagePause};
use crate::interpreter::{InterpreterStep, RunningEvent};
use crate::player::Player;
use crate::world::{EventTriggers, MoveQueue, RouteStepper, ScenePause, dir_delta};
use bevy::prelude::*;

pub(super) fn register(app: &mut App) {
    // RPG_RT checks player actions before resuming the foreground interpreter.
    crate::player::update::character(app, || {
        update
            .after(crate::menu::MenuInput)
            .after(crate::player::PlayerInput)
            .after(crate::vehicles::VehicleInput)
            .before(crate::player::PlayerStep)
            .before(InterpreterStep)
            .before(super::MessageUpdate)
    });
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
    mut players: Query<(&Player, Option<&MoveQueue>, Option<&mut RouteStepper>)>,
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
    let Ok((player, queue, mut route)) = players.single_mut() else {
        return;
    };
    if queue.is_some_and(MoveQueue::busy) || route.as_ref().is_some_and(|route| route.active()) {
        return;
    }
    let hero = (player.tile_x, player.tile_y);
    let direction = route
        .as_mut()
        .map_or(player.dir, |route| route.normalize_direction(player));
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

use super::{Dialogue, InputPrompts, MessagePause};
use crate::interpreter::{InterpreterStep, RunningEvent};
use crate::player::{Player, facing_tile};
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::world::{MapData, MapEvents, MoveQueue, RouteStepper, ScenePause};
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
    data: Res<MapData>,
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
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
    let (mut tx, mut ty) = data.normalize_tile(fx, fy);
    for hop in 0..=3 {
        for event in &map_events.events {
            if event.x as i32 != tx || event.y as i32 != ty {
                continue;
            }
            if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
                && page.trigger == 0
                && page.layer == 1
            {
                running.start(event.id, page.commands.clone());
                return;
            }
        }
        if hop == 3 || !data.is_counter(tx, ty) {
            break;
        }
        (tx, ty) = data.normalize_tile(tx + dx, ty + dy);
    }
    for event in &map_events.events {
        if event.x as i32 != player.tile_x || event.y as i32 != player.tile_y {
            continue;
        }
        if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
            && page.trigger == 0
            && page.layer != 1
        {
            running.start(event.id, page.commands.clone());
            return;
        }
    }
}

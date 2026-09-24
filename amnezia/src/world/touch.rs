use super::{EventSprite, MapEvents, MoveQueue, RouteStepper, ScenePause};
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(crate) struct TouchEvents(pub Vec<u32>);

#[cfg(test)]
pub(super) fn trigger(world: &mut World) {
    world.run_system_cached_with(trigger_event, None).unwrap();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn trigger_event(
    In(target): In<Option<u32>>,
    mut touches: ResMut<TouchEvents>,
    mut running: ResMut<RunningEvent>,
    dialogue: Res<Dialogue>,
    prompts: crate::dialogue::InputPrompts,
    scene: ScenePause,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    players: Query<(&Player, &MoveQueue, &RouteStepper)>,
    sprites: Query<(&EventSprite, &MoveQueue)>,
) {
    let attempts = std::mem::take(&mut touches.0);
    if running.active() || dialogue.active || prompts.active() || scene.paused() || scene.riding() {
        return;
    }
    let Ok((hero, queue, route)) = players.single() else {
        return;
    };
    if queue.busy() || route.active() {
        return;
    }
    for event in &events.events {
        if target.is_some_and(|id| event.id != id) {
            continue;
        }
        let Some(page) = active_page(event, &switches, &variables, &party, &inventory) else {
            continue;
        };
        if page.trigger != 2 {
            continue;
        }
        let collision = if page.layer == 1 {
            attempts.contains(&event.id)
        } else {
            event.x as i32 == hero.tile_x
                && event.y as i32 == hero.tile_y
                && sprites
                    .iter()
                    .any(|(sprite, queue)| sprite.id == event.id && !queue.busy())
        };
        if collision {
            running.start(event.id, page.commands.clone());
            break;
        }
    }
}

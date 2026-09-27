use super::{EventTriggers, RouteStepper, ScenePause};
use crate::interpreter::RunningEvent;
use crate::player::Player;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(crate) struct TouchEvents(pub Vec<u32>);

#[cfg(test)]
pub(super) fn trigger(world: &mut World) {
    world
        .run_system_cached_with(trigger_event, (None, true))
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn trigger_event(
    In((target, overlap)): In<(Option<u32>, bool)>,
    mut touches: ResMut<TouchEvents>,
    mut running: ResMut<RunningEvent>,
    scene: ScenePause,
    mut triggers: EventTriggers,
    players: Query<(&Player, &RouteStepper)>,
) {
    let attempts = std::mem::take(&mut touches.0);
    touches.0.extend(
        attempts
            .iter()
            .copied()
            .filter(|&id| target.is_some_and(|target| target != id)),
    );
    if running.active() || scene.paused() {
        return;
    }
    let Ok((hero, route)) = players.single() else {
        return;
    };
    let mut pending = Vec::new();
    for event in &triggers.events.events {
        if target.is_some_and(|id| event.id != id) {
            continue;
        }
        let Some((index, page)) = triggers.page(event) else {
            continue;
        };
        if page.trigger != 2 {
            continue;
        }
        let collision = if page.layer == 1 {
            attempts.contains(&event.id)
        } else {
            overlap
                && !route.forced()
                && event.x as i32 == hero.tile_x
                && event.y as i32 == hero.tile_y
                && triggers.stopped(event.id)
        };
        if collision {
            pending.push((event.id, index, !page.commands.is_empty()));
        }
    }
    for (id, index, has_commands) in pending {
        triggers.reset_stop_count(id);
        if has_commands {
            triggers.queue(&mut running, id, index, (hero.tile_x, hero.tile_y), false);
        }
    }
}

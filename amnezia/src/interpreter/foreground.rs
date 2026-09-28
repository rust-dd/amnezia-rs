use super::exec::Exec;
use super::parallel::{CommonEvents, common_gate_on};
use super::{Blockers, Fade, RunningEvent};
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use crate::world::{MapData, MapEvents, MapRebuilt};
use bevy::ecs::message::MessageCursor;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod queue;
pub(super) use queue::Queue;

#[derive(Resource, Default)]
pub(super) struct Inbox {
    rebuilt: MessageCursor<MapRebuilt>,
    unpause: MessageCursor<UnpauseEvent>,
}

#[derive(Message)]
pub(crate) struct UnpauseEvent(pub u32);

impl RunningEvent {
    pub(crate) fn queue_event(
        &mut self,
        map_id: u32,
        id: u32,
        page: usize,
        decision: bool,
    ) -> bool {
        self.queue.schedule(map_id, id, page, decision)
    }

    #[cfg(test)]
    pub(crate) fn queued_ids(&self) -> Vec<u32> {
        self.queue.waiting_ids()
    }

    fn unpause_event(&mut self, id: u32) {
        self.queue.unpause(id);
        if self.active() && self.frame.base_event_id() == id {
            self.queued_owner = true;
        }
    }
}

#[derive(SystemParam)]
struct Pages<'w> {
    data: Res<'w, MapData>,
    events: Res<'w, MapEvents>,
    switches: Res<'w, Switches>,
    variables: Res<'w, Variables>,
    party: Res<'w, Party>,
    inventory: Res<'w, Inventory>,
}

pub(crate) fn queue_autorun(world: &mut World, id: u32) {
    refresh(world);
    world.run_system_cached_with(schedule_autorun, id).unwrap();
}

fn schedule_autorun(In(id): In<u32>, pages: Pages, mut running: ResMut<RunningEvent>) {
    let Some(event) = pages.events.events.iter().find(|event| event.id == id) else {
        return;
    };
    let Some(index) = active_page_index(
        event,
        &pages.switches,
        &pages.variables,
        &pages.party,
        &pages.inventory,
    ) else {
        return;
    };
    let page = &event.pages[index];
    if page.trigger == 3 && !page.commands.is_empty() {
        running.queue.schedule(pages.data.map_id, id, index, false);
    }
}

pub(super) fn refresh(world: &mut World) {
    world.run_system_cached(refresh_queue).unwrap();
}

fn refresh_queue(
    pages: Pages,
    mut running: ResMut<RunningEvent>,
    mut inbox: ResMut<Inbox>,
    rebuilt: Option<Res<Messages<MapRebuilt>>>,
    unpause: Res<Messages<UnpauseEvent>>,
) {
    if rebuilt.is_some_and(|messages| inbox.rebuilt.read(&messages).count() > 0) {
        if running.restoring_queue {
            running.restoring_queue = false;
        } else {
            running.queue = default();
        }
    }
    if !running.restoring_queue {
        running.queue.refresh(
            pages.data.map_id,
            &pages.events,
            &pages.switches,
            &pages.variables,
            &pages.party,
            &pages.inventory,
        );
    }
    for event in inbox.unpause.read(&unpause) {
        running.unpause_event(event.0);
    }
}

pub(super) fn select_next(
    fade: Res<Fade>,
    blockers: Blockers,
    common: Res<CommonEvents>,
    mut running: ResMut<RunningEvent>,
    mut exec: Exec,
) -> bool {
    if exec.scene_paused(blockers.fade_busy(&fade), blockers.any()) {
        return false;
    }
    let common = common
        .0
        .iter()
        .filter(|event| {
            event.trigger == 3
                && common_gate_on(event, &exec.switches)
                && !event.commands.is_empty()
        })
        .min_by_key(|event| event.id);
    let map = running.queue.take_next().and_then(|(id, index, decision)| {
        let event = exec
            .subsystems
            .flow
            .map_events
            .as_ref()?
            .events
            .iter()
            .find(|event| event.id == id)?;
        Some((id, event.pages.get(index)?.commands.clone(), decision))
    });
    if common.is_none() && map.is_none() {
        return false;
    }
    exec.begin_map_event();
    running.queued_owner = true;
    if let Some(common) = common {
        running.frame.start(0, common.commands.clone());
    }
    if let Some((id, commands, decision)) = map {
        if running.frame.active() {
            running.frame.push_foreground(commands, id);
        } else {
            running.frame.start(id, commands);
        }
        running.frame.decision = decision;
    }
    true
}

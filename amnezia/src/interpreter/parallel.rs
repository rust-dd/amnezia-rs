//! Concurrently-running background interpreters: RM2000's parallel-process map
//! event pages (trigger 4) and parallel common events (trigger 4). Each gets its
//! own [`Frame`] in the [`ParallelPool`], stepped every frame with the same
//! per-frame command budget the foreground uses, sharing the one game state
//! through [`Exec`]. A finished background page loops from the top the next
//! frame, matching RM2000's "runs continuously" semantics.
//!
//! Scene changes pause the pool. Messages pause their owner and message-sensitive
//! commands, while other background scripts keep running.

use super::exec::{Exec, Operation, RunOutcome, run_operation};
use super::frame::Frame;
use crate::assets::{asset_root, load_ron};
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use crate::world::MapEvents;
use amnezia_data::{CommonEvent, EventCommand};
use bevy::prelude::*;

mod pages;
mod update;
pub(super) use pages::PageOwner;
pub(crate) use update::map_event;

/// The database's common events, read once at boot. Autostart (trigger 3) events
/// run foreground-style from `autorun`; parallel (trigger 4) events run in the
/// [`ParallelPool`]. This game ships a single empty stub common event, so the
/// list is effectively dormant, but the machinery drives any that exist.
#[derive(Resource, Default)]
pub struct CommonEvents(pub Vec<CommonEvent>);

impl CommonEvents {
    /// Load `common_events.ron`, or an empty set if it is absent (headless tests).
    pub(super) fn load() -> Self {
        let path = format!("{}/common_events.ron", asset_root());
        if std::path::Path::new(&path).exists() {
            CommonEvents(load_ron(&path))
        } else {
            CommonEvents::default()
        }
    }
}

/// What a pooled frame is running, and the key the reconciler matches it on. A
/// map page is keyed by `(event id, page index)` so a condition change that
/// promotes a different page tears down the old frame and starts the new one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ParallelSource {
    /// A parallel common event, by its 1-based id.
    Common(u32),
    /// A trigger-4 map event page, by event id and active page index.
    MapPage(u32, usize),
}

/// One pooled background interpreter: what it runs, the event id in scope (for
/// `this event` references; 0 for a common event), the command list it loops, and
/// its execution [`Frame`].
struct ParallelFrame {
    source: ParallelSource,
    owner: Option<PageOwner>,
    event_id: u32,
    commands: Vec<EventCommand>,
    frame: Frame,
}

/// The set of live background interpreters plus the map id they were built for, so
/// a teleport to a new map drops the old map's parallel pages before the new
/// map's are reconciled in.
#[derive(Resource, Default)]
pub struct ParallelPool {
    frames: Vec<ParallelFrame>,
    last_map: Option<u32>,
    pages: std::collections::BTreeMap<u32, pages::Selection>,
}

impl ParallelPool {
    pub(crate) fn enter_map(&mut self, map_id: Option<u32>) {
        if self.last_map == map_id {
            return;
        }
        self.frames
            .retain(|entry| !matches!(entry.source, ParallelSource::MapPage(..)));
        self.pages.clear();
        self.last_map = map_id;
    }

    pub(super) fn settle_scene(&mut self, ticket: u64, cancelled: bool) {
        for entry in &mut self.frames {
            entry.frame.settle_scene(ticket, cancelled);
        }
    }

    /// Retained background interpreters, including gated common events.
    pub fn count(&self) -> usize {
        self.frames.len()
    }
}

pub(super) fn run_parallel(world: &mut World) {
    update::run(world);
}

pub(super) fn refresh_map_pages(
    mut pool: ResMut<ParallelPool>,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
) {
    pool.refresh_pages(Some(&events), &switches, &variables, &party, &inventory);
}

fn map_source(id: u32, exec: &Exec) -> Option<ParallelSource> {
    let events = exec.subsystems.flow.map_events.as_ref()?;
    let event = events.events.iter().find(|event| event.id == id)?;
    let index = active_page_index(
        event,
        &exec.switches,
        &exec.variables,
        &exec.party,
        &exec.inventory,
    )?;
    (event.pages[index].trigger == 4 && !event.pages[index].commands.is_empty())
        .then_some(ParallelSource::MapPage(id, index))
}

pub(super) fn step_source(
    source: ParallelSource,
    operation: Operation,
    pool: &mut ParallelPool,
    common_events: &CommonEvents,
    exec: &mut Exec,
) -> RunOutcome {
    let mut entry = if let Some(index) = pool.frames.iter().position(|entry| entry.source == source)
    {
        pool.frames.remove(index)
    } else {
        if operation != Operation::Resume {
            return RunOutcome::Finished;
        }
        let Some((event_id, commands)) = fetch_commands(
            source,
            common_events,
            exec.subsystems.flow.map_events.as_deref(),
        ) else {
            return RunOutcome::Finished;
        };
        if commands.is_empty() {
            return RunOutcome::Finished;
        }
        ParallelFrame {
            source,
            owner: pool.owner(source),
            event_id,
            commands,
            frame: Frame::default(),
        }
    };
    if !entry.frame.active() {
        if operation != Operation::Resume {
            return RunOutcome::Finished;
        }
        entry.frame.start(entry.event_id, entry.commands.clone());
        entry.frame.parallel = true;
    }
    let outcome = run_operation(operation, &mut entry.frame, exec, false, entry.owner, pool);
    if entry.owner.is_none_or(|owner| owner.current(pool)) {
        pool.frames.push(entry);
    }
    outcome
}

fn discard_orphaned_results(pool: &ParallelPool, foreground: &Frame, exec: &mut Exec) {
    // Another parallel event can change a prompt owner's page before confirmation.
    if !foreground.choice_pending && !pool.frames.iter().any(|p| p.frame.choice_pending) {
        exec.choice.result = None;
    }
    if !foreground.input_pending
        && !pool.frames.iter().any(|p| p.frame.input_pending)
        && let Some(value) = exec.subsystems.input_number.result.take()
    {
        exec.variables
            .set(exec.subsystems.input_number.var_id, value as i32);
    }
}

/// Retain common interpreters across switch pauses and map interpreters while
/// no page is active. Selecting another page clears the previous interpreter.
fn reconcile(
    pool: &mut ParallelPool,
    common_events: &CommonEvents,
    map_events: Option<&MapEvents>,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
) {
    pool.refresh_pages(map_events, switches, variables, party, inventory);
    let mut desired = Vec::<ParallelSource>::new();
    for ce in &common_events.0 {
        if ce.trigger == 4 && !ce.commands.is_empty() {
            desired.push(ParallelSource::Common(ce.id));
        }
    }
    if let Some(map_events) = map_events {
        for event in &map_events.events {
            if let Some(index) = active_page_index(event, switches, variables, party, inventory)
                && event.pages[index].trigger == 4
                && !event.pages[index].commands.is_empty()
            {
                desired.push(ParallelSource::MapPage(event.id, index));
            }
        }
    }

    pool.frames.retain(|pf| {
        matches!(pf.source, ParallelSource::MapPage(..)) || desired.contains(&pf.source)
    });
    for key in &desired {
        if pool.frames.iter().any(|pf| pf.source == *key) {
            continue;
        }
        let Some((event_id, commands)) = fetch_commands(*key, common_events, map_events) else {
            continue;
        };
        pool.frames.push(ParallelFrame {
            source: *key,
            owner: pool.owner(*key),
            event_id,
            commands,
            frame: Frame::default(),
        });
    }
}

/// The `(event id, command list)` for a newly desired source. Cloned only when a
/// frame is added, so a kept frame never re-clones its page each tick.
fn fetch_commands(
    key: ParallelSource,
    common_events: &CommonEvents,
    map_events: Option<&MapEvents>,
) -> Option<(u32, Vec<EventCommand>)> {
    match key {
        // A common event has no map event id; `this event` (10005) resolves to 0.
        ParallelSource::Common(id) => common_events
            .0
            .iter()
            .find(|c| c.id == id)
            .map(|ce| (0, ce.commands.clone())),
        ParallelSource::MapPage(event_id, index) => map_events?
            .events
            .iter()
            .find(|e| e.id == event_id)
            .and_then(|e| e.pages.get(index))
            .map(|page| (event_id, page.commands.clone())),
    }
}

/// Whether a common event has no switch condition or its condition is met.
pub(super) fn common_gate_on(event: &CommonEvent, switches: &crate::state::Switches) -> bool {
    !event.switch_flag || switches.get(event.switch_id)
}

#[cfg(test)]
mod tests;

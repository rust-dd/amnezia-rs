//! Concurrently-running background interpreters: RM2000's parallel-process map
//! event pages (trigger 4) and parallel common events (trigger 2). Each gets its
//! own [`Frame`] in the [`ParallelPool`], stepped every frame with the same
//! per-frame command budget the foreground uses, sharing the one game state
//! through [`Exec`]. A finished background page loops from the top the next
//! frame, matching RM2000's "runs continuously" semantics.
//!
//! The whole pool pauses whenever a foreground scene owns the shared flow — a
//! message, choice, teleport fade, menu, shop, battle, title, or Game Over —
//! exactly as the foreground interpreter pauses, so a parallel event never runs
//! commands behind an open message box.

use super::exec::{Exec, run_frame};
use super::frame::Frame;
use super::params::Blockers;
use crate::assets::{asset_root, load_ron};
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use crate::teleport::Fade;
use crate::world::MapEvents;
use amnezia_data::{CommonEvent, EventCommand};
use bevy::prelude::*;

/// The database's common events, read once at boot. Autostart (trigger 1) events
/// run foreground-style from `autorun`; parallel (trigger 2) events run in the
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
enum ParallelSource {
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
}

impl ParallelPool {
    /// The number of background interpreters currently scheduled — one per running
    /// parallel common event and active trigger-4 map page. Stable across the
    /// per-frame loop (a single-pass page that finishes and restarts still counts),
    /// so it reads cleanly on the debug HUD.
    pub fn count(&self) -> usize {
        self.frames.len()
    }
}

/// Step every background interpreter one frame: reconcile the live set against
/// the current switches/variables (and the loaded map), then run each frame under
/// the per-frame command budget, pausing the whole pool while a foreground scene
/// owns the shared flow.
pub(super) fn run_parallel(
    time: Res<Time>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut pool: ResMut<ParallelPool>,
    common_events: Res<CommonEvents>,
    mut exec: Exec,
) {
    // Drop the previous map's parallel pages when the map changes; the reconcile
    // below rebuilds the new map's set. Detected from the loaded map id rather
    // than a MapChanged reader to keep the system-parameter count down.
    let map_id = exec.subsystems.flow.map_data.as_deref().map(|m| m.map_id);
    if pool.last_map != map_id {
        pool.frames
            .retain(|pf| !matches!(pf.source, ParallelSource::MapPage(..)));
        pool.last_map = map_id;
    }
    reconcile(
        &mut pool,
        &common_events,
        exec.subsystems.flow.map_events.as_deref(),
        &exec.switches,
        &exec.variables,
        &exec.party,
        &exec.inventory,
    );

    // Fade and the menu/shop/battle overlays can't change while the pool steps, so
    // sample them once; the message/choice/teleport/title/Game-Over conditions can
    // (a parallel event may open them), so they are re-checked each iteration.
    let static_blocked = fade.busy() || blockers.any();
    let dt = time.delta_secs();
    let mut i = 0;
    while i < pool.frames.len() {
        if static_blocked || exec.scene_owns_flow(false, false) {
            break;
        }
        // A finished (or freshly reconciled) page restarts from the top — parallel
        // events run continuously, re-executing when their list ends.
        if !pool.frames[i].frame.active() {
            let (event_id, commands) = {
                let pf = &pool.frames[i];
                (pf.event_id, pf.commands.clone())
            };
            pool.frames[i].frame.start(event_id, commands);
            pool.frames[i].frame.parallel = true;
        }
        // A page that finishes in a single pass (no wait/loop) is left inactive and
        // restarts next frame, so it drives its effect once per frame rather than
        // spinning to the step cap.
        let _ = run_frame(&mut pool.frames[i].frame, &mut exec, dt, false);
        i += 1;
    }
}

/// Bring the live frame set in line with what should be running now: parallel
/// common events whose gate switch is on, and each map event whose active page is
/// a trigger-4 parallel process. Frames whose source is no longer desired are
/// dropped; newly desired sources get a fresh (idle) frame that the stepping loop
/// starts. Matching frames keep running with their state intact.
fn reconcile(
    pool: &mut ParallelPool,
    common_events: &CommonEvents,
    map_events: Option<&MapEvents>,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
) {
    let mut desired: Vec<ParallelSource> = Vec::new();
    for ce in &common_events.0 {
        // A parallel common event runs while its switch is on, or unconditionally
        // when it names no switch (switch_id 0 = RM2000's switch_flag off).
        if ce.trigger == 2 && common_gate_on(ce, switches) && !ce.commands.is_empty() {
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

    pool.frames.retain(|pf| desired.contains(&pf.source));
    for key in &desired {
        if pool.frames.iter().any(|pf| pf.source == *key) {
            continue;
        }
        let Some((event_id, commands)) = fetch_commands(*key, common_events, map_events) else {
            continue;
        };
        pool.frames.push(ParallelFrame {
            source: *key,
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

/// Whether a common event is currently gated on: it names no switch, or its
/// switch is set. Shared with `autorun` for the autostart (trigger 1) form.
pub(super) fn common_gate_on(event: &CommonEvent, switches: &crate::state::Switches) -> bool {
    event.switch_id == 0 || switches.get(event.switch_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::{EventCondition, EventPage};

    fn cmd(code: u32) -> EventCommand {
        EventCommand {
            code,
            indent: 0,
            string: String::new(),
            params: Vec::new(),
        }
    }

    fn page(trigger: u32, condition: EventCondition) -> EventPage {
        EventPage {
            trigger,
            graphic_name: String::new(),
            graphic_index: 0,
            direction: 2,
            pattern: 1,
            move_type: 0,
            move_frequency: 3,
            move_speed: 3,
            move_route: Default::default(),
            layer: 0,
            condition,
            commands: vec![cmd(10210)],
        }
    }

    fn event(id: u32, pages: Vec<EventPage>) -> amnezia_data::Event {
        amnezia_data::Event {
            id,
            x: 0,
            y: 0,
            name: String::new(),
            pages,
        }
    }

    fn common_event(id: u32, trigger: u32, switch_id: u32) -> CommonEvent {
        CommonEvent {
            id,
            name: String::new(),
            trigger,
            switch_id,
            commands: vec![cmd(10210)],
        }
    }

    /// Run the reconciler with fresh empty state except the given switches.
    fn reconcile_with(
        pool: &mut ParallelPool,
        commons: &CommonEvents,
        map: Option<&MapEvents>,
        switches: &Switches,
    ) {
        let (variables, party, inventory) =
            (Variables::default(), Party::default(), Inventory::default());
        reconcile(pool, commons, map, switches, &variables, &party, &inventory);
    }

    #[test]
    fn parallel_common_event_appears_and_disappears_with_its_switch() {
        let commons = CommonEvents(vec![common_event(1, 2, 5)]);
        let mut pool = ParallelPool::default();
        let mut switches = Switches::default();

        reconcile_with(&mut pool, &commons, None, &switches);
        assert_eq!(
            pool.count(),
            0,
            "a parallel common event stays out while its switch is off"
        );

        switches.set(5, true);
        reconcile_with(&mut pool, &commons, None, &switches);
        assert_eq!(pool.count(), 1, "it joins the pool once its switch is on");
        assert!(matches!(pool.frames[0].source, ParallelSource::Common(1)));

        switches.set(5, false);
        reconcile_with(&mut pool, &commons, None, &switches);
        assert_eq!(
            pool.count(),
            0,
            "and leaves the pool when the switch goes off"
        );
    }

    #[test]
    fn unconditional_common_event_and_call_only_are_handled() {
        // Switch id 0 = always on; trigger 0 (call) is never a background process.
        let commons = CommonEvents(vec![common_event(1, 2, 0), common_event(2, 0, 0)]);
        let mut pool = ParallelPool::default();
        reconcile_with(&mut pool, &commons, None, &Switches::default());
        assert_eq!(
            pool.count(),
            1,
            "only the unconditional parallel event runs"
        );
        assert!(matches!(pool.frames[0].source, ParallelSource::Common(1)));
    }

    #[test]
    fn only_trigger_four_map_pages_join_the_pool() {
        let map = MapEvents {
            events: vec![
                event(1, vec![page(4, EventCondition::default())]),
                event(2, vec![page(0, EventCondition::default())]),
            ],
        };
        let mut pool = ParallelPool::default();
        reconcile_with(
            &mut pool,
            &CommonEvents::default(),
            Some(&map),
            &Switches::default(),
        );
        assert_eq!(
            pool.count(),
            1,
            "the trigger-4 page runs; the plain event does not"
        );
        assert!(matches!(
            pool.frames[0].source,
            ParallelSource::MapPage(1, 0)
        ));
    }

    #[test]
    fn a_promoted_page_resets_the_frame_to_the_new_page() {
        // Page 1 (trigger 4) activates over page 0 (trigger 4) once switch 5 is on.
        let map = MapEvents {
            events: vec![event(
                7,
                vec![
                    page(4, EventCondition::default()),
                    page(
                        4,
                        EventCondition {
                            flags: 0x01,
                            switch_a: 5,
                            ..Default::default()
                        },
                    ),
                ],
            )],
        };
        let mut pool = ParallelPool::default();
        let mut switches = Switches::default();

        reconcile_with(&mut pool, &CommonEvents::default(), Some(&map), &switches);
        assert!(matches!(
            pool.frames[0].source,
            ParallelSource::MapPage(7, 0)
        ));

        switches.set(5, true);
        reconcile_with(&mut pool, &CommonEvents::default(), Some(&map), &switches);
        assert_eq!(pool.count(), 1, "still one frame for the event");
        assert!(
            matches!(pool.frames[0].source, ParallelSource::MapPage(7, 1)),
            "the frame is keyed to the newly active page"
        );
    }

    #[test]
    fn reconcile_is_idempotent_and_preserves_running_frames() {
        let map = MapEvents {
            events: vec![event(1, vec![page(4, EventCondition::default())])],
        };
        let mut pool = ParallelPool::default();
        reconcile_with(
            &mut pool,
            &CommonEvents::default(),
            Some(&map),
            &Switches::default(),
        );
        // Mark the frame active (as the stepping loop would) and reconcile again.
        pool.frames[0].frame.start(1, vec![cmd(10210)]);
        reconcile_with(
            &mut pool,
            &CommonEvents::default(),
            Some(&map),
            &Switches::default(),
        );
        assert_eq!(
            pool.count(),
            1,
            "a still-desired frame is neither dropped nor duplicated"
        );
        assert!(
            pool.frames[0].frame.active(),
            "and its running state is preserved"
        );
    }
}

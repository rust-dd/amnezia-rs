//! The event interpreter. A foreground [`RunningEvent`] runs the active event
//! page's RM2000 command list one step at a time — driving the message box,
//! teleport fade, and the game's switches and variables — while a background
//! [`ParallelPool`] runs parallel-process map pages (trigger 4) and parallel
//! common events (trigger 2) concurrently against the same shared state. Both
//! execute the identical opcode dispatch ([`exec`]) over their own execution
//! state ([`frame::Frame`]); the only difference is scheduling. A page's commands
//! are a flat list with a per-command `indent`; conditional branches use that
//! indent to delimit their bodies.

use crate::battle::BattleActive;
use crate::dialogue::Dialogue;
use crate::gameover::GameOverActive;
use crate::menu::MenuOpen;
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::teleport::Fade;
use crate::title::TitleActive;
use crate::world::MapEvents;
use amnezia_data::EventCommand;
use bevy::prelude::*;

mod actor_query;
mod branch;
mod commands;
mod control_vars;
mod event_rng;
mod exec;
mod flow;
mod frame;
mod opcodes;
mod parallel;
mod params;
mod present;
#[cfg(test)]
mod tests;

pub(crate) use event_rng::EventRng;
use exec::{Exec, run_frame};
use frame::Frame;
use params::Blockers;

pub(crate) use commands::{actor_targets, apply_control_switches, operate_value};
pub use parallel::{CommonEvents, ParallelPool};

/// The foreground interpreter: one event page executing at a time. Wraps the
/// shared execution [`Frame`] so movement, autorun, dialogue, and the save system
/// can ask whether an event is running without seeing the interpreter internals.
#[derive(Resource, Default)]
pub struct RunningEvent {
    frame: Frame,
}

impl RunningEvent {
    /// Whether an event is currently executing. Triggers and movement pause while
    /// this holds.
    pub fn active(&self) -> bool {
        self.frame.active()
    }

    /// The running event's id for the debug HUD (`None` when idle).
    pub fn debug_id(&self) -> Option<u32> {
        self.frame.active().then_some(self.frame.event_id)
    }

    /// Begin running `commands` from the top. Ignored if a run is already live, so
    /// one event can't interrupt another mid-sequence.
    pub fn start(&mut self, event_id: u32, commands: Vec<EventCommand>) {
        self.frame.start(event_id, commands);
    }
}

pub struct InterpreterPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct InterpreterStep;

impl Plugin for InterpreterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunningEvent>()
            .init_resource::<crate::dialogue::MessageOptions>()
            .init_resource::<EventRng>()
            .init_resource::<ParallelPool>()
            .init_resource::<crate::vehicles::Vehicles>()
            .init_resource::<crate::system_bgm::SystemBgm>()
            .init_resource::<crate::panorama::Panorama>()
            .insert_resource(CommonEvents::load())
            .add_systems(
                Update,
                (autorun, run_interpreter, parallel::run_parallel)
                    .chain()
                    .in_set(InterpreterStep),
            );
    }
}

/// Execute the foreground event, one burst of commands per frame. Pauses while a
/// message box is open, a teleport fade is running, a blocking overlay is up, or a
/// `Wait` is counting down; resumes automatically once the block clears.
fn run_interpreter(
    time: Res<Time>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut running: ResMut<RunningEvent>,
    mut exec: Exec,
) {
    if !running.frame.active() {
        return;
    }
    let scene_blocked = exec.scene_owns_flow(fade.busy(), blockers.any());
    run_frame(
        &mut running.frame,
        &mut exec,
        time.delta_secs(),
        scene_blocked,
    );
}

/// Start a foreground autorun when nothing else is running: the map's autostart
/// (trigger 3) event page, or — failing that — a common event whose autostart
/// (trigger 1) switch is on. RM2000 replays an autostart page every frame its
/// condition holds; a cutscene ends by flipping a switch so a non-autorun page
/// becomes active and it stops. A common autostart likewise repeats while its
/// switch stays on.
#[allow(clippy::too_many_arguments)]
fn autorun(
    prompts: crate::dialogue::InputPrompts,
    map_events: Res<MapEvents>,
    common_events: Res<CommonEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    dialogue: Res<Dialogue>,
    fade: Res<Fade>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    battle: Res<BattleActive>,
    title: Res<TitleActive>,
    gameover: Res<GameOverActive>,
    mut running: ResMut<RunningEvent>,
) {
    if running.active()
        || prompts.active()
        || dialogue.active
        || fade.busy()
        || menu.0
        || shop.0
        || battle.0
        || title.0
        || gameover.0
    {
        return;
    }
    for event in &map_events.events {
        if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
            && page.trigger == 3
        {
            running.start(event.id, page.commands.clone());
            return;
        }
    }
    // No map autostart is waiting: run a common autostart whose switch is on. The
    // common event runs in the global scope, so its `this event` reference is 0.
    for ce in &common_events.0 {
        if ce.trigger == 1 && parallel::common_gate_on(ce, &switches) && !ce.commands.is_empty() {
            running.start(0, ce.commands.clone());
            return;
        }
    }
}

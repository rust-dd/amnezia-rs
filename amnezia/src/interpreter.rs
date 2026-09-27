//! The event interpreter. A foreground [`RunningEvent`] runs the active event
//! page's RM2000 command list one step at a time — driving the message box,
//! teleport fade, and the game's switches and variables — while a background
//! [`ParallelPool`] runs parallel-process map pages (trigger 4) and parallel
//! common events (trigger 4) concurrently against the same shared state. Both
//! execute the identical opcode dispatch ([`exec`]) over their own execution
//! state ([`frame::Frame`]); the only difference is scheduling. A page's commands
//! are a flat list with a per-command `indent`; conditional branches use that
//! indent to delimit their bodies.

use crate::teleport::Fade;
use amnezia_data::EventCommand;
use bevy::prelude::*;

mod actor_query;
mod branch;
mod commands;
pub(crate) mod continuation;
mod control_vars;
mod driver;
mod event_rng;
mod exec;
mod flow;
pub(crate) mod foreground;
mod frame;
mod map_change;
mod opcodes;
mod parallel;
mod params;
mod present;
pub(crate) mod saved;
pub(crate) mod scenes;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) use event_rng::EventRng;
use frame::Frame;
pub(crate) use map_change::on_map_change;
pub(crate) use parallel::map_event as update_map_event;
use params::Blockers;

pub(crate) use commands::{actor_targets, apply_control_switches, operate_value};
pub use parallel::{CommonEvents, ParallelPool};

/// The foreground interpreter: one event page executing at a time. Wraps the
/// shared execution [`Frame`] so movement, autorun, dialogue, and the save system
/// can ask whether an event is running without seeing the interpreter internals.
#[derive(Resource, Default)]
pub struct RunningEvent {
    frame: Frame,
    queue: foreground::Queue,
    queued_owner: bool,
    restoring_queue: bool,
    fresh: bool,
}

impl RunningEvent {
    /// Whether foreground commands or a suspended foreground wait are live.
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
        if !self.frame.active() {
            self.queued_owner = false;
            self.fresh = true;
        }
        self.frame.start(event_id, commands);
    }

    pub(crate) fn event_paused(&self, id: u32) -> bool {
        self.queue.paused(id)
            || (!self.queued_owner && self.active() && self.frame.base_event_id() == id)
    }

    pub(crate) fn waiting(&self) -> bool {
        self.queue.waiting()
    }

    pub(crate) fn event_waiting(&self, id: u32) -> bool {
        self.queue.waiting_for(id)
    }
}

pub struct InterpreterPlugin;

pub(crate) fn refresh_map_pages(world: &mut World) {
    if world.contains_resource::<ParallelPool>() {
        world
            .run_system_cached(parallel::refresh_map_pages)
            .unwrap();
        foreground::refresh(world);
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct InterpreterStep;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ParallelStep;

impl Plugin for InterpreterPlugin {
    fn build(&self, app: &mut App) {
        scenes::register(app);
        app.init_resource::<RunningEvent>()
            .init_resource::<continuation::Continuation>()
            .init_resource::<foreground::Inbox>()
            .add_message::<foreground::UnpauseEvent>()
            .init_resource::<crate::dialogue::MessageOptions>()
            .init_resource::<EventRng>()
            .init_resource::<ParallelPool>()
            .init_resource::<crate::vehicles::Vehicles>()
            .init_resource::<crate::system_bgm::SystemBgm>()
            .init_resource::<crate::panorama::Panorama>()
            .init_resource::<crate::transitions::Transition>()
            .init_resource::<crate::transitions::Settings>()
            .init_resource::<crate::transitions::Defaults>()
            .init_resource::<crate::timing::GameFrames>()
            .insert_resource(CommonEvents::load())
            .configure_sets(
                Update,
                (
                    InterpreterStep.after(crate::dialogue::MessageUpdate),
                    ParallelStep
                        .after(crate::teleport::MapTransfer)
                        .after(crate::world::saved::RestoreCharacters)
                        .after(crate::vehicles::saved::RestoreVehicles)
                        .before(crate::dialogue::MessageUpdate)
                        .before(crate::menu::MenuInput),
                ),
            )
            .add_systems(Update, run_interpreter.in_set(InterpreterStep))
            .add_systems(
                Update,
                parallel::run_parallel
                    .in_set(ParallelStep)
                    .in_set(crate::world::update::EventStep),
            );
    }
}

/// Execute the foreground event, one burst of commands per frame. Pauses while a
/// message box is open, a teleport fade is running, a blocking overlay is up, or a
/// `Wait` is counting down; resumes automatically once the block clears.
fn run_interpreter(world: &mut World) {
    driver::foreground(world);
}

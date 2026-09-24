//! Shared opcode IO, dispatch and resumable command execution.

mod actors;
mod dispatch;
mod handlers;
mod key_input;
mod message_gate;
mod messages;
mod step;
pub(super) use step::{Operation, RunOutcome, run_operation};
mod vehicles;

use super::params::SubsystemIo;
use crate::audio::AudioRequest;
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::PendingTeleport;
use crate::world::{EventSprite, MoveQueue, RouteStepper};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use dispatch::dispatch;

/// Shared opcode access, released between commands so map refreshes can update
/// the live character components before the next command reads them.
#[derive(SystemParam)]
pub(super) struct Exec<'w, 's> {
    pub(super) dialogue: ResMut<'w, Dialogue>,
    pub(super) choice: ResMut<'w, Choice>,
    pub(super) switches: ResMut<'w, Switches>,
    pub(super) variables: ResMut<'w, Variables>,
    pub(super) inventory: ResMut<'w, Inventory>,
    pub(super) party: ResMut<'w, Party>,
    pub(super) pending: ResMut<'w, PendingTeleport>,
    pub(super) hero_queue:
        Query<'w, 's, (&'static mut MoveQueue, &'static mut RouteStepper), With<Player>>,
    pub(super) event_movers: Query<
        'w,
        's,
        (
            &'static EventSprite,
            &'static mut MoveQueue,
            &'static mut RouteStepper,
        ),
        Without<Player>,
    >,
    pub(super) audio: MessageWriter<'w, AudioRequest>,
    pub(super) unpause: MessageWriter<'w, super::foreground::UnpauseEvent>,
    pub(super) subsystems: SubsystemIo<'w, 's>,
}

impl Exec<'_, '_> {
    /// Foreground execution waits on every message; parallel frames wait only
    /// for their own prompt or for commands that require a free message window.
    pub(super) fn scene_owns_flow(&self, fade_busy: bool, overlay_open: bool) -> bool {
        self.scene_paused(fade_busy, overlay_open) || self.message_pending()
    }

    pub(super) fn scene_paused(&self, fade_busy: bool, overlay_open: bool) -> bool {
        fade_busy
            || self.subsystems.event_save.0
            || self.subsystems.mapfx.transitions.state.busy()
            || overlay_open
            || self.pending.0.is_some()
            || self.subsystems.flow.title.0
            || self.subsystems.gameover.0
    }

    fn message_active(&self) -> bool {
        self.dialogue.busy() || self.choice.active() || self.subsystems.input_number.active()
    }

    fn message_pending(&self) -> bool {
        self.dialogue.active || self.choice.active() || self.subsystems.input_number.active()
    }

    fn message_reserved(&self) -> bool {
        self.message_active()
            || self.choice.result.is_some()
            || self.subsystems.input_number.result.is_some()
    }

    fn message_command_reserved(
        &self,
        foreground: bool,
        command: &amnezia_data::EventCommand,
    ) -> bool {
        let blocked = if message_gate::allows_closing_handoff(command) {
            !self.dialogue.allows_next(foreground)
                || self.choice.active()
                || self.subsystems.input_number.active()
        } else {
            self.message_active()
        };
        blocked || self.choice.result.is_some() || self.subsystems.input_number.result.is_some()
    }
}

/// What [`dispatch`] decided after handling one command.
pub(super) enum Flow {
    /// The command is done; step to the next one this frame.
    Advance,
    /// Suspend command execution until the next interpreter update.
    Yield,
    /// The run is over (Game Over / Return to Title); the caller stops the frame.
    Stop,
}

/// Whether any forced move route is still running — the hero's stepper or any
/// event's. `ProceedWithMovement` (11340) yields on this until all have drained.
fn any_route_running(
    hero_queue: &Query<(&'static mut MoveQueue, &'static mut RouteStepper), With<Player>>,
    event_movers: &Query<
        (
            &'static EventSprite,
            &'static mut MoveQueue,
            &'static mut RouteStepper,
        ),
        Without<Player>,
    >,
) -> bool {
    hero_queue.iter().any(|(_, stepper)| stepper.pending())
        || event_movers.iter().any(|(_, _, stepper)| stepper.pending())
}

/// Resolve an RM2000 character reference — 10001 the hero, 10005 this event, any
/// other positive value an event id — to its `(tile_x, tile_y, facing)`, read from
/// the live hero and event sprites. Shared by the `ControlVariables` character
/// operand and the `ConditionalBranch` orientation check; an unknown reference or
/// a missing sprite yields `None`.
pub(super) fn resolve_character(
    char_ref: i32,
    this_event: u32,
    players: &Query<&Player>,
    events: &Query<(&EventSprite, &mut MoveQueue, &mut RouteStepper), Without<Player>>,
    vehicles: &crate::vehicles::Vehicles,
) -> Option<(i32, i32, u32)> {
    if (10002..=10004).contains(&char_ref) {
        vehicles.character(char_ref)
    } else if char_ref == 10001 {
        players
            .single()
            .ok()
            .map(|p| vehicles.hero_position((p.tile_x, p.tile_y, p.dir)))
    } else {
        let id = if char_ref == 10005 {
            this_event as i32
        } else {
            char_ref
        };
        events
            .iter()
            .find(|(e, _, _)| e.id as i32 == id)
            .map(|(e, _, _)| (e.tile_x, e.tile_y, e.dir))
    }
}

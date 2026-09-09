//! The shared interpreter core: the [`Exec`] IO bundle every opcode handler
//! reads and writes, and [`run_frame`] — the per-frame resume-and-step driver
//! that both the foreground [`super::RunningEvent`] and each parallel-pool frame
//! run against. Extracting the state ([`super::frame::Frame`]) and the dispatch
//! (`dispatch`) from the driver is what lets one command list run in the
//! foreground while others run concurrently in the background, all sharing the
//! same game state through `Exec`.

mod actors;
mod dispatch;
mod handlers;
mod vehicles;

use super::commands::key_code;
use super::frame::{Frame, MAX_STEPS_PER_FRAME};
use super::params::SubsystemIo;
use crate::audio::AudioRequest;
use crate::battle::BattleOutcome;
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::PendingTeleport;
use crate::world::{EventSprite, MoveQueue, RouteStepper};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use dispatch::dispatch;

/// Every resource, writer, and query the opcode dispatch touches, bundled into
/// one `SystemParam` so both the foreground and parallel systems can pull the
/// identical shared state and hand it to [`run_frame`] as a single argument.
/// Two systems each taking `Exec` conflict on this access, so Bevy serialises
/// them — foreground and parallel never mutate the game state at the same
/// instant, only within the same frame.
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
    pub(super) subsystems: SubsystemIo<'w, 's>,
}

impl Exec<'_, '_> {
    /// Whether a foreground scene owns the shared UI/flow this frame — a message
    /// box, a choice, a teleport fade, a queued transfer, the title, or a Game
    /// Over. Both the foreground guard and the parallel pool pause on it, so a
    /// message a parallel event opens freezes everything until it is dismissed,
    /// exactly as RM2000's shared message window does.
    pub(super) fn scene_owns_flow(&self, fade_busy: bool, overlay_open: bool) -> bool {
        fade_busy
            || overlay_open
            || self.dialogue.active
            || self.choice.active()
            || self.pending.0.is_some()
            || self.subsystems.flow.title.0
            || self.subsystems.gameover.0
            || self.subsystems.input_number.active()
    }
}

/// What [`dispatch`] decided after handling one command.
pub(super) enum Flow {
    /// The command is done; step to the next one this frame.
    Advance,
    /// The command opened a blocking screen or armed a wait; yield the frame for
    /// the rest of this frame and resume next frame.
    Yield,
    /// The run is over (Game Over / Return to Title); the caller stops the frame.
    Stop,
}

/// How a [`run_frame`] call ended, so the parallel pool can loop a finished
/// background page while the foreground simply idles.
pub(super) enum RunOutcome {
    /// The frame yielded (or hit the per-frame step cap) but is still live.
    Yielded,
    /// The frame's command list ended (or a Stop opcode terminated it).
    Finished,
}

/// Run one frame's worth of an interpreter: settle any pending resume, honour the
/// pause conditions, then execute a bounded batch of commands. `scene_blocked`
/// is the caller's pause decision (see [`Exec::scene_owns_flow`]) — the shared
/// scene guard that stops foreground and background alike; `battle_pending` (a
/// per-frame flag) is checked here so only the frame awaiting a fight pauses on
/// it.
pub(super) fn run_frame(
    frame: &mut Frame,
    x: &mut Exec,
    dt: f32,
    scene_blocked: bool,
) -> RunOutcome {
    if frame.battle_pending
        && let Some(outcome) = x.subsystems.battle_result.0.take()
    {
        frame.battle_pending = false;
        if outcome == BattleOutcome::Defeat && frame.defeat_is_unhandled() {
            x.subsystems.gameover.0 = true;
            frame.stop();
            return RunOutcome::Finished;
        }
        if outcome == BattleOutcome::Escape && frame.escape_ends_event() {
            frame.stop();
            return RunOutcome::Finished;
        }
        frame.battle_outcome = Some(outcome);
    }
    // Pause while a scene owns the flow, or while this frame still awaits its
    // fight's result.
    if scene_blocked || frame.battle_pending {
        return RunOutcome::Yielded;
    }
    // Resume after the player confirmed a choice: record the pick so the
    // ShowChoice re-executes past the menu and the options self-select.
    if let Some(result) = x.choice.result.take() {
        frame.choices.insert(x.choice.indent, result);
    }
    // Resume after a merchant screen closes: record whether a trade happened and
    // step into the block so the Transaction/Stay (or NoTransaction/Cancel)
    // handler arms self-select, mirroring the battle-outcome handlers.
    if frame.shop_pending {
        frame.shop_transacted = Some(x.subsystems.merchant.outcome.transacted);
        frame.shop_pending = false;
        frame.ip += 1;
    }
    // Resume after the player entered a number: store it in the target variable,
    // then step past the InputNumber command.
    if frame.input_pending {
        if let Some(value) = x.subsystems.input_number.result.take() {
            x.variables
                .set(x.subsystems.input_number.var_id, value as i32);
        }
        frame.input_pending = false;
        frame.ip += 1;
    }
    // Resume a waiting KeyInputProc: poll the accepted keys and, once one is
    // pressed, store its RM2000 code in the target variable and step past the
    // command; otherwise keep the event paused for another frame.
    if frame.key_pending {
        let keys = &x.subsystems.flow.keys;
        let code = key_code(
            &frame.key_accept,
            keys.just_pressed(KeyCode::ArrowUp),
            keys.just_pressed(KeyCode::ArrowDown),
            keys.just_pressed(KeyCode::ArrowLeft),
            keys.just_pressed(KeyCode::ArrowRight),
            keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
            keys.just_pressed(KeyCode::Escape),
            keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight),
        );
        if code == 0 {
            return RunOutcome::Yielded;
        }
        x.variables.set(frame.key_var, code);
        frame.key_pending = false;
        frame.ip += 1;
    }
    if frame.wait > 0.0 {
        frame.wait -= dt;
        return RunOutcome::Yielded;
    }
    if frame.wait_movement {
        if any_route_running(&x.hero_queue, &x.event_movers) || x.subsystems.mapfx.vehicles.moving()
        {
            return RunOutcome::Yielded;
        }
        frame.wait_movement = false;
    }
    for _ in 0..MAX_STEPS_PER_FRAME {
        let Some(command) = frame.commands.get(frame.ip).cloned() else {
            // A callee finished: pop back to the caller frame and resume it; the
            // run ends only when there is no caller left to return to.
            if let Some(caller) = frame.call_stack.pop() {
                frame.commands = caller.commands;
                frame.ip = caller.ip;
                frame.event_id = caller.event_id;
                continue;
            }
            frame.stop();
            return RunOutcome::Finished;
        };
        match dispatch(frame, command, x) {
            Flow::Advance => {}
            Flow::Yield => return RunOutcome::Yielded,
            Flow::Stop => {
                frame.stop();
                return RunOutcome::Finished;
            }
        }
    }
    RunOutcome::Yielded
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
    hero_queue
        .iter()
        .any(|(_, stepper)| stepper.forced() && stepper.active())
        || event_movers
            .iter()
            .any(|(_, _, stepper)| stepper.forced() && stepper.active())
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

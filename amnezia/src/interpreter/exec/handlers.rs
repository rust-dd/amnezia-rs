//! The multi-line opcode handlers lifted out of [`super::dispatch`] so that file
//! stays legible: the control-variable operand resolution, the conditional-branch
//! evaluation, the character relocation, the forced move, the map battle
//! animation, and the screen/picture presentation block. Each takes the same
//! `(frame, command, exec)` the dispatch arm would and returns its [`Flow`].

use super::super::actor_query::ActorCtx;
use super::super::branch::branch_holds;
use super::super::commands::{anim_frame_count, battle_anim_wait, resolve_anim_target};
use super::super::control_vars::{apply_control_variables, resolve_operand};
use super::super::flow::skip_true_body;
use super::super::frame::{Frame, MoveWait};
use super::super::present::{Present, parse_present};
use super::{Exec, Flow, resolve_character};
use crate::animation::ShowMapAnimation;
use crate::world::{RelocateEvent, decode_route};
use amnezia_data::EventCommand;

/// `ControlVariables` (10220): resolve the operand from live state, then assign it
/// under the command's target mode and operation.
pub(super) fn control_variables(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    // Only the character operand (type 6) needs the world queries, resolved here.
    let character = if command.params.get(4).copied() == Some(6) {
        resolve_character(
            command.params.get(5).copied().unwrap_or(0),
            frame.event_id,
            &x.subsystems.flow.players,
            &x.event_movers,
        )
    } else {
        None
    };
    let actors = ActorCtx {
        data: &x.subsystems.actor_edits.game_data,
        progression: &x.subsystems.actor_edits.progression,
        vitals: &x.subsystems.vitals,
        hero_name: &x.subsystems.actor_edits.hero_name.0,
    };
    let operand = resolve_operand(
        &command.params,
        &x.variables,
        &x.inventory,
        &x.party,
        &actors,
        x.subsystems.mapfx.game_clock.seconds(),
        character,
    );
    apply_control_variables(
        &mut x.variables,
        &mut x.subsystems.event_rng,
        &command.params,
        operand,
    );
    frame.ip += 1;
    Flow::Advance
}

/// `ConditionalBranch` (12010): evaluate the predicate and either step into the
/// true body or skip past it (into an else body or the terminator).
pub(super) fn conditional_branch(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    // Timer conditional (type 2) compares the running clock; the rest are state
    // checks in `branch_holds`. The actor sub-checks (type 5) read from `ActorCtx`,
    // and the orientation check (type 6) needs the referenced character's real
    // facing, resolved here from the world.
    let kind = command.params.first().copied().unwrap_or(-1);
    let holds = if kind == 2 {
        let target = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
        let secs = x.subsystems.mapfx.game_clock.seconds();
        if command.params.get(2).copied().unwrap_or(0) == 0 {
            secs >= target
        } else {
            secs <= target
        }
    } else {
        let facing = if kind == 6 {
            resolve_character(
                command.params.get(1).copied().unwrap_or(0),
                frame.event_id,
                &x.subsystems.flow.players,
                &x.event_movers,
            )
            .map(|(_, _, dir)| dir)
        } else {
            None
        };
        let actors = ActorCtx {
            data: &x.subsystems.actor_edits.game_data,
            progression: &x.subsystems.actor_edits.progression,
            vitals: &x.subsystems.vitals,
            hero_name: &x.subsystems.actor_edits.hero_name.0,
        };
        branch_holds(
            &command.params,
            &command.string,
            &x.switches,
            &x.variables,
            &x.party,
            &x.inventory,
            &actors,
            facing,
        )
    };
    if holds {
        frame.ip += 1;
    } else {
        frame.ip = skip_true_body(&frame.commands, frame.ip, command.indent);
    }
    Flow::Advance
}

/// `ChangeEventLocation` (10860): move an event to a tile. `params = [event_ref,
/// mode, x, y]`; mode 1 reads the coords from variables. event_ref 10005 = this
/// event, N = event id (10001 = hero, not relocated here).
pub(super) fn change_event_location(
    frame: &mut Frame,
    command: &EventCommand,
    x: &mut Exec,
) -> Flow {
    let event_ref = command.params.first().copied().unwrap_or(0);
    let mode = command.params.get(1).copied().unwrap_or(0);
    let rx = command.params.get(2).copied().unwrap_or(0);
    let ry = command.params.get(3).copied().unwrap_or(0);
    let (tx, ty) = if mode == 1 {
        (x.variables.get(rx as u32), x.variables.get(ry as u32))
    } else {
        (rx, ry)
    };
    let event_id = if event_ref == 10005 {
        frame.event_id as i32
    } else {
        event_ref
    };
    if event_ref != 10001 && event_id > 0 && tx >= 0 && ty >= 0 {
        x.subsystems.relocate_writer.write(RelocateEvent {
            event_id: event_id as u32,
            x: tx as u32,
            y: ty as u32,
        });
    }
    frame.ip += 1;
    Flow::Advance
}

/// `MoveEvent` (11330): enqueue the decoded route onto its target and pause until
/// that target's queue drains. `10001` is the hero, `10005` this event, else an
/// event id.
pub(super) fn move_event(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    let target = command.params.first().copied().unwrap_or(0);
    let steps = decode_route(&command.params);
    let wait = if target == 10001 {
        if let Ok(mut queue) = x.hero_queue.single_mut() {
            queue.enqueue_route(steps);
        }
        MoveWait::Hero
    } else {
        let id = if target == 10005 {
            frame.event_id as i32
        } else {
            target
        };
        if let Some((_, mut queue)) = x.event_movers.iter_mut().find(|(e, _)| e.id as i32 == id) {
            queue.enqueue_route(steps);
        }
        MoveWait::Event(id.max(0) as u32)
    };
    frame.wait_move = Some(wait);
    frame.ip += 1;
    Flow::Yield
}

/// `ShowBattleAnimation` (11210): play an animation on a character. `params =
/// [anim_id, target_char_ref, wait, global]`. `global` tiles it 3×3, `wait` blocks
/// the event for the animation's duration (mirroring EasyRPG). A short `params` or
/// an unresolvable target no-ops and never waits.
pub(super) fn show_battle_animation(
    frame: &mut Frame,
    command: &EventCommand,
    x: &mut Exec,
) -> Flow {
    let global = command.params.get(3).copied().unwrap_or(0) > 0;
    let wait = if let [anim_id, target_ref, ..] = command.params.as_slice()
        && *anim_id >= 0
        && let Some(target) = resolve_anim_target(*target_ref, frame.event_id)
    {
        let anim_id = *anim_id as u32;
        x.subsystems.visuals.anim_writer.write(ShowMapAnimation {
            anim_id,
            target,
            global,
        });
        let frames = anim_frame_count(&x.subsystems.visuals.library, anim_id);
        battle_anim_wait(&command.params, frames)
    } else {
        None
    };
    frame.ip += 1;
    match wait {
        Some(secs) => {
            frame.wait = secs;
            Flow::Yield
        }
        None => Flow::Advance,
    }
}

/// The presentation opcodes (screen effects, pictures, Game Over): emit the
/// message the presentation plugins consume, waiting (as `Wait` does) when the
/// effect must finish before the next command; Game Over ends the run.
pub(super) fn present(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    match parse_present(command, &x.variables) {
        Some(Present::Screen(effect, wait)) => {
            x.subsystems.screen_writer.write(effect);
            frame.ip += 1;
            wait_or_advance(frame, wait)
        }
        Some(Present::Picture(picture, wait)) => {
            x.subsystems.picture_writer.write(picture);
            frame.ip += 1;
            wait_or_advance(frame, wait)
        }
        Some(Present::GameOver) => {
            x.subsystems.gameover.0 = true;
            Flow::Stop
        }
        None => {
            frame.ip += 1;
            Flow::Advance
        }
    }
}

/// Arm a `Wait` and yield when the presentation carries a duration, else advance.
fn wait_or_advance(frame: &mut Frame, wait: Option<f32>) -> Flow {
    match wait {
        Some(secs) => {
            frame.wait = secs;
            Flow::Yield
        }
        None => Flow::Advance,
    }
}

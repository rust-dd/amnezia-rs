use super::{Exec, Flow, any_route_running, dispatch, key_input, message_gate};
use crate::battle::BattleOutcome;
use crate::interpreter::frame::Frame;
use crate::interpreter::parallel::{PageOwner, ParallelPool};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::interpreter) enum Operation {
    Resume,
    Command,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::interpreter) enum RunOutcome {
    Advance,
    Yielded,
    Finished,
}

pub(in crate::interpreter) fn run_operation(
    operation: Operation,
    frame: &mut Frame,
    x: &mut Exec,
    scene_blocked: bool,
    source: Option<PageOwner>,
    pool: &mut ParallelPool,
) -> RunOutcome {
    if !refresh_parallel_pages(frame, source, pool, x) {
        return RunOutcome::Finished;
    }
    let outcome = match resume(frame, x, scene_blocked) {
        RunOutcome::Advance if operation == Operation::Command => command(frame, x),
        outcome => outcome,
    };
    if refresh_parallel_pages(frame, source, pool, x) {
        outcome
    } else {
        RunOutcome::Finished
    }
}

fn resume(frame: &mut Frame, x: &mut Exec, scene_blocked: bool) -> RunOutcome {
    if frame.scene_request.is_some() {
        return RunOutcome::Yielded;
    }
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
    if scene_blocked || frame.battle_pending {
        return RunOutcome::Yielded;
    }
    if frame.message_pending {
        if if frame.parallel {
            x.message_active()
        } else {
            x.message_pending()
        } {
            return RunOutcome::Yielded;
        }
        frame.message_pending = false;
    }
    if frame.choice_pending {
        if x.choice.active() {
            return RunOutcome::Yielded;
        }
        if let Some(result) = x.choice.result.take() {
            frame.choices.insert(x.choice.indent, result);
        }
        frame.choice_pending = false;
    }
    if frame.shop_pending {
        if x.subsystems.merchant.open.0
            || x.subsystems
                .merchant
                .scene
                .as_ref()
                .is_some_and(|scene| scene.active())
            || x.subsystems
                .merchant
                .inn
                .as_ref()
                .is_some_and(|inn| inn.active())
        {
            return RunOutcome::Yielded;
        }
        frame.shop_transacted = Some(x.subsystems.merchant.outcome.transacted);
        frame.shop_pending = false;
        frame.ip += 1;
    }
    if frame.input_pending {
        if x.subsystems.input_number.active() {
            return RunOutcome::Yielded;
        }
        if let Some(value) = x.subsystems.input_number.result.take() {
            x.variables
                .set(x.subsystems.input_number.var_id, value as i32);
        }
        frame.input_pending = false;
        frame.ip += 1;
    }
    if frame.consume_wait() {
        return RunOutcome::Yielded;
    }
    if frame.wait_movement {
        if any_route_running(&x.hero_queue, &x.event_movers)
            || x.subsystems.mapfx.vehicles.routes_pending()
        {
            return RunOutcome::Yielded;
        }
        frame.wait_movement = false;
    }
    if !key_input::resume(frame, x) {
        return RunOutcome::Yielded;
    }
    RunOutcome::Advance
}

fn command(frame: &mut Frame, x: &mut Exec) -> RunOutcome {
    let Some(command) = frame.commands.get(frame.ip).cloned() else {
        if frame.return_to_caller() {
            return RunOutcome::Advance;
        }
        frame.stop();
        return RunOutcome::Finished;
    };
    if message_gate::needs_free_message(&command, !frame.parallel)
        && x.message_command_reserved(!frame.parallel, &command)
    {
        return RunOutcome::Yielded;
    }
    match dispatch(frame, command, x) {
        Flow::Advance => RunOutcome::Advance,
        Flow::Yield => RunOutcome::Yielded,
        Flow::Stop => {
            frame.stop();
            RunOutcome::Finished
        }
    }
}

fn refresh_parallel_pages(
    frame: &mut Frame,
    source: Option<PageOwner>,
    pool: &mut ParallelPool,
    exec: &Exec,
) -> bool {
    pool.discard_changed_pages(exec);
    if source.is_some_and(|source| !source.current(pool)) {
        frame.stop();
        return false;
    }
    true
}

use super::continuation::{self, Continuation, Owner};
use super::exec::{Exec, Operation, RunOutcome, run_operation};
use super::frame::MAX_STEPS_PER_FRAME;
use super::parallel::{CommonEvents, ParallelPool, ParallelSource};
use super::{Blockers, Fade, RunningEvent};
use bevy::prelude::*;

pub(super) fn foreground(world: &mut World) {
    if world.resource::<Continuation>().waiting() {
        return;
    }
    let mut remaining = continuation::budget(world, Owner::Foreground);
    loop {
        if run(world, None, &mut remaining, true) != RunOutcome::Finished
            || remaining == 0
            || scene_limit(world, remaining)
        {
            return;
        }
        if !world
            .run_system_cached(super::foreground::select_next)
            .unwrap()
        {
            return;
        }
    }
}

pub(super) fn parallel(world: &mut World, source: ParallelSource, owns_async: bool) -> RunOutcome {
    let mut remaining = if owns_async {
        continuation::budget(world, Owner::Parallel(source))
    } else {
        MAX_STEPS_PER_FRAME
    };
    run(world, Some(source), &mut remaining, owns_async)
}

fn run(
    world: &mut World,
    source: Option<ParallelSource>,
    remaining: &mut usize,
    owns_async: bool,
) -> RunOutcome {
    flush(world);
    if *remaining == 0 {
        return RunOutcome::Yielded;
    }
    let outcome = step(world, source, Operation::Resume);
    if outcome != RunOutcome::Advance {
        return outcome;
    }
    while *remaining > 0 {
        if scene_limit(world, *remaining) {
            return RunOutcome::Yielded;
        }
        let outcome = step(world, source, Operation::Command);
        if outcome == RunOutcome::Finished {
            return outcome;
        }
        *remaining -= 1;
        if let RunOutcome::Async(op) = outcome {
            if !owns_async
                || continuation::suspend(world, Owner::interpreter(source), *remaining, op)
            {
                return RunOutcome::Suspended;
            }
            continue;
        }
        if outcome != RunOutcome::Advance {
            return outcome;
        }
    }
    RunOutcome::Yielded
}

fn scene_limit(world: &World, remaining: usize) -> bool {
    remaining < MAX_STEPS_PER_FRAME && world.resource::<super::scenes::Requests>().pending()
}

fn step(world: &mut World, source: Option<ParallelSource>, operation: Operation) -> RunOutcome {
    let outcome = match source {
        Some(source) => world
            .run_system_cached_with(step_parallel, (source, operation))
            .unwrap(),
        None => {
            let (outcome, finished) = world
                .run_system_cached_with(step_foreground, operation)
                .unwrap();
            if let Some(id) = finished {
                world
                    .run_system_cached_with(crate::world::finish_foreground, id)
                    .unwrap();
            }
            outcome
        }
    };
    flush(world);
    outcome
}

fn flush(world: &mut World) {
    super::scenes::cancel_replaced(world);
    crate::vehicles::flush(world);
    crate::teleport::flush_quick(world);
    crate::world::update::flush(world);
    super::foreground::refresh(world);
    crate::appearance::flush(world);
    crate::shop::inn::open_pending(world);
    crate::dialogue::position::latch(world);
}

fn step_foreground(
    In(operation): In<Operation>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut running: ResMut<RunningEvent>,
    mut pool: ResMut<ParallelPool>,
    mut exec: Exec,
) -> (RunOutcome, Option<u32>) {
    if !running.frame.active() {
        return (RunOutcome::Finished, None);
    }
    if std::mem::take(&mut running.fresh) && !blockers.battle_active() {
        exec.begin_map_event();
    }
    let scene_blocked = exec.scene_owns_flow(blockers.fade_busy(&fade), blockers.any());
    let base_id = running.frame.base_event_id();
    let outcome = run_operation(
        operation,
        &mut running.frame,
        &mut exec,
        scene_blocked,
        None,
        &mut pool,
    );
    if outcome == RunOutcome::Finished {
        if !blockers.battle_active() {
            exec.dialogue.face = default();
        }
        running.queue.unpause(base_id);
        running.queued_owner = false;
    }
    (
        outcome,
        (outcome == RunOutcome::Finished && base_id > 0).then_some(base_id),
    )
}

fn step_parallel(
    In((source, operation)): In<(ParallelSource, Operation)>,
    mut pool: ResMut<ParallelPool>,
    common_events: Res<CommonEvents>,
    mut exec: Exec,
) -> RunOutcome {
    super::parallel::step_source(source, operation, &mut pool, &common_events, &mut exec)
}

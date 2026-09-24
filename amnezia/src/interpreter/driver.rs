use super::exec::{Exec, Operation, RunOutcome, run_operation};
use super::frame::MAX_STEPS_PER_FRAME;
use super::parallel::{CommonEvents, ParallelPool, ParallelSource};
use super::{Blockers, Fade, RunningEvent};
use bevy::prelude::*;

pub(super) fn foreground(world: &mut World) {
    run(world, None);
}

pub(super) fn parallel(world: &mut World, source: ParallelSource) {
    run(world, Some(source));
}

fn run(world: &mut World, source: Option<ParallelSource>) {
    flush(world);
    let mut operation = Operation::Resume;
    for _ in 0..=MAX_STEPS_PER_FRAME {
        let outcome = match source {
            Some(source) => world
                .run_system_cached_with(step_parallel, (source, operation))
                .unwrap(),
            None => world
                .run_system_cached_with(step_foreground, operation)
                .unwrap(),
        };
        flush(world);
        if outcome != RunOutcome::Advance {
            return;
        }
        operation = Operation::Command;
    }
}

fn flush(world: &mut World) {
    crate::world::update::flush(world);
    crate::appearance::flush(world);
}

fn step_foreground(
    In(operation): In<Operation>,
    time: Res<Time>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut running: ResMut<RunningEvent>,
    mut pool: ResMut<ParallelPool>,
    mut exec: Exec,
) -> RunOutcome {
    if !running.frame.active() {
        return RunOutcome::Finished;
    }
    let scene_blocked = exec.scene_owns_flow(fade.busy(), blockers.any());
    run_operation(
        operation,
        &mut running.frame,
        &mut exec,
        time.delta_secs(),
        scene_blocked,
        None,
        &mut pool,
    )
}

fn step_parallel(
    In((source, operation)): In<(ParallelSource, Operation)>,
    time: Res<Time>,
    mut pool: ResMut<ParallelPool>,
    common_events: Res<CommonEvents>,
    mut exec: Exec,
) -> RunOutcome {
    super::parallel::step_source(
        source,
        operation,
        &mut pool,
        &common_events,
        &mut exec,
        time.delta_secs(),
    )
}

use super::*;
use crate::interpreter::params::Blockers;
use crate::teleport::Fade;

enum Phase {
    Begin,
    After,
    Common(u32),
    Map(u32),
}

#[derive(PartialEq)]
enum Progress {
    Continue,
    NoPage,
    Paused,
    Run(ParallelSource),
}

pub(super) fn run(world: &mut World) {
    crate::interpreter::foreground::refresh(world);
    if world.run_system_cached_with(step, Phase::Begin).unwrap() == Progress::Paused {
        return;
    }
    crate::world::update::begin(world);
    let mut ids = world
        .resource::<CommonEvents>()
        .0
        .iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for id in ids {
        if phase(world, Phase::Common(id)) == Progress::Paused {
            return;
        }
    }
    let mut ids = world
        .resource::<MapEvents>()
        .events
        .iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for id in ids {
        if !map_event(world, id) {
            return;
        }
    }
    crate::world::update::refresh(world);
}

pub(crate) fn map_event(world: &mut World, id: u32) -> bool {
    crate::world::update::refresh(world);
    match phase(world, Phase::Map(id)) {
        Progress::Paused => false,
        Progress::NoPage => true,
        Progress::Continue => {
            crate::world::update::event(world, id);
            true
        }
        Progress::Run(_) => unreachable!(),
    }
}

fn phase(world: &mut World, phase: Phase) -> Progress {
    let progress = world.run_system_cached_with(step, phase).unwrap();
    if let Progress::Run(source) = progress {
        crate::interpreter::driver::parallel(world, source);
        world.run_system_cached_with(step, Phase::After).unwrap()
    } else {
        progress
    }
}

fn step(
    In(phase): In<Phase>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut pool: ResMut<ParallelPool>,
    common_events: Res<CommonEvents>,
    foreground: Res<crate::interpreter::RunningEvent>,
    mut exec: Exec,
) -> Progress {
    if matches!(phase, Phase::Begin) {
        let map_id = exec.subsystems.flow.map_data.as_deref().map(|m| m.map_id);
        pool.enter_map(map_id);
        reconcile(
            &mut pool,
            &common_events,
            exec.subsystems.flow.map_events.as_deref(),
            &exec.switches,
            &exec.variables,
            &exec.party,
            &exec.inventory,
        );
    } else {
        pool.discard_changed_pages(&exec);
    }
    discard_orphaned_results(&pool, &foreground.frame, &mut exec);
    if exec.scene_paused(fade.busy(), blockers.any()) {
        return Progress::Paused;
    }
    let source = match phase {
        Phase::Begin | Phase::After => None,
        Phase::Common(id) => common_events
            .0
            .iter()
            .find(|event| event.id == id)
            .filter(|event| event.trigger == 4 && common_gate_on(event, &exec.switches))
            .map(|_| ParallelSource::Common(id)),
        Phase::Map(id) => {
            let Some(events) = exec.subsystems.flow.map_events.as_ref() else {
                return Progress::NoPage;
            };
            let Some(event) = events.events.iter().find(|event| event.id == id) else {
                return Progress::NoPage;
            };
            if active_page_index(
                event,
                &exec.switches,
                &exec.variables,
                &exec.party,
                &exec.inventory,
            )
            .is_none()
            {
                return Progress::NoPage;
            }
            map_source(id, &exec)
        }
    };
    if let Some(source) = source {
        return Progress::Run(source);
    }
    if exec.scene_paused(false, false) {
        Progress::Paused
    } else {
        Progress::Continue
    }
}

use super::*;
use crate::interpreter::params::Blockers;
use crate::teleport::Fade;

enum Phase {
    Begin,
    Common(u32),
    Map(u32),
}

#[derive(PartialEq)]
enum Progress {
    Continue,
    NoPage,
    Paused,
}

pub(super) fn run(world: &mut World) {
    if world.run_system_cached_with(step, Phase::Begin).unwrap() == Progress::Paused {
        return;
    }
    let mut ids = world
        .resource::<CommonEvents>()
        .0
        .iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for id in ids {
        if world
            .run_system_cached_with(step, Phase::Common(id))
            .unwrap()
            == Progress::Paused
        {
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
        crate::world::update::refresh(world);
        match world.run_system_cached_with(step, Phase::Map(id)).unwrap() {
            Progress::Paused => return,
            Progress::NoPage => {}
            Progress::Continue => crate::world::update::event(world, id),
        }
    }
    crate::world::update::refresh(world);
}

#[allow(clippy::too_many_arguments)]
fn step(
    In(phase): In<Phase>,
    time: Res<Time>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut pool: ResMut<ParallelPool>,
    common_events: Res<CommonEvents>,
    foreground: Res<crate::interpreter::RunningEvent>,
    mut exec: Exec,
) -> Progress {
    if matches!(phase, Phase::Begin) {
        let map_id = exec.subsystems.flow.map_data.as_deref().map(|m| m.map_id);
        if pool.last_map != map_id {
            pool.frames
                .retain(|entry| !matches!(entry.source, ParallelSource::MapPage(..)));
            pool.pages.clear();
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
    } else {
        pool.discard_changed_pages(&exec);
    }
    discard_orphaned_results(&pool, &foreground.frame, &mut exec);
    if exec.scene_paused(fade.busy(), blockers.any()) {
        return Progress::Paused;
    }
    let source = match phase {
        Phase::Begin => None,
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
        step_source(
            source,
            &mut pool,
            &common_events,
            &mut exec,
            time.delta_secs(),
        );
        discard_orphaned_results(&pool, &foreground.frame, &mut exec);
    }
    if exec.scene_paused(false, false) {
        Progress::Paused
    } else {
        Progress::Continue
    }
}

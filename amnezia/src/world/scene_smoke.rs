use crate::interpreter::RunningEvent;
use crate::state::Switches;
use amnezia_data::{EventCommand, EventPage, Map};
use bevy::prelude::*;
use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod pixels;
pub(crate) use pixels::snapshot;

#[derive(Clone, Copy)]
struct Case {
    map: u32,
    event: u32,
    page: usize,
    position: (i32, i32),
    switch: u32,
    mode: u32,
    phases: usize,
}

const CASES: [Case; 6] = [
    Case {
        map: 1,
        event: 3,
        page: 0,
        position: (35, 13),
        switch: 0,
        mode: 0,
        phases: 1,
    },
    Case {
        map: 2,
        event: 17,
        page: 0,
        position: (5, 6),
        switch: 0,
        mode: 1,
        phases: 4,
    },
    Case {
        map: 10,
        event: 2,
        page: 0,
        position: (12, 8),
        switch: 0,
        mode: 2,
        phases: 1,
    },
    Case {
        map: 1,
        event: 46,
        page: 0,
        position: (12, 8),
        switch: 0,
        mode: 3,
        phases: 4,
    },
    Case {
        map: 10,
        event: 17,
        page: 1,
        position: (9, 11),
        switch: 19,
        mode: 4,
        phases: 1,
    },
    Case {
        map: 2,
        event: 8,
        page: 0,
        position: (16, 8),
        switch: 0,
        mode: 5,
        phases: 4,
    },
];

#[derive(Resource, Default)]
struct Probe {
    case: usize,
    arrival: Option<u32>,
    seen: BTreeSet<(u32, u32)>,
    done: bool,
    checked: Arc<AtomicUsize>,
    updates: usize,
}

fn definition(case: Case) -> (Map, EventPage) {
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_{:04}.ron",
        crate::assets::asset_root(),
        case.map
    ));
    let page = map
        .events
        .iter()
        .find(|event| event.id == case.event)
        .unwrap()
        .pages[case.page]
        .clone();
    assert_eq!(page.animation_type, case.mode);
    (map, page)
}

fn commands(world: &mut World, case: Case) -> Vec<EventCommand> {
    world.resource_mut::<Switches>().load(if case.switch == 0 {
        vec![]
    } else {
        vec![(case.switch, true)]
    });
    world
        .resource_mut::<crate::screenfx::TintState>()
        .set_tone([100.0; 4]);
    [
        (
            10810,
            vec![case.map as i32, case.position.0, case.position.1],
        ),
        // A foreground wait isolates the original graphics from unrelated story triggers.
        (11410, vec![100_000]),
    ]
    .into_iter()
    .map(|(code, params)| EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    })
    .collect()
}

pub(crate) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.init_resource::<Probe>();
    commands(world, CASES[0])
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<String> {
    let probe = world.get_resource::<Probe>()?;
    if probe.done || world.resource::<crate::teleport::Fade>().busy() {
        return None;
    }
    let case = CASES[probe.case];
    if world.resource::<super::MapData>().map_id != case.map {
        return None;
    }
    let arrival = *world.resource_mut::<Probe>().arrival.get_or_insert(frame);
    let elapsed = frame - arrival;
    if elapsed < 30 {
        return None;
    }
    let (_, page) = definition(case);
    let actor = world
        .query::<&super::EventSprite>()
        .iter(world)
        .find(|actor| actor.id == case.event)
        .unwrap();
    assert_eq!(
        (actor.charset.as_str(), actor.index, actor.layer),
        (page.graphic_name.as_str(), page.graphic_index, page.layer)
    );
    if case.mode != 5 {
        assert_eq!(actor.dir, page.direction);
    }
    if matches!(case.mode, 0 | 2 | 4 | 5) {
        assert_eq!(actor.frame, page.pattern);
    }
    let phase = (actor.dir, actor.frame);
    world.resource_mut::<Probe>().updates += 1;
    if world.resource_mut::<Probe>().seen.insert(phase) {
        return Some(format!(
            "map-character-{}-{}-{}",
            case.mode, phase.0, phase.1
        ));
    }
    if elapsed >= 150 {
        assert_eq!(
            world.resource::<Probe>().seen.len(),
            case.phases,
            "mode {}",
            case.mode
        );
        let next = world.resource::<Probe>().case + 1;
        if next == CASES.len() {
            world.resource_mut::<Probe>().done = true;
        } else {
            let mut probe = world.resource_mut::<Probe>();
            probe.case = next;
            probe.arrival = None;
            probe.seen.clear();
            let list = commands(world, CASES[next]);
            world.insert_resource(RunningEvent::default());
            world.resource_mut::<RunningEvent>().start(0, list);
        }
    }
    None
}

pub(crate) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.done)
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(probe.done);
    assert_eq!(
        probe.checked.load(Ordering::Relaxed),
        CASES.iter().map(|case| case.phases).sum::<usize>()
    );
    assert!(probe.updates >= 6 * 120);
    info!(
        "original map characters: all six animation modes, translucent torch/crystal, {} live updates and fifteen rendered phases verified",
        probe.updates
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_cover_all_six_original_page_modes_without_replacing_their_graphics() {
        for (mode, case) in CASES.into_iter().enumerate() {
            assert_eq!(case.mode as usize, mode);
            let (_, page) = definition(case);
            assert!(!page.graphic_name.is_empty());
            assert_eq!(page.translucent, matches!(mode, 1 | 5));
        }
    }
}

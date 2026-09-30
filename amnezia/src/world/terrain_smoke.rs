use super::{Character, MapData};
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::vehicles::Vehicles;
use amnezia_data::{Chipset, EventCommand, Map};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod edges;
mod pixels;
pub(crate) use pixels::snapshot;

#[derive(Clone, Copy)]
struct Case {
    map: u32,
    terrain: u32,
    landing: bool,
    blocking_event: Option<u32>,
}

const CASES: [Case; 9] = [
    Case {
        map: 13,
        terrain: 10,
        landing: false,
        blocking_event: None,
    },
    Case {
        map: 13,
        terrain: 9,
        landing: false,
        blocking_event: None,
    },
    Case {
        map: 13,
        terrain: 1,
        landing: true,
        blocking_event: None,
    },
    Case {
        map: 13,
        terrain: 2,
        landing: false,
        blocking_event: None,
    },
    Case {
        map: 13,
        terrain: 3,
        landing: true,
        blocking_event: Some(45),
    },
    Case {
        map: 220,
        terrain: 2,
        landing: false,
        blocking_event: None,
    },
    Case {
        map: 220,
        terrain: 1,
        landing: true,
        blocking_event: None,
    },
    Case {
        map: 222,
        terrain: 1,
        landing: true,
        blocking_event: None,
    },
    Case {
        map: 222,
        terrain: 2,
        landing: false,
        blocking_event: None,
    },
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Arriving,
    Ground,
    Ascending,
    Flying,
    Descending,
    Finished,
    Done,
}

#[derive(Resource, Default)]
struct Probe {
    case: usize,
    stage: Stage,
    since: u32,
    arrival: Option<u32>,
    position: (i32, i32),
    heights: u32,
    outcomes: usize,
    checked: Arc<AtomicUsize>,
}

fn definition(case: Case) -> (Map, Chipset, (i32, i32)) {
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_{:04}.ron",
        crate::assets::asset_root(),
        case.map
    ));
    let chip = crate::assets::load_ron::<Vec<Chipset>>(&format!(
        "{}/chipsets.ron",
        crate::assets::asset_root()
    ))
    .into_iter()
    .find(|chip| chip.id == map.chipset_id)
    .unwrap();
    // Blank touch events cannot interrupt the foreground wait or obscure the ship.
    let position = (3..map.height - 3)
        .flat_map(|y| (3..map.width - 3).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let index = (y * map.width + x) as usize;
            let terrain = crate::tiles::passages_lower_index(map.lower[index])
                .and_then(|index| chip.terrain_data.get(index))
                .copied()
                .unwrap_or(1);
            u32::from(terrain) == case.terrain
                && map.events.iter().all(|event| {
                    event.x.abs_diff(x) > 1
                        || event.y.abs_diff(y) > 1
                        || event.pages.iter().all(|page| {
                            page.graphic_name.is_empty()
                                && page.graphic_index == 0
                                && page.layer == 0
                        })
                })
        })
        .unwrap_or_else(|| {
            panic!(
                "map {} terrain {} has no unobstructed interior sample",
                case.map, case.terrain
            )
        });
    (map, chip, (position.0 as i32, position.1 as i32))
}

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    }
}

fn hold(mut list: Vec<EventCommand>) -> Vec<EventCommand> {
    // Ending-map autoruns are outside this focused terrain probe.
    list.push(command(11410, vec![100_000]));
    list
}

fn enter_case(world: &mut World) -> Vec<EventCommand> {
    let case = CASES[world.resource::<Probe>().case];
    let (_, _, position) = definition(case);
    world.resource_mut::<Probe>().position = position;
    world.insert_resource(Vehicles::default());
    world.resource_mut::<crate::state::Switches>().load(vec![]);
    world
        .resource_mut::<crate::screenfx::TintState>()
        .set_tone([100.0; 4]);
    hold(vec![command(
        10810,
        vec![case.map as i32, position.0, position.1],
    )])
}

pub(crate) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.init_resource::<Probe>();
    enter_case(world)
}

fn run(world: &mut World, list: Vec<EventCommand>) {
    world.insert_resource(RunningEvent::default());
    world.resource_mut::<RunningEvent>().start(0, list);
}

fn stage(world: &mut World, next: Stage, frame: u32) {
    let mut probe = world.resource_mut::<Probe>();
    info!(
        "original terrain case {}: {:?} -> {next:?}",
        probe.case, probe.stage
    );
    probe.stage = next;
    probe.since = frame;
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<String> {
    let probe = world.get_resource::<Probe>()?;
    if probe.stage == Stage::Done || world.resource::<crate::teleport::Fade>().busy() {
        return None;
    }
    let (index, current, since, position) = (probe.case, probe.stage, probe.since, probe.position);
    let case = CASES[index];
    if world.resource::<MapData>().map_id != case.map {
        return None;
    }
    let arrival = *world.resource_mut::<Probe>().arrival.get_or_insert(frame);
    assert!(
        frame - arrival < 500,
        "terrain probe stalled in {current:?}"
    );
    let data = world.resource::<MapData>();
    assert_eq!(
        data.terrain_at(position.0, position.1).unwrap().id,
        case.terrain
    );
    assert!(data.airship_passable(position.0, position.1));
    assert_eq!(
        data.airship_landing_tile(position.0, position.1),
        case.landing
    );
    let occupants = world
        .query::<&super::EventSprite>()
        .iter(world)
        .filter(|actor| actor.tile() == position)
        .map(|actor| actor.id)
        .collect::<Vec<_>>();
    assert_eq!(
        occupants,
        case.blocking_event.into_iter().collect::<Vec<_>>()
    );
    assert_eq!(
        world.query::<&Player>().single(world).unwrap().tile(),
        position
    );
    match current {
        Stage::Arriving if frame >= arrival + 30 => {
            stage(world, Stage::Ground, frame);
            return Some(format!("terrain-{index}-ground"));
        }
        Stage::Ground if frame >= since + 20 => {
            run(
                world,
                hold(vec![
                    command(10850, vec![2, 0, case.map as i32, position.0, position.1]),
                    command(10840, vec![]),
                ]),
            );
            stage(world, Stage::Ascending, frame);
        }
        Stage::Ascending => {
            let vehicles = world.resource::<Vehicles>();
            let height = vehicles.airship_altitude() as u32;
            let cruising = vehicles.save.riding == Some(2) && !vehicles.airship_transitioning();
            world.resource_mut::<Probe>().heights |= 1 << height;
            if cruising {
                assert_eq!(height, 16);
                assert_eq!(world.resource::<Probe>().heights, (1 << 17) - 1);
                stage(world, Stage::Flying, frame);
                return Some(format!("terrain-{index}-flight"));
            }
        }
        Stage::Flying if frame >= since + 20 => {
            world.resource_mut::<Probe>().heights = 0;
            run(world, hold(vec![command(10840, vec![])]));
            stage(world, Stage::Descending, frame);
        }
        Stage::Descending => {
            let vehicles = world.resource::<Vehicles>();
            let height = vehicles.airship_altitude() as u32;
            let settled = frame >= since + 33 && !vehicles.airship_transitioning();
            let riding = vehicles.save.riding;
            world.resource_mut::<Probe>().heights |= 1 << height;
            if settled {
                let landing = case.landing && case.blocking_event.is_none();
                assert_eq!(riding, if landing { None } else { Some(2) });
                assert_eq!(height, if landing { 0 } else { 16 });
                assert_eq!(world.resource::<Probe>().heights, (1 << 17) - 1);
                world.resource_mut::<Probe>().outcomes += 1;
                stage(world, Stage::Finished, frame);
                return Some(format!("terrain-{index}-result"));
            }
        }
        Stage::Finished if frame >= since + 20 => {
            if index + 1 == CASES.len() {
                stage(world, Stage::Done, frame);
            } else {
                let mut probe = world.resource_mut::<Probe>();
                probe.case += 1;
                probe.arrival = None;
                probe.heights = 0;
                stage(world, Stage::Arriving, frame);
                let list = enter_case(world);
                run(world, list);
            }
        }
        _ => {}
    }
    None
}

pub(crate) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.stage == Stage::Done)
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.stage, Stage::Done);
    assert_eq!(probe.outcomes, CASES.len());
    assert_eq!(probe.checked.load(Ordering::Relaxed), CASES.len() * 2);
    info!(
        "original terrain: nine samples on maps 13/220/222, forest transparency, all flight heights and permitted/rejected landings verified"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_terrain_samples_keep_events_and_map_geometry_unchanged() {
        for case in CASES {
            let (map, chip, (x, y)) = definition(case);
            let index =
                crate::tiles::passages_lower_index(map.lower[(y * map.width as i32 + x) as usize])
                    .unwrap();
            assert_eq!(u32::from(chip.terrain_data[index]), case.terrain);
            assert!(x >= 3 && y >= 3);
            assert!(x < map.width as i32 - 3 && y < map.height as i32 - 3);
            let occupants = map
                .events
                .iter()
                .filter(|event| event.x == x as u32 && event.y == y as u32)
                .map(|event| event.id)
                .collect::<Vec<_>>();
            assert_eq!(
                occupants,
                case.blocking_event.into_iter().collect::<Vec<_>>()
            );
        }
    }
}

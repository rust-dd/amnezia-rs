use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Party, Switches, Variables};
use crate::world::{Character, MapData, MoveQueue};
use amnezia_data::EventCommand;
use bevy::prelude::*;

mod controls;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Depart,
    CancelOff,
    MapOn,
    CancelOn,
    MapOff,
    EnterOff,
    ReturnOff,
    MapOnAgain,
    EnterOn,
    ReturnOn,
    HideMap,
    ApproachTower,
    LandTower,
    CancelTower,
    LeaveTower,
    RevisitTower,
    LeaveAgain,
    Done,
}

#[derive(Resource)]
struct Probe {
    stage: Stage,
    generation: u64,
    pending: bool,
    choices: usize,
    departures: usize,
    interiors: usize,
    towers: usize,
    tutorial: bool,
    tower_dialogue: usize,
    last_dialogue: String,
    captures: usize,
    settled: u8,
}

pub(super) fn pirate() -> bool {
    std::env::args().any(|arg| arg == "--smoke-airship-pirate")
}

fn direction() -> u32 {
    std::env::args()
        .find_map(|arg| arg.strip_prefix("--smoke-direction=").map(str::to_owned))
        .map_or(4, |value| value.parse::<u32>().unwrap())
}

pub(super) fn entry(world: &mut World) -> Vec<EventCommand> {
    assert!((1..=4).contains(&direction()));
    world.insert_resource(Probe {
        stage: Stage::Depart,
        generation: 0,
        pending: false,
        choices: 0,
        departures: 0,
        interiors: 0,
        towers: 0,
        tutorial: false,
        tower_dialogue: 0,
        last_dialogue: String::new(),
        captures: 0,
        settled: 0,
    });
    world
        .resource_mut::<Switches>()
        .load(super::airship_history::switches(&[
            301, 327, 371, 402, 404, 405, 627,
        ]));
    world.resource_mut::<Party>().restore(if pirate() {
        vec![1, 2, 3, 8]
    } else {
        vec![1, 2, 3, 4]
    });
    for (id, value) in [(48, 13), (49, 54), (50, 36), (53, direction() as i32)] {
        world.resource_mut::<Variables>().set(id, value);
    }
    vec![
        EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![126, 9, 10],
        },
        EventCommand {
            code: 11330,
            indent: 0,
            string: String::new(),
            params: vec![10001, 8, 0, 0, 12],
        },
    ]
}

pub(super) fn input(world: &mut World, frame: u32) {
    controls::input(world, frame);
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<String> {
    let stage = world.get_resource::<Probe>()?.stage;
    if stage == Stage::Done {
        return None;
    }
    assert!(frame < 6500, "free flight stalled in {stage:?}");
    let map = world.resource::<MapData>().map_id;
    let text = world
        .resource::<Dialogue>()
        .boxes
        .iter()
        .flat_map(|page| &page.lines)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if text != world.resource::<Probe>().last_dialogue {
        let mut probe = world.resource_mut::<Probe>();
        if text.contains("A Draco irányítása") {
            assert!(!probe.tutorial, "flight tutorial repeated after re-entry");
            probe.tutorial = true;
        }
        if text.contains("Vajon miért építették") {
            probe.tower_dialogue += 1;
        }
        probe.last_dialogue = text;
    }
    let choice = world.resource::<Choice>();
    if choice.active
        && world.resource::<Dialogue>().prompt_input_ready()
        && world.resource::<Probe>().generation != choice.generation
    {
        let generation = choice.generation;
        let options = choice.options.clone();
        controls::verify_options(stage, &options);
        let mut probe = world.resource_mut::<Probe>();
        probe.generation = generation;
        probe.pending = true;
        probe.choices += 1;
        probe.captures += 1;
        return Some(format!("airship-free-{:02}-{stage:?}", probe.captures));
    }
    if !idle(world) {
        world.resource_mut::<Probe>().settled = 0;
        return None;
    }
    let mut probe = world.resource_mut::<Probe>();
    probe.settled = probe.settled.saturating_add(1);
    if probe.settled < 3 {
        return None;
    }
    match stage {
        Stage::Depart | Stage::ReturnOff | Stage::ReturnOn if map == 13 => {
            verify_flight(world, (54, 36), direction() - 1);
            assert!(world.resource::<Switches>().get(410));
            verify_map(world, stage == Stage::ReturnOn);
            let mut probe = world.resource_mut::<Probe>();
            probe.departures += 1;
            probe.stage = match stage {
                Stage::Depart => Stage::CancelOff,
                Stage::ReturnOff => Stage::MapOnAgain,
                Stage::ReturnOn => Stage::HideMap,
                _ => unreachable!(),
            };
            probe.pending = false;
        }
        Stage::EnterOff | Stage::EnterOn
            if map == 126 && !world.resource::<Switches>().get(408) =>
        {
            assert_eq!(
                world.resource::<crate::vehicles::Vehicles>().save.riding,
                None
            );
            let hero = world.query::<&Player>().single(world).unwrap();
            assert_eq!(
                (hero.tile(), hero.charset.as_str(), hero.index),
                ((9, 10), "Chara1", 0)
            );
            assert_eq!(world.resource::<Variables>().get(53), direction() as i32);
            assert_eq!(
                world.resource::<Switches>().get(611),
                stage == Stage::EnterOn
            );
            verify_map(world, false);
            let mut probe = world.resource_mut::<Probe>();
            probe.interiors += 1;
            probe.stage = if stage == Stage::EnterOff {
                Stage::ReturnOff
            } else {
                Stage::ReturnOn
            };
            probe.pending = false;
            return Some(format!("airship-free-interior-{}", probe.interiors));
        }
        Stage::CancelOff
        | Stage::MapOn
        | Stage::CancelOn
        | Stage::MapOff
        | Stage::MapOnAgain
        | Stage::HideMap
            if map == 13 && world.resource::<Probe>().pending =>
        {
            verify_map(
                world,
                matches!(stage, Stage::MapOn | Stage::CancelOn | Stage::MapOnAgain),
            );
            world.resource_mut::<Probe>().stage = match stage {
                Stage::CancelOff => Stage::MapOn,
                Stage::MapOn => Stage::CancelOn,
                Stage::CancelOn => Stage::MapOff,
                Stage::MapOff => Stage::EnterOff,
                Stage::MapOnAgain => Stage::EnterOn,
                Stage::HideMap => Stage::ApproachTower,
                _ => unreachable!(),
            };
            world.resource_mut::<Probe>().pending = false;
        }
        Stage::ApproachTower if map == 13 => {
            let (hero, queue) = world
                .query::<(&Player, &MoveQueue)>()
                .single(world)
                .unwrap();
            if hero.tile() == (54, 35) && !queue.busy() {
                world.resource_mut::<Probe>().stage = Stage::LandTower;
            }
        }
        Stage::LandTower | Stage::RevisitTower
            if map == 249 && !world.resource::<Switches>().get(408) =>
        {
            assert_eq!(
                world.resource::<crate::vehicles::Vehicles>().save.riding,
                None
            );
            assert!(world.resource::<Switches>().get(532));
            let hero = world.query::<&Player>().single(world).unwrap();
            assert_eq!(
                (hero.tile(), hero.charset.as_str(), hero.index),
                ((12, 2), "Chara1", 0)
            );
            let mut probe = world.resource_mut::<Probe>();
            probe.towers += 1;
            probe.stage = if stage == Stage::LandTower {
                Stage::CancelTower
            } else {
                Stage::LeaveAgain
            };
            probe.pending = false;
            return Some(format!("airship-free-tower-{}", probe.towers));
        }
        Stage::CancelTower if map == 249 && world.resource::<Probe>().pending => {
            world.resource_mut::<Probe>().stage = Stage::LeaveTower;
            world.resource_mut::<Probe>().pending = false;
        }
        Stage::LeaveTower | Stage::LeaveAgain if map == 13 => {
            verify_flight(world, (54, 35), crate::tiles::DIR_UP);
            assert_eq!(world.resource::<Variables>().get(53), 1);
            let mut probe = world.resource_mut::<Probe>();
            probe.stage = if stage == Stage::LeaveTower {
                Stage::RevisitTower
            } else {
                Stage::Done
            };
            probe.pending = false;
            return Some(format!("airship-free-tower-return-{}", probe.towers));
        }
        _ => {}
    }
    None
}

fn idle(world: &World) -> bool {
    !world.resource::<RunningEvent>().active()
        && !world.resource::<Choice>().active
        && world.resource::<Choice>().result.is_none()
        && !world.resource::<Dialogue>().busy()
        && !world.resource::<crate::teleport::Fade>().busy()
        && !world.resource::<crate::transitions::Transition>().busy()
        && !world
            .resource::<crate::vehicles::Vehicles>()
            .airship_transitioning()
}

fn verify_flight(world: &mut World, tile: (i32, i32), direction: u32) {
    let vehicles = world.resource::<crate::vehicles::Vehicles>();
    assert_eq!(vehicles.save.riding, Some(2));
    assert_eq!(
        (
            vehicles.save.vehicles[2].tile(),
            vehicles.save.vehicles[2].dir
        ),
        (tile, direction)
    );
    let (hero, visible) = world
        .query::<(&Player, &InheritedVisibility)>()
        .single(world)
        .unwrap();
    assert_eq!(hero.tile(), tile);
    assert!(!visible.get());
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    assert_eq!(
        world
            .query::<(&crate::vehicles::VehicleSprite, &InheritedVisibility)>()
            .iter(world)
            .filter(|(id, visible)| id.0 == 2 && visible.get())
            .count(),
        1
    );
}

fn verify_map(world: &mut World, visible: bool) {
    assert_eq!(world.resource::<Switches>().get(610), visible);
    let pictures = world
        .run_system_cached(|pictures: crate::picture::saved::Capture| pictures.snapshot())
        .unwrap();
    assert_eq!(
        pictures
            .iter()
            .any(|picture| picture.id == 1 && picture.name == "Map"),
        visible
    );
    if !visible {
        assert!(pictures.is_empty());
    }
}

pub(super) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.stage == Stage::Done)
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.stage, Stage::Done);
    assert_eq!((probe.departures, probe.interiors, probe.towers), (3, 2, 2));
    assert!(probe.tutorial);
    assert_eq!(probe.tower_dialogue, 1);
    assert_eq!(probe.choices, 14);
    info!(
        "original free flight: pirate={}, saved direction={}, all map/cancel/re-entry branches, first/repeated tower visits and return to flight verified",
        pirate(),
        direction()
    );
}

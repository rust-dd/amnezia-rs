use super::{EventSprite, MapData, MoveQueue};
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::Switches;
use amnezia_data::EventCommand;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Stage {
    #[default]
    WaterEdge,
    CheckingWater,
    StairBelow,
    Blocked,
    StairRight,
    Entering,
    Continued,
    Done,
}

#[derive(Resource, Default)]
struct Probe {
    stage: Stage,
    since: u32,
    blocked: usize,
}

fn transfer(map: i32, x: i32, y: i32) -> Vec<EventCommand> {
    vec![EventCommand {
        code: 10810,
        indent: 0,
        string: String::new(),
        params: vec![map, x, y],
    }]
}

pub(crate) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.init_resource::<Probe>();
    world.resource_mut::<Switches>().load(vec![(36, true)]);
    transfer(13, 72, 86)
}

pub(crate) fn input(world: &mut World) {
    let stage = world.get_resource::<Probe>().map(|probe| probe.stage);
    let key = match stage {
        Some(Stage::CheckingWater) => Some(KeyCode::ArrowRight),
        Some(Stage::Blocked) => Some(KeyCode::ArrowUp),
        Some(Stage::Entering | Stage::Continued) => Some(KeyCode::ArrowLeft),
        _ => None,
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
}

fn set_stage(world: &mut World, next: Stage, frame: u32) {
    let mut probe = world.resource_mut::<Probe>();
    info!("original passages: {:?} -> {next:?}", probe.stage);
    probe.stage = next;
    probe.since = frame;
}

fn tile_event(world: &mut World, id: u32, tile: u32, position: (i32, i32)) {
    let sprites = world
        .query::<&EventSprite>()
        .iter(world)
        .filter(|event| event.id == id)
        .collect::<Vec<_>>();
    assert_eq!(sprites.len(), 1);
    let event = sprites[0];
    assert_eq!((event.tile_x, event.tile_y), position);
    assert!(event.charset.is_empty());
    assert_eq!((event.index, event.layer), (tile, 0));
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let probe = world.get_resource::<Probe>()?;
    let (stage, since) = (probe.stage, probe.since);
    if frame < 200 || stage == Stage::Done {
        return None;
    }
    assert!(
        frame.saturating_sub(since.max(150)) < 900,
        "passage probe stalled in {stage:?}"
    );
    let map = world.resource::<MapData>().map_id;
    let hero = world.query::<&Player>().single(world).unwrap();
    let position = (hero.tile_x, hero.tile_y);
    let idle = !world.resource::<crate::teleport::Fade>().busy()
        && !world.resource::<crate::transitions::Transition>().busy()
        && !world.resource::<RunningEvent>().active()
        && world
            .query_filtered::<&MoveQueue, With<Player>>()
            .single(world)
            .is_ok_and(|queue| !queue.busy());
    match stage {
        Stage::WaterEdge if idle && map == 13 => {
            assert_eq!(position, (72, 86));
            assert!(!world.resource::<MapData>().passable(73, 86));
            tile_event(world, 67, 69, (73, 86));
            set_stage(world, Stage::CheckingWater, frame);
            return Some("passage-water-event");
        }
        Stage::CheckingWater => {
            assert_eq!(
                position,
                (72, 86),
                "a passable event tile cannot bypass impassable water at the origin"
            );
            world.resource_mut::<Probe>().blocked += 1;
            if frame >= since + 45 {
                set_stage(world, Stage::StairBelow, frame);
                world
                    .resource_mut::<RunningEvent>()
                    .start(0, transfer(19, 6, 8));
            }
        }
        Stage::StairBelow if idle && map == 19 => {
            assert_eq!(position, (6, 8));
            assert!(world.resource::<MapData>().passable(position.0, position.1));
            tile_event(world, 10, 138, (6, 7));
            set_stage(world, Stage::Blocked, frame);
            return Some("passage-stair-blocked");
        }
        Stage::Blocked => {
            assert_eq!(map, 19);
            assert_eq!(
                position,
                (6, 8),
                "the switched staircase must reject entry from the floor below"
            );
            world.resource_mut::<Probe>().blocked += 1;
            if frame >= since + 45 {
                set_stage(world, Stage::StairRight, frame);
                world
                    .resource_mut::<RunningEvent>()
                    .start(0, transfer(19, 7, 7));
            }
        }
        Stage::StairRight if idle && map == 19 => {
            assert_eq!(position, (7, 7));
            set_stage(world, Stage::Entering, frame);
        }
        Stage::Entering if idle && map == 20 => {
            assert_eq!(
                position,
                (12, 8),
                "original staircase must finish its post-transfer step"
            );
            set_stage(world, Stage::Continued, frame);
        }
        Stage::Continued if idle && position == (11, 8) => {
            set_stage(world, Stage::Done, frame);
            return Some("passage-continued");
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
    assert!(probe.blocked >= 90);
    info!(
        "original water boundary, tile event, blocked staircase edge, touch transfer and returned movement verified"
    );
}

use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Party, Switches, Variables};
use crate::world::{Character, MapData, MoveQueue};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Resource, Default)]
struct Probe {
    maps: Vec<u32>,
    flight: BTreeSet<(i32, i32)>,
    captures: BTreeSet<u32>,
    walking: bool,
    done: bool,
}

pub(super) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.insert_resource(Probe::default());
    world
        .resource_mut::<Switches>()
        .load(super::airship_history::switches(&[323, 327, 334, 627]));
    world.resource_mut::<Party>().restore(vec![1, 2]);
    vec![EventCommand {
        code: 10810,
        indent: 0,
        string: String::new(),
        params: vec![132, 9, 11],
    }]
}

pub(super) fn input(world: &mut World, frame: u32) {
    let mut key = None;
    if let Some(probe) = world.get_resource::<Probe>() {
        if probe.walking {
            key = Some(KeyCode::ArrowDown);
        } else if !probe.done && frame.is_multiple_of(15) {
            let dialogue = world.resource::<Dialogue>();
            if world.resource::<Choice>().active {
                if dialogue.prompt_input_ready() {
                    key = Some(KeyCode::Enter);
                }
            } else if dialogue.active {
                key = Some(KeyCode::Enter);
            } else if world.resource::<MapData>().map_id == 118 {
                let party = world.resource::<Party>();
                let cursor = world.resource::<Variables>().get(88);
                key = match cursor {
                    2 if !party.has(2) => Some(KeyCode::Enter),
                    2 => Some(KeyCode::ArrowRight),
                    3 if !party.has(4) => Some(KeyCode::Enter),
                    3 => Some(KeyCode::ArrowDown),
                    6 if !party.has(8) => Some(KeyCode::Enter),
                    _ => None,
                };
            }
        }
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if world.get_resource::<Probe>()?.done {
        return None;
    }
    let map = world.resource::<MapData>().map_id;
    if ![13, 118, 126, 132, 152, 154].contains(&map) {
        return None;
    }
    assert!(frame < 11000, "return from fortress stalled on map {map}");
    if world.resource::<Probe>().maps.last() != Some(&map) {
        world.resource_mut::<Probe>().maps.push(map);
    }
    if map == 13 {
        let vehicles = world.resource::<crate::vehicles::Vehicles>();
        if vehicles.save.riding == Some(2) && !vehicles.save.boarding {
            let tile = vehicles.save.vehicles[2].tile();
            let (hero, visible) = world
                .query::<(&Player, &InheritedVisibility)>()
                .single(world)
                .unwrap();
            assert_eq!(hero.tile(), tile);
            assert!(!visible.get());
            assert_eq!(
                world
                    .query::<(&crate::vehicles::VehicleSprite, &InheritedVisibility)>()
                    .iter(world)
                    .filter(|(id, visible)| id.0 == 2 && visible.get())
                    .count(),
                1
            );
            world.resource_mut::<Probe>().flight.insert(tile);
        }
    }
    let dialogue = world.resource::<Dialogue>();
    if dialogue.ready_to_advance() && !world.resource::<Probe>().captures.contains(&map) {
        let text = dialogue
            .boxes
            .iter()
            .flat_map(|page| &page.lines)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let label = match map {
            132 if text.contains("Húzzunk innen") => Some("airship-return-departure"),
            126 if text.contains("Akkor mehetünk Stern") => Some("airship-return-deck"),
            152 if text.contains("Köszönöm a fuvart") => Some("airship-return-tyran"),
            154 if text.contains("válságos") => Some("airship-return-hospital"),
            _ => None,
        };
        if label.is_some() {
            world.resource_mut::<Probe>().captures.insert(map);
            return label;
        }
    }
    if map == 118
        && world.resource::<Choice>().active
        && dialogue.prompt_input_ready()
        && !world.resource::<Probe>().captures.contains(&118)
    {
        assert_eq!(world.resource::<Party>().snapshot(), [1, 2, 4, 8]);
        world.resource_mut::<Probe>().captures.insert(118);
        return Some("airship-return-party");
    }
    if map == 154
        && world.resource::<Probe>().maps.contains(&118)
        && !world.resource::<Switches>().get(307)
        && !world.resource::<RunningEvent>().active()
        && !world.resource::<Dialogue>().busy()
        && !world.resource::<crate::teleport::Fade>().busy()
        && !world.resource::<crate::transitions::Transition>().busy()
        && world.resource::<crate::screenfx::TintState>().tone() == [100.0; 4]
    {
        let (hero, queue) = world
            .query::<(&Player, &MoveQueue)>()
            .single(world)
            .unwrap();
        if queue.busy() {
            return None;
        }
        if !world.resource::<Probe>().walking {
            assert_eq!(hero.tile(), (14, 12));
            world.resource_mut::<Probe>().walking = true;
        } else if hero.tile() == (14, 13) {
            let mut probe = world.resource_mut::<Probe>();
            probe.walking = false;
            probe.done = true;
            return Some("airship-return-continued");
        }
    }
    None
}

pub(super) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.done)
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(probe.done);
    super::cast_pixels::verify_finished(
        world,
        &[
            "airship-return-departure",
            "airship-return-deck",
            "airship-return-tyran",
            "airship-return-hospital",
        ],
    );
    assert_eq!(probe.maps, [132, 13, 126, 13, 152, 154, 118, 154]);
    assert_eq!(probe.captures, BTreeSet::from([118, 126, 132, 152, 154]));
    for tile in expected_flight() {
        assert!(
            probe.flight.contains(&tile),
            "missing original return-flight tile {tile:?}"
        );
    }
    let variables = world.resource::<Variables>();
    assert_eq!(
        [
            variables.get(48),
            variables.get(49),
            variables.get(50),
            variables.get(53)
        ],
        [13, 44, 99, 2]
    );
    let vehicles = world.resource::<crate::vehicles::Vehicles>();
    assert_eq!(vehicles.save.riding, None);
    assert_eq!(
        (
            vehicles.save.vehicles[2].definition.map_id,
            vehicles.save.vehicles[2].tile()
        ),
        (13, (92, 113))
    );
    for id in [371, 372, 373, 374] {
        assert!(world.resource::<Switches>().get(id), "switch {id}");
    }
    assert_eq!(world.resource::<Party>().snapshot(), [1, 2, 4, 8]);
    assert!(world.resource::<crate::menu::MenuAccess>().0);
    assert!(!world.resource::<Switches>().get(307));
    assert_eq!(
        world
            .resource::<crate::audio::CurrentBgm>()
            .track()
            .unwrap()
            .name,
        "Hospital"
    );
    assert!(!world.resource::<crate::player::HeroHidden>().0);
    info!(
        "full fortress return: both flight routes, deck departure, Tyran, hospital, original party selector and returned controls verified"
    );
}

pub(super) fn expected_flight() -> BTreeSet<(i32, i32)> {
    let mut tiles = BTreeSet::new();
    for (start, legs) in [
        (
            (28, 100),
            vec![(1, 0, 4), (0, 1, 1), (-1, 0, 8), (0, -1, 2), (1, 0, 20)],
        ),
        ((54, 99), vec![(1, 0, 30), (0, 1, 14), (1, 0, 8)]),
    ] {
        let (mut x, mut y) = start;
        tiles.insert((x, y));
        for (dx, dy, count) in legs {
            for _ in 0..count {
                x += dx;
                y += dy;
                tiles.insert((x, y));
            }
        }
    }
    tiles
}

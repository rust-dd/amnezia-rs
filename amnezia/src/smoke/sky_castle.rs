use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Party, Switches};
use crate::world::{Character, EventSprite, MapData, MoveQueue};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Resource, Default)]
struct Probe {
    maps: Vec<u32>,
    generation: u64,
    choices: usize,
    cancellations: BTreeSet<usize>,
    flight: BTreeSet<(i32, i32)>,
    jumped: BTreeSet<u32>,
    landed: BTreeSet<u32>,
    briefing: bool,
    murder_mentioned: bool,
    staged: bool,
    walking: bool,
    done: bool,
}

fn pirate() -> bool {
    std::env::args().any(|arg| arg == "--smoke-airship-pirate")
}

pub(super) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.insert_resource(Probe::default());
    world
        .resource_mut::<Switches>()
        .load(super::airship_history::switches(&[327, 371, 402, 627]));
    world.resource_mut::<Switches>().set(323, pirate());
    world.resource_mut::<Switches>().set(301, !pirate());
    world.resource_mut::<Party>().restore(if pirate() {
        vec![1, 2, 3, 8]
    } else {
        vec![1, 2, 3, 4]
    });
    for id in 1..=9 {
        world.resource_mut::<crate::vitals::Vitals>().set(id, 1, 0);
    }
    vec![
        EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![126, 9, 11],
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
    let mut key = None;
    if let Some(probe) = world.get_resource::<Probe>() {
        if probe.walking {
            key = Some(KeyCode::ArrowDown);
        } else if !probe.done && frame.is_multiple_of(15) {
            let choice = world.resource::<Choice>();
            if choice.active {
                if world.resource::<Dialogue>().prompt_input_ready()
                    && probe.generation == choice.generation
                {
                    key = Some(if matches!(probe.choices, 1 | 3) {
                        KeyCode::Escape
                    } else {
                        KeyCode::Enter
                    });
                }
            } else if world.resource::<Dialogue>().active {
                key = Some(KeyCode::Enter);
            } else if world.resource::<MapData>().map_id == 126
                && idle(world)
                && probe.choices < 5
                && (!matches!(probe.choices, 1 | 3) || probe.cancellations.contains(&probe.choices))
            {
                let hero = world.query::<&Player>().single(world).unwrap();
                key = Some(if hero.tile() == (9, 11) {
                    KeyCode::ArrowUp
                } else {
                    KeyCode::Enter
                });
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
    if ![126, 170, 171, 172].contains(&map) {
        return None;
    }
    assert!(frame < 11000, "sky castle route stalled on map {map}");
    if world.resource::<Probe>().maps.last() != Some(&map) {
        world.resource_mut::<Probe>().maps.push(map);
    }
    if map == 126 && world.resource::<Dialogue>().ready_to_advance() {
        let text = world
            .resource::<Dialogue>()
            .boxes
            .iter()
            .flat_map(|page| &page.lines)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if text.contains("Stan-t is megölte") {
            world.resource_mut::<Probe>().murder_mentioned = true;
        }
        if text.contains("Szóval erre használható") && !world.resource::<Probe>().briefing {
            world.resource_mut::<Probe>().briefing = true;
            return Some("airship-sky-briefing");
        }
    }
    let choice = world.resource::<Choice>();
    if choice.active
        && world.resource::<Dialogue>().prompt_input_ready()
        && choice.generation != world.resource::<Probe>().generation
    {
        let generation = choice.generation;
        let options = choice.options.clone();
        assert!(world.resource::<Probe>().briefing);
        assert!(world.resource::<Switches>().get(405));
        let variables = world.resource::<crate::state::Variables>();
        assert_eq!(
            [
                variables.get(48),
                variables.get(49),
                variables.get(50),
                variables.get(53)
            ],
            [13, 93, 60, 4]
        );
        let mut probe = world.resource_mut::<Probe>();
        probe.generation = generation;
        probe.choices += 1;
        let count = probe.choices;
        assert!(count <= 5);
        assert_eq!(options.len(), if matches!(count, 3 | 5) { 2 } else { 3 });
        return Some(match count {
            1 => "airship-sky-cancel",
            2 => "airship-sky-retry",
            3 => "airship-sky-decline",
            4 => "airship-sky-ready",
            5 => "airship-sky-confirm",
            _ => unreachable!(),
        });
    }
    let count = world.resource::<Probe>().choices;
    if map == 126 && matches!(count, 1 | 3) && idle(world) {
        assert!(!world.resource::<Switches>().get(408));
        assert!(!world.resource::<Switches>().get(411));
        assert_eq!(
            world.resource::<crate::vehicles::Vehicles>().save.riding,
            None
        );
        let (pilot, route) = world
            .query::<(&EventSprite, &crate::world::RouteStepper)>()
            .iter(world)
            .find(|(actor, _)| actor.id == 27)
            .unwrap();
        if route.pending() {
            return None;
        }
        assert_eq!(
            (pilot.charset.as_str(), pilot.index, pilot.tile(), pilot.dir),
            (
                "Chara2",
                if pirate() { 4 } else { 2 },
                (9, 9),
                crate::tiles::DIR_UP
            )
        );
        world.resource_mut::<Probe>().cancellations.insert(count);
    }
    if map == 171 && world.resource::<crate::vehicles::Vehicles>().save.riding == Some(2) {
        let (hero, visible) = world
            .query::<(&Player, &InheritedVisibility)>()
            .single(world)
            .unwrap();
        let tile = hero.tile();
        assert!(!visible.get());
        assert_eq!(
            tile,
            world.resource::<crate::vehicles::Vehicles>().save.vehicles[2].tile()
        );
        world.resource_mut::<Probe>().flight.insert(tile);
    }
    if map == 170 {
        let states = world
            .query::<(&EventSprite, &MoveQueue)>()
            .iter(world)
            .filter(|(actor, _)| (1..=8).contains(&actor.id))
            .map(|(actor, queue)| (actor.id, actor.tile(), queue.jumping()))
            .collect::<Vec<_>>();
        for (id, tile, jumping) in states {
            let mut probe = world.resource_mut::<Probe>();
            if jumping {
                probe.jumped.insert(id);
            }
            if probe.jumped.contains(&id) && !jumping && tile.1 < 24 {
                probe.landed.insert(id);
            }
        }
        let dialogue = world.resource::<Dialogue>();
        let text = dialogue
            .boxes
            .iter()
            .flat_map(|page| &page.lines)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if dialogue.ready_to_advance()
            && text.contains("Sikerült...")
            && !world.resource::<Probe>().staged
        {
            assert!(world.resource::<crate::player::HeroHidden>().0);
            assert_eq!(world.resource::<Probe>().landed, cast_ids());
            if pirate() {
                let (actor, visible) = world
                    .query::<(&EventSprite, &InheritedVisibility)>()
                    .iter(world)
                    .find(|(actor, _)| actor.id == 7)
                    .unwrap();
                assert!(actor.charset.is_empty() && !visible.get());
                assert_eq!(actor.tile(), (21, 24));
            }
            world.resource_mut::<Probe>().staged = true;
            return Some("airship-sky-island");
        }
    }
    if map == 172
        && world.resource::<Switches>().get(428)
        && world.resource::<Switches>().get(430)
        && idle(world)
    {
        let (hero, queue) = world
            .query::<(&Player, &MoveQueue)>()
            .single(world)
            .unwrap();
        if queue.busy() {
            return None;
        }
        if !world.resource::<Probe>().walking {
            assert_eq!(hero.tile(), (12, 15));
            world.resource_mut::<Probe>().walking = true;
        } else if hero.tile() == (12, 16) {
            let mut probe = world.resource_mut::<Probe>();
            probe.walking = false;
            probe.done = true;
            return Some("airship-sky-continued");
        }
    }
    None
}

fn idle(world: &World) -> bool {
    !world.resource::<RunningEvent>().active()
        && !world.resource::<Dialogue>().busy()
        && !world.resource::<crate::teleport::Fade>().busy()
        && !world.resource::<crate::transitions::Transition>().busy()
}

pub(super) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.done)
}

fn cast_ids() -> BTreeSet<u32> {
    (1..=8).filter(|id| *id != 7 || !pirate()).collect()
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(probe.done && probe.staged && probe.briefing);
    assert_eq!(probe.murder_mentioned, pirate());
    assert_eq!(world.resource::<Switches>().get(301), !pirate());
    super::cast_pixels::verify_finished(world, &["airship-sky-briefing", "airship-sky-island"]);
    assert_eq!(probe.maps, [126, 171, 126, 170, 172]);
    assert_eq!(probe.choices, 5);
    assert_eq!(probe.cancellations, BTreeSet::from([1, 3]));
    assert_eq!(probe.jumped, cast_ids());
    assert_eq!(probe.landed, probe.jumped);
    for y in 7..=54 {
        assert!(
            probe.flight.contains(&(10, y)),
            "missing sky flight tile 10,{y}"
        );
    }
    for id in [404, 411, 412, 428, 430, 583, 584] {
        assert!(world.resource::<Switches>().get(id), "switch {id}");
    }
    for id in 1..=9 {
        assert_eq!(
            world.resource::<crate::vitals::Vitals>().get_stored(id),
            None,
            "actor {id} not healed"
        );
    }
    assert_eq!(world.resource::<Party>().snapshot(), [1]);
    assert_eq!(
        world.resource::<crate::vehicles::Vehicles>().save.riding,
        None
    );
    assert!(!world.resource::<crate::player::HeroHidden>().0);
    info!(
        "full sky castle route: pilot pirate={}, briefing, both cancellations, confirmation, flight, all {} jumps, healing, castle entrance and returned controls verified",
        pirate(),
        probe.jumped.len()
    );
}

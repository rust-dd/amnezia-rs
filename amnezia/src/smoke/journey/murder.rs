use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::state::{Party, Switches, Variables};
use crate::world::{Character, EventSprite, MapData};
use amnezia_data::EventCommand;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Probe {
    choice: bool,
    witnesses: bool,
    branch: bool,
}

pub(super) fn enabled() -> bool {
    std::env::args().any(|arg| arg == "--smoke-airship-murder")
}

fn comfort() -> bool {
    std::env::args().any(|arg| arg == "--smoke-airship-affinity")
}

pub(super) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.insert_resource(Probe::default());
    world
        .resource_mut::<Switches>()
        .load(vec![(323, true), (324, true)]);
    world.resource_mut::<Variables>().set(1, 9);
    world.resource_mut::<Party>().restore(vec![1, 2]);
    let mut timer = world.resource_mut::<crate::timer::GameClock>();
    timer.set_secs(120);
    timer.start();
    vec![
        EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![129, 15, 3],
        },
        EventCommand {
            code: 11330,
            indent: 0,
            string: String::new(),
            params: vec![10001, 8, 0, 0, 13],
        },
    ]
}

pub(super) fn input(world: &mut World) -> bool {
    if !world.contains_resource::<Probe>() || world.resource::<MapData>().map_id != 129 {
        return false;
    }
    let frame = world.resource::<super::super::SmokeRun>().frame;
    let choice = world.resource::<Choice>();
    let key = if choice.active {
        if !world.resource::<Dialogue>().prompt_input_ready() {
            None
        } else if choice.cursor != usize::from(comfort()) {
            Some(KeyCode::ArrowDown)
        } else {
            Some(KeyCode::Enter)
        }
    } else {
        Some(KeyCode::Enter)
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if frame.is_multiple_of(12)
        && let Some(key) = key
    {
        keys.press(key);
    }
    true
}

pub(super) fn drive(world: &mut World) -> Option<&'static str> {
    world.get_resource::<Probe>()?;
    if world.resource::<MapData>().map_id != 129 {
        return None;
    }
    if world.resource::<Choice>().active
        && world.resource::<Dialogue>().prompt_input_ready()
        && !world.resource::<Probe>().choice
    {
        assert_eq!(
            world.resource::<Choice>().options,
            ["Szedd össze magad!", "Semmi baj... Nyugodj meg..."]
        );
        world.resource_mut::<Probe>().choice = true;
        return Some("airship-murder-choice");
    }
    let dialogue = world.resource::<Dialogue>();
    if !dialogue.ready_to_advance() {
        return None;
    }
    let text = dialogue
        .boxes
        .iter()
        .flat_map(|page| &page.lines)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if text.contains(if comfort() {
        "De... Az ott..."
    } else {
        "Igazad van... Erősnek kell"
    }) {
        world.resource_mut::<Probe>().branch = true;
    }
    if text.contains("Ki ez a") && !world.resource::<Probe>().witnesses {
        let actors = world
            .query::<&EventSprite>()
            .iter(world)
            .filter(|actor| (6..=9).contains(&actor.id))
            .map(|actor| (actor.id, actor.charset.clone(), actor.index, actor.tile()))
            .collect::<Vec<_>>();
        assert_eq!(actors.len(), 4);
        for (id, charset, index, tile) in actors {
            if id == 7 {
                assert_eq!((charset.as_str(), index), ("Torch", 1));
                continue;
            }
            let expected = match id {
                6 => ("Chara1", 1, (15, 3)),
                8 => ("Chara2", 4, (13, 5)),
                9 => ("Chara4", 0, (15, 5)),
                _ => unreachable!(),
            };
            assert_eq!(
                (charset.as_str(), index, tile),
                expected,
                "murder witness {id}"
            );
        }
        world.resource_mut::<Probe>().witnesses = true;
        return Some("airship-murder-witnesses");
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    let Some(probe) = world.get_resource::<Probe>() else {
        return;
    };
    assert!(probe.choice && probe.witnesses && probe.branch);
    super::super::cast_pixels::verify_finished(world, &["airship-murder-witnesses"]);
    assert_eq!(
        world.resource::<Variables>().get(1),
        if comfort() { 10 } else { 8 }
    );
    assert!(world.resource::<Switches>().get(326));
    assert_eq!(world.resource::<Variables>().get(92), 1);
    info!(
        "original murder discovery, witnesses, choice branch and affinity delta verified; comfort={}",
        comfort()
    );
}

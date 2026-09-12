use crate::dialogue::{Dialogue, view};
use crate::interpreter::RunningEvent;
use crate::state::Switches;
use crate::timing::GameFrames;
use amnezia_data::EventCommand;
use bevy::prelude::*;

mod prompts;

#[derive(Resource, Default)]
struct Fixture {
    case: u32,
    started: Option<u32>,
    checks: u32,
    completed: u8,
    captures: u8,
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        262 | 355 | 530 | 960 => Some(KeyCode::Enter),
        280 => Some(KeyCode::Space),
        320 | 450 | 820 => Some(KeyCode::Escape),
        940 => Some(KeyCode::ArrowUp),
        _ => None,
    }
}

fn original(map_id: u32, raw: &str) -> EventCommand {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    map.events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .find(|command| command.code == 10110 && command.string == raw)
        .expect("original slow dialogue header")
        .clone()
}

fn start(world: &mut World, case: u32, message: EventCommand, fps: f64) {
    assert!(!world.resource::<Dialogue>().active);
    assert!(!world.resource::<RunningEvent>().active());
    world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / fps),
    ));
    world.init_resource::<Fixture>();
    let mut fixture = world.resource_mut::<Fixture>();
    fixture.case = case;
    fixture.started = None;
    world.resource_mut::<RunningEvent>().start(
        0,
        vec![
            message,
            EventCommand {
                code: 10210,
                indent: 0,
                string: String::new(),
                params: vec![0, 9000 + case as i32, 9000 + case as i32, 0],
            },
        ],
    );
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    if let Some(label) = prompts::drive(world, frame) {
        return Some(label);
    }
    match frame {
        260 => start(world, 1, original(183, "\\S[5]Tiffany"), 60.0),
        350 => start(world, 2, original(220, "\\S[5]\\N[1]"), 144.0),
        490 => start(
            world,
            3,
            EventCommand {
                code: 10110,
                indent: 0,
                string: "ab\\|c\\^".into(),
                params: Vec::new(),
            },
            144.0,
        ),
        700 => {
            assert!(!world.resource::<Dialogue>().active);
            assert!(!world.resource::<RunningEvent>().active());
            for switch in 9001..=9003 {
                assert!(world.resource::<Switches>().get(switch));
            }
        }
        _ => {}
    }
    let case = world.get_resource::<Fixture>()?.case;
    let clock = world.resource::<GameFrames>().frame;
    let dialogue = world.resource::<Dialogue>();
    let Some(reveal) = dialogue.reveal.as_ref() else {
        if case == 3
            && let Some(started) = world.resource::<Fixture>().started
            && world.resource::<Fixture>().completed & 4 == 0
        {
            assert!(!dialogue.active);
            assert_eq!(clock.wrapping_sub(started) + 1, 67);
            world.resource_mut::<Fixture>().completed |= 4;
            return Some("dialogue-auto-closed");
        }
        return None;
    };
    let actual = reveal.text().to_string();
    let complete = reveal.is_complete();
    let started = *world.resource_mut::<Fixture>().started.get_or_insert(clock);
    let age = clock.wrapping_sub(started) + 1;
    let expected = if case < 3 {
        let name = if case == 1 { "Tiffany" } else { "Ron" };
        assert_eq!(complete, age >= name.len() as u32 * 3);
        name.chars()
            .take(((age - 1) / 3 + 1) as usize)
            .collect::<String>()
    } else {
        assert!(age < 67);
        assert!(!complete);
        if age < 63 { "ab" } else { "abc" }.to_string()
    };
    assert_eq!(actual, expected, "case {case}, logical tick {age}");
    assert!(!world.resource::<Switches>().get(9000 + case));
    verify_view(world, &actual);
    let mut fixture = world.resource_mut::<Fixture>();
    fixture.checks += 1;
    if complete {
        fixture.completed |= 1 << (case - 1);
    }
    let snapshot = match (case, age) {
        (1, 1) => Some((1, "dialogue-tiffany-first")),
        (1, 10) => Some((2, "dialogue-tiffany-partial")),
        (1, 21) => Some((4, "dialogue-tiffany-complete")),
        (2, 9) => Some((8, "dialogue-ron-144fps")),
        (3, 30) => Some((16, "dialogue-long-pause")),
        _ => None,
    };
    snapshot.and_then(|(bit, label)| {
        let fresh = fixture.captures & bit == 0;
        fixture.captures |= bit;
        fresh.then_some(label)
    })
}

fn verify_view(world: &mut World, expected: &str) {
    let (text, visible) = world
        .query_filtered::<
            (&crate::font::bitmap::PixelText, &InheritedVisibility),
            With<view::DialogueText>,
        >()
        .single(world)
        .unwrap();
    assert_eq!(text.runs.len(), 1);
    assert_eq!(text.runs[0].text, expected);
    assert!(visible.get());
    assert_eq!(
        *world
            .query_filtered::<&Visibility, With<view::DialoguePanel>>()
            .single(world)
            .unwrap(),
        Visibility::Visible
    );
}

pub(crate) fn verify_finished(world: &World) {
    prompts::verify_finished(world);
    let fixture = world.resource::<Fixture>();
    assert_eq!(fixture.completed, 7);
    assert_eq!(fixture.captures, 31);
    assert!(fixture.checks > 200);
    for switch in 9001..=9003 {
        assert!(world.resource::<Switches>().get(switch));
    }
    info!(
        "dialogue timing: {} rendered-state checks, original names at 60/144 FPS, 67-tick auto-close",
        fixture.checks
    );
}

use super::*;
use crate::dialogue::{MessageOptions, MessagePosition, MessageTransparent, PromptClock};
use crate::interpreter::RunningEvent;
use crate::state::{Switches, Variables};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod pixels;
mod source;

pub(crate) use pixels::snapshot;

const LABELS: [&str; 10] = [
    "embedded-choice-typing",
    "embedded-choice-a",
    "embedded-choice-b",
    "embedded-number-a",
    "embedded-number-b",
    "embedded-three-lines-a",
    "embedded-three-lines-b",
    "standalone-choice-typing",
    "standalone-choice-a",
    "standalone-choice-b",
];

#[derive(Resource)]
struct Probe {
    black: Entity,
    captured: u16,
    pixels: Arc<AtomicUsize>,
    checked: u32,
}

pub(crate) fn input(world: &mut World, frame: u32) -> bool {
    if frame < 1900 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    *keys = default();
    let pressed = match frame {
        1902 | 1903 | 2032 | 2033 | 2162 | 2163 | 2140 | 2270 | 2322 | 2323 => {
            &[KeyCode::Enter][..]
        }
        2010 | 2430 => &[KeyCode::ArrowDown, KeyCode::Enter],
        2130 => &[KeyCode::ArrowDown],
        2260 => &[KeyCode::ArrowUp],
        _ => &[],
    };
    for key in pressed {
        keys.press(*key);
    }
    true
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1900 {
        let black = world
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                GlobalZIndex(99),
            ))
            .id();
        world.insert_resource(Probe {
            black,
            captured: 0,
            pixels: Arc::default(),
            checked: 0,
        });
        world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
    }
    if frame == 2490 {
        let black = world.resource::<Probe>().black;
        world.despawn(black);
    }
    let case = match frame {
        1900..=2020 => 0,
        2030..=2150 => 1,
        2160..=2280 => 2,
        2320..=2440 => 3,
        _ => return None,
    };
    let age = frame - [1900, 2030, 2160, 2320][case];
    if age == 0 {
        assert!(!world.resource::<Dialogue>().active);
        assert!(!world.resource::<RunningEvent>().active());
        world.insert_resource(
            [
                MessagePosition::Bottom,
                MessagePosition::Top,
                MessagePosition::Middle,
                MessagePosition::Bottom,
            ][case],
        );
        world.insert_resource(MessageTransparent(case == 2));
        world.resource_mut::<MessageOptions>().fixed = true;
        if case == 0 || case == 3 {
            world.resource_mut::<Variables>().set(9013, -1);
        }
        world
            .resource_mut::<RunningEvent>()
            .start(0, source::commands(case));
        return None;
    }
    if age == 120 {
        assert!(!world.resource::<Dialogue>().active);
        assert!(!world.resource::<RunningEvent>().active());
        assert!(world.resource::<Switches>().get(9015 + case as u32));
        let (variable, expected) = [(9013, 1), (24, 9), (9014, 1), (9013, 1)][case];
        assert_eq!(world.resource::<Variables>().get(variable), expected);
    }
    if age >= 100 {
        return None;
    }
    let dialogue = world.resource::<Dialogue>();
    assert!(dialogue.active);
    assert!(!world.resource::<Switches>().get(9015 + case as u32));
    let reveal = dialogue.reveal.as_ref()?;
    assert!(!reveal.arrow_visible());
    let partial = (case == 0 && reveal.text() == "Ron\n(S") || (case == 3 && reveal.text() == "(S");
    let capture = if partial {
        if case == 0 { 0 } else { 7 }
    } else if dialogue.prompt_input_ready() {
        let numeric = case == 1 || case == 2;
        let phase = usize::from(world.resource::<PromptClock>().source_x(0, numeric) == 96.0);
        if case == 3 {
            8 + phase
        } else {
            1 + case * 2 + phase
        }
    } else {
        assert!(!world.resource::<Choice>().active());
        assert!(!world.resource::<InputNumber>().active());
        world.resource_mut::<Probe>().checked += 1;
        return None;
    };
    let mut probe = world.resource_mut::<Probe>();
    probe.checked += 1;
    if probe.captured & (1 << capture) != 0 {
        return None;
    }
    probe.captured |= 1 << capture;
    Some(LABELS[capture])
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.captured, 1023);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 10);
    assert!(probe.checked > 300);
    assert!(!world.resource::<Dialogue>().active);
    assert!(!world.resource::<RunningEvent>().active());
    info!(
        "typed prompts: {} typed/ready states, 768000 reference pixels and four exact result handoffs",
        probe.checked
    );
}

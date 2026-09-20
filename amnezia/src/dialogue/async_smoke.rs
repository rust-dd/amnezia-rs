use super::{Dialogue, MessageOptions, MessagePosition, MessageTransparent};
use crate::choice::Choice;
use crate::inputnumber::InputNumber;
use crate::interpreter::RunningEvent;
use crate::state::Variables;
use crate::timing::{GameFrames, SceneFrames};
use crate::transitions::{Kind, Transition};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

mod source;

#[derive(Clone, Copy)]
enum Stage {
    Typing,
    Transition,
    Frozen(u32),
    Resuming,
    Settled,
    Finished,
}

#[derive(Resource)]
struct Probe {
    black: Entity,
    stage: Stage,
    kind: usize,
    scene: u32,
    text: String,
    completed: u8,
    checks: u32,
}

pub(crate) fn input(world: &mut World, frame: u32) -> bool {
    if frame < 2520 {
        return false;
    }
    let press = world.get_resource::<Probe>().is_some_and(|probe| {
        matches!(probe.stage, Stage::Transition)
            || (matches!(probe.stage, Stage::Resuming) && probe.kind > 0)
    });
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    *keys = default();
    if press {
        keys.press(KeyCode::ArrowDown);
        keys.press(KeyCode::Enter);
    }
    true
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 2520 {
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
            stage: Stage::Finished,
            kind: 0,
            scene: 0,
            text: String::new(),
            completed: 0,
            checks: 0,
        });
    }
    if frame == 2960 {
        let black = world.resource::<Probe>().black;
        world.despawn(black);
    }
    if !(2520..2940).contains(&frame) {
        return None;
    }
    let kind = ((frame - 2520) / 140) as usize;
    let age = (frame - 2520) % 140;
    if age == 0 {
        assert!(matches!(world.resource::<Probe>().stage, Stage::Finished));
        assert!(!world.resource::<RunningEvent>().active());
        world.resource_mut::<Probe>().kind = kind;
        world.resource_mut::<Probe>().stage = Stage::Typing;
        world.insert_resource(MessagePosition::Bottom);
        world.insert_resource(MessageTransparent(false));
        world.resource_mut::<MessageOptions>().fixed = true;
        time(world, true);
        if kind > 0 {
            world
                .resource_mut::<Variables>()
                .set(9018 + kind as u32, -1);
        }
        world
            .resource_mut::<RunningEvent>()
            .start(0, source::commands(kind));
        return None;
    }
    if age == 20 {
        let text = world
            .resource::<Dialogue>()
            .reveal
            .as_ref()
            .map_or("", |reveal| reveal.text())
            .to_string();
        let scene = world.resource::<SceneFrames>().frame;
        let raw = world.resource::<GameFrames>().frame;
        let mut probe = world.resource_mut::<Probe>();
        probe.text = text;
        probe.scene = scene;
        probe.stage = Stage::Transition;
        assert!(world.resource_mut::<Transition>().start_for(
            Kind::Fade,
            false,
            raw,
            IVec2::new(160, 120),
            if kind == 0 { 36 } else { 6 },
        ));
    }
    if age == 95 {
        assert!(matches!(world.resource::<Probe>().stage, Stage::Settled));
        if kind == 0 {
            world.resource_mut::<Dialogue>().close();
        } else {
            assert!(!world.resource::<RunningEvent>().active());
            assert_eq!(
                world.resource::<Variables>().get(9018 + kind as u32),
                if kind == 1 { 1 } else { 9 }
            );
        }
        let mut probe = world.resource_mut::<Probe>();
        probe.completed |= 1 << kind;
        probe.stage = Stage::Finished;
    }
    let stage = world.resource::<Probe>().stage;
    match stage {
        Stage::Transition | Stage::Frozen(_) => {
            verify_held(world);
            world.resource_mut::<Probe>().checks += 1;
            if matches!(stage, Stage::Transition) && !world.resource::<Transition>().busy() {
                time(world, false);
                world.resource_mut::<Probe>().stage = Stage::Frozen(frame + 12);
            } else if let Stage::Frozen(until) = stage
                && frame >= until
            {
                time(world, true);
                world.resource_mut::<Probe>().stage = Stage::Resuming;
                return Some(
                    [
                        "dialogue-async-text",
                        "dialogue-async-choice",
                        "dialogue-async-number",
                    ][kind],
                );
            }
        }
        Stage::Resuming => {
            let probe = world.resource::<Probe>();
            assert_eq!(
                world.resource::<SceneFrames>().frame,
                probe.scene.wrapping_add(1)
            );
            if kind == 0 {
                let expected = "abcdefghij"
                    .repeat(12)
                    .chars()
                    .take(probe.text.len() + 2)
                    .collect::<String>();
                assert_eq!(
                    world.resource::<Dialogue>().reveal.as_ref().unwrap().text(),
                    expected
                );
            }
            world.resource_mut::<Probe>().stage = Stage::Settled;
        }
        _ => {}
    }
    None
}

fn verify_held(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(world.resource::<SceneFrames>().frame, probe.scene);
    let dialogue = world.resource::<Dialogue>();
    assert_eq!(
        dialogue.reveal.as_ref().map_or("", |reveal| reveal.text()),
        probe.text
    );
    if probe.kind == 1 {
        let choice = world.resource::<Choice>();
        assert!(choice.active());
        assert_eq!((choice.cursor, choice.result), (0, None));
    } else if probe.kind == 2 {
        let number = world.resource::<InputNumber>();
        assert!(number.active());
        assert_eq!((number.value, number.result), (0, None));
    }
}

fn time(world: &mut World, running: bool) {
    world.insert_resource(TimeUpdateStrategy::ManualDuration(if running {
        Duration::from_secs_f64(1.0 / 60.0)
    } else {
        Duration::ZERO
    }));
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.completed, 7);
    assert!(probe.checks >= 84);
    assert!(!world.resource::<Dialogue>().active);
    assert!(!world.resource::<Choice>().active());
    assert!(!world.resource::<InputNumber>().active());
    assert!(!world.resource::<RunningEvent>().active());
    info!(
        "asynchronous messages: {} held states, three frozen-window captures, exact reveal resume and choice/number handoffs",
        probe.checks
    );
}

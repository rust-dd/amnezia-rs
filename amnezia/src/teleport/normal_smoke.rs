use crate::choice::Choice;
use crate::dialogue::{Dialogue, MessagePosition, MessageTransparent};
use crate::inputnumber::InputNumber;
use crate::interpreter::{CommonEvents, RunningEvent};
use crate::state::{Inventory, Switches, Variables};
use crate::world::MapData;
use amnezia_data::{CommonEvent, EventCommand};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

mod pixels;
pub(crate) use pixels::snapshot;

#[derive(Resource)]
struct Probe {
    black: Entity,
    resume: Option<u32>,
    completed: u8,
    closing: bool,
    checks: u32,
    gold: i32,
    pixels: Arc<AtomicUsize>,
}

fn command(code: u32, indent: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        params,
        string: String::new(),
    }
}

pub(crate) fn entry() -> Vec<EventCommand> {
    vec![command(10810, 0, vec![13, 60, 60])]
}

fn commands(case: u32) -> Vec<EventCommand> {
    let mut commands = match case {
        0 => vec![
            command(10140, 0, vec![2]),
            EventCommand {
                string: "Marad".into(),
                ..command(20140, 0, vec![0])
            },
            command(10220, 1, vec![0, 4699, 4699, 1, 0, 1]),
            EventCommand {
                string: "Megse".into(),
                ..command(20140, 0, vec![1])
            },
            command(10220, 1, vec![0, 4699, 4699, 1, 0, 1]),
            command(20141, 0, vec![]),
        ],
        1 => vec![command(10150, 0, vec![3, 4698])],
        2 => vec![
            command(10730, 0, vec![1, 30, 1]),
            command(20730, 0, vec![]),
            command(10220, 1, vec![0, 4699, 4699, 1, 0, 1]),
            command(20731, 0, vec![]),
            command(10220, 1, vec![0, 4699, 4699, 1, 0, 1]),
            command(20732, 0, vec![]),
        ],
        3 => vec![EventCommand {
            string: "Megmarad a parbeszed.".into(),
            ..command(10110, 0, vec![])
        }],
        _ => unreachable!(),
    };
    commands.push(command(
        10220,
        0,
        vec![0, 4700 + case as i32, 4700 + case as i32, 1, 0, 1],
    ));
    commands.push(command(
        10210,
        0,
        vec![0, 4700 + case as i32, 4700 + case as i32, 1],
    ));
    commands
}

fn time(world: &mut World, advancing: bool) {
    world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        if advancing {
            Duration::from_secs_f64(1.0 / 60.0)
        } else {
            Duration::ZERO
        },
    ));
}

fn start(world: &mut World, case: u32) {
    assert!(!world.resource::<Dialogue>().busy());
    assert!(!world.resource::<RunningEvent>().active());
    world.resource_mut::<Dialogue>().face = default();
    world.insert_resource(MessagePosition::Bottom);
    world.insert_resource(MessageTransparent(false));
    world
        .resource_mut::<crate::dialogue::MessageOptions>()
        .fixed = true;
    if case == 3 {
        world
            .resource_mut::<RunningEvent>()
            .start(7, commands(case));
    } else {
        world.resource_mut::<Switches>().set(4700 + case, true);
        world.resource_mut::<CommonEvents>().0.push(CommonEvent {
            id: 900 + case,
            name: "Transfer prompt probe".into(),
            trigger: 4,
            switch_flag: true,
            switch_id: 4700 + case,
            commands: commands(case),
        });
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
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
        let gold = world.resource::<Inventory>().gold();
        world.insert_resource(Probe {
            black,
            resume: None,
            completed: 0,
            closing: false,
            checks: 0,
            gold,
            pixels: default(),
        });
        world.resource_mut::<Variables>().set(4698, 77);
        world.resource_mut::<Variables>().set(4699, 0);
        for variable in 4700..4704 {
            world.resource_mut::<Variables>().set(variable, 0);
        }
        let mut settings = world.resource_mut::<crate::transitions::Settings>();
        settings.change(&[0, 20], &crate::transitions::Defaults([0; 6]));
        settings.change(&[1, 20], &crate::transitions::Defaults([0; 6]));
    }
    if !(300..1100).contains(&frame) {
        return None;
    }
    if world.resource::<Probe>().resume == Some(frame) {
        time(world, true);
        world.resource_mut::<Probe>().resume = None;
    }
    let case = (frame - 300) / 200;
    let age = (frame - 300) % 200;
    if age == 0 {
        start(world, case);
    }
    if age == 90 {
        assert!(world.resource::<Dialogue>().active);
        assert_eq!(world.resource::<Dialogue>().from_foreground, case == 3);
        assert_eq!(world.resource::<InputNumber>().active(), case == 1);
        assert_eq!(world.resource::<Choice>().active(), case == 0 || case == 2);
        let (map, x, y) = if case.is_multiple_of(2) {
            (4, 7, 6)
        } else {
            (13, 60, 60)
        };
        world.resource_mut::<super::PendingTeleport>().0 = Some((map, x, y));
    }
    if case == 2 && age > 90 && !world.resource::<Probe>().closing {
        let dialogue = world.resource::<Dialogue>();
        if !dialogue.active && dialogue.lifecycle.message.closing() {
            assert_eq!(dialogue.lifecycle.message.half_height(80), 34);
            assert_eq!(dialogue.lifecycle.gold.half_height(32), 13);
            assert!(!world.resource::<Choice>().active());
            world.resource_mut::<Probe>().closing = true;
            world.resource_mut::<Probe>().resume = Some(frame + 5);
            time(world, false);
            return Some("normal-transfer-inn-closing");
        }
    }
    if age == 130 {
        assert_eq!(
            world.resource::<MapData>().map_id,
            if case.is_multiple_of(2) { 4 } else { 13 }
        );
        assert!(!world.resource::<super::Fade>().busy());
        assert_eq!(world.resource::<Dialogue>().active, case == 3);
        assert_eq!(
            world.resource::<Variables>().get(4700 + case),
            i32::from(case != 3)
        );
        if case == 3 {
            assert_eq!(world.resource::<RunningEvent>().debug_id(), Some(0));
            return Some("normal-transfer-foreground");
        }
    }
    if age == 150 && case == 3 {
        world.resource_mut::<Dialogue>().close();
    }
    if age == 170 {
        assert!(!world.resource::<Dialogue>().busy());
        assert!(!world.resource::<Choice>().active());
        assert!(!world.resource::<InputNumber>().active());
        assert!(!world.resource::<crate::shop::inn::State>().active());
        assert_eq!(world.resource::<Variables>().get(4700 + case), 1);
        assert_eq!(world.resource::<Variables>().get(4699), 0);
        assert_eq!(world.resource::<Variables>().get(4698), 77);
        assert_eq!(
            world.resource::<Inventory>().gold(),
            world.resource::<Probe>().gold
        );
        world.resource_mut::<Probe>().completed |= 1 << case;
        if case == 3 {
            let black = world.resource::<Probe>().black;
            world.despawn(black);
        }
    }
    if age > 170 {
        assert_eq!(world.resource::<Variables>().get(4700 + case), 1);
        world.resource_mut::<Probe>().checks += 1;
    }
    (case == 3 && age == 185).then_some("normal-transfer-finished")
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.completed, 15);
    assert_eq!(probe.checks, 116);
    assert!(probe.closing);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 1);
    info!(
        "normal transfers: four prompt owners, 116 continuation checks and 76800 closing-window pixels verified"
    );
}

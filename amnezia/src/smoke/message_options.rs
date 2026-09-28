use amnezia_data::EventCommand;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct PositionChecks(u8);

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        string: String::new(),
        params,
    }
}

pub(super) fn entry() -> Vec<EventCommand> {
    vec![command(10810, vec![3, 15, 12])]
}

pub(super) fn drive(world: &mut World, frame: u32) {
    moving_position(world, frame);
    if matches!(frame, 260 | 405) {
        assert!(
            !world
                .resource::<crate::interpreter::RunningEvent>()
                .active()
        );
        world
            .resource_mut::<crate::interpreter::RunningEvent>()
            .start(
                0,
                vec![
                    command(10120, vec![0, 2, i32::from(frame == 260), 1]),
                    EventCommand {
                        code: 10110,
                        indent: 0,
                        string: "Árvíztűrő tükörfúrógép.\nHol vagyok?".into(),
                        params: vec![],
                    },
                ],
            );
    }
    if matches!(frame, 350 | 500) {
        let top = frame == 350;
        crate::dialogue::verify_placement(world, top);
        super::capture(
            world,
            if top {
                "message-auto-top"
            } else {
                "message-fixed-bottom"
            },
        );
    }
    if matches!(frame, 400 | 550) {
        world.resource_mut::<crate::dialogue::Dialogue>().close();
    }
}

fn moving_position(world: &mut World, frame: u32) {
    use crate::dialogue::Dialogue;
    use crate::interpreter::{CommonEvents, RunningEvent};
    if frame == 600 {
        assert!(!world.resource::<RunningEvent>().active());
        world.insert_resource(PositionChecks::default());
        world
            .resource_mut::<RunningEvent>()
            .start(0, vec![command(10810, vec![3, 4, 9])]);
    }
    if frame == 720 {
        assert!(!world.resource::<RunningEvent>().active());
        assert!(!world.resource::<Dialogue>().busy());
        let events = [
            vec![
                command(10120, vec![0, 2, 1, 1]),
                text("Ablaknyitás mozgás előtt."),
                command(11410, vec![1000]),
            ],
            vec![
                command(11330, vec![10001, 8, 0, 0, 0]),
                command(11410, vec![1000]),
            ],
        ];
        for (index, commands) in events.into_iter().enumerate() {
            world
                .resource_mut::<CommonEvents>()
                .0
                .push(amnezia_data::CommonEvent {
                    id: 910 + index as u32,
                    name: "Command-time message position".into(),
                    trigger: 4,
                    switch_flag: false,
                    switch_id: 0,
                    commands,
                });
        }
    }
    if frame == 790 {
        world.resource_mut::<Dialogue>().close();
    }
    if frame == 800 {
        world
            .resource_mut::<RunningEvent>()
            .start(0, vec![text("Új üzenet: új ablakhely.")]);
    }
    if matches!(frame, 760 | 840) {
        assert!(world.resource::<Dialogue>().active);
        let hero = world
            .query::<&crate::player::Player>()
            .single(world)
            .unwrap();
        assert_eq!((hero.tile_x, hero.tile_y), (4, 8));
        let top = frame == 760;
        crate::dialogue::verify_placement(world, top);
        world.resource_mut::<PositionChecks>().0 |= if top { 1 } else { 2 };
        super::capture(
            world,
            if top {
                "message-opening-position-held"
            } else {
                "message-next-position-refreshed"
            },
        );
    }
}

fn text(value: &str) -> EventCommand {
    EventCommand {
        string: value.into(),
        ..command(10110, vec![])
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<PositionChecks>().0, 3);
    info!(
        "message position: command-time placement retained after movement and refreshed for the next message"
    );
}

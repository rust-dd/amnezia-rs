use amnezia_data::EventCommand;
use bevy::prelude::*;

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

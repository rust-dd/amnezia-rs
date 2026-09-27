use amnezia_data::EventCommand;
use bevy::prelude::*;

mod walking;
pub(super) use walking::verify_finished;

#[derive(Resource)]
struct Checkpoint(Vec2);

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        string: String::new(),
        params,
    }
}

pub(super) fn entry() -> Vec<EventCommand> {
    vec![command(10810, vec![13, 60, 60])]
}

pub(super) fn drive(world: &mut World, frame: u32) {
    walking::drive(world, frame);
    if frame == 260 {
        assert!(
            !world
                .resource::<crate::interpreter::RunningEvent>()
                .active()
        );
        let position = world
            .resource::<crate::player::CameraPan>()
            .position
            .unwrap();
        world.insert_resource(Checkpoint(position));
        world
            .resource_mut::<crate::interpreter::RunningEvent>()
            .start(
                0,
                vec![
                    command(11060, vec![0, 0, 1, 4, 1]),
                    command(11330, vec![10001, 8, 0, 0, 36, 1, 1, 1, 1, 37]),
                    command(11340, vec![]),
                    command(11060, vec![2, 2, 3, 3, 1]),
                    command(11410, vec![5]),
                    command(11060, vec![3, 0, 1, 6, 1]),
                    command(11060, vec![1, 0, 1, 4, 1]),
                ],
            );
    }
    if frame == 370 {
        let pan = world.resource::<crate::player::CameraPan>();
        assert!(pan.locked);
        assert_eq!(pan.offset, Vec2::new(0.0, -48.0));
        assert_eq!(
            pan.position.unwrap(),
            world.resource::<Checkpoint>().0 + pan.offset
        );
        super::capture(world, "camera-locked-pan");
    }
    if frame == 440 {
        let pan = world.resource::<crate::player::CameraPan>();
        assert!(!pan.locked);
        assert_eq!(pan.offset, Vec2::ZERO);
        assert_eq!(pan.position.unwrap(), world.resource::<Checkpoint>().0);
        let hero = world
            .query::<&crate::player::Player>()
            .single(world)
            .unwrap();
        assert_eq!((hero.tile_x, hero.tile_y), (64, 60));
        super::capture(world, "camera-returned");
    }
}

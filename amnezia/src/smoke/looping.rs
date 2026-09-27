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
    vec![
        command(10810, vec![13, 0, 0]),
        command(10850, vec![2, 0, 13, 0, 0]),
        command(10840, vec![]),
        command(11330, vec![10004, 8, 0, 0, 3, 0]),
        command(11340, vec![]),
    ]
}

pub(super) fn drive(world: &mut World, frame: u32) {
    if frame == 220 {
        assert_eq!(world.resource::<crate::world::MapData>().map_id, 13);
    }
    if frame == 350 {
        let position = world
            .resource::<crate::vehicles::Vehicles>()
            .character(10004)
            .unwrap();
        assert_eq!((position.0, position.1), (139, 139));
        let hero = world
            .query::<&crate::player::Player>()
            .single(world)
            .unwrap();
        assert_eq!((hero.tile_x, hero.tile_y), (139, 139));
        assert_eq!(
            world.resource::<crate::vehicles::Vehicles>().save.riding,
            Some(2)
        );
        super::capture(world, "looping-world-corner");
    }
    if frame == 500 {
        world
            .resource_mut::<crate::vehicles::Vehicles>()
            .save
            .riding = None;
        world
            .resource_mut::<crate::interpreter::RunningEvent>()
            .start(
                0,
                vec![
                    command(10810, vec![87, 10, 0]),
                    command(11330, vec![10001, 8, 0, 0, 36, 0, 37]),
                    command(11340, vec![]),
                ],
            );
    }
    if frame == 700 {
        let hero = world
            .query::<&crate::player::Player>()
            .single(world)
            .unwrap();
        assert_eq!((hero.tile_x, hero.tile_y), (10, 29));
        assert_eq!(world.resource::<crate::world::MapData>().map_id, 87);
        super::capture(world, "looping-vertical-seam");
    }
}

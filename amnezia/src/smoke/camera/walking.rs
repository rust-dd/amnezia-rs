use crate::player::{CameraPan, Player};
use crate::world::{MoveQueue, RouteStepper};
use bevy::prelude::*;

#[derive(Resource)]
struct Probe {
    origin: Vec2,
    checks: u32,
}

pub(super) fn drive(world: &mut World, frame: u32) {
    if frame == 500 {
        assert!(
            !world
                .resource::<crate::interpreter::RunningEvent>()
                .active()
        );
        world.resource_mut::<CameraPan>().recenter(false);
    }
    if frame == 510 {
        let origin = world.resource::<CameraPan>().position.unwrap();
        world.insert_resource(Probe { origin, checks: 0 });
        let (queue, mut route) = world
            .query_filtered::<(&MoveQueue, &mut RouteStepper), With<Player>>()
            .single_mut(world)
            .unwrap();
        assert!(!queue.busy());
        route.set_speed(1);
        route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 36, 1, 37]));
    }
    if (511..=585).contains(&frame) {
        let offset = if frame <= 573 {
            (frame - 510) as f32 / 4.0
        } else {
            23.75
        };
        let expected = world.resource::<Probe>().origin + Vec2::X * offset;
        assert_eq!(world.resource::<CameraPan>().position, Some(expected));
        world.resource_mut::<Probe>().checks += 1;
    }
    if frame == 573 {
        world
            .query_filtered::<&mut RouteStepper, With<Player>>()
            .single_mut(world)
            .unwrap()
            .set_speed(6);
    }
    if frame == 574 {
        assert!(
            !world
                .query_filtered::<&MoveQueue, With<Player>>()
                .single(world)
                .unwrap()
                .busy()
        );
    }
    match frame {
        515 => super::super::capture(world, "camera-slow-walk"),
        574 => super::super::capture(world, "camera-walk-overshoot"),
        585 => super::super::capture(world, "camera-walk-stopped"),
        _ => {}
    }
}

pub(in crate::smoke) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().checks, 75);
    info!(
        "walking camera: 75 exact slow-step, live-acceleration and stopped-state checks verified"
    );
}

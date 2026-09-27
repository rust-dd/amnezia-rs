use crate::picture::PictureCommand;
use crate::player::{CameraPan, Player};
use crate::world::{MoveQueue, RouteStepper};
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

mod pixels;
pub(in crate::smoke) use pixels::snapshot;

#[derive(Resource)]
struct Probe {
    origin: Vec2,
    effects: Vec2,
    checks: u32,
    pixels: Arc<AtomicU32>,
}

pub(super) fn drive(world: &mut World, frame: u32) {
    if frame == 600 {
        world.resource_mut::<CameraPan>().recenter(false);
    }
    if frame == 610 {
        for (id, x, fixed) in [(11, 96.0, 1), (12, 224.0, 0)] {
            world.write_message(PictureCommand::show(
                id,
                "Cross",
                x,
                60.0,
                &[0, 0, 0, 0, fixed, 800, 0, 0, 100, 100, 100, 100, 0, 0],
            ));
        }
    }
    if frame == 620 {
        let camera = world.resource::<CameraPan>();
        world.insert_resource(Probe {
            origin: camera.position.unwrap(),
            effects: camera.effects_position().unwrap(),
            checks: 0,
            pixels: Arc::default(),
        });
        world.resource_mut::<CameraPan>().command(&[2, 1, 1, 1, 0]);
    }
    if (621..=690).contains(&frame) {
        let scrolled = (frame - 620).min(64) as f32 / 4.0;
        let correction = if frame >= 657 { 7.0 } else { 0.0 };
        let probe = world.resource::<Probe>();
        let pan = world.resource::<CameraPan>();
        assert_eq!(
            pan.position,
            Some(probe.origin + Vec2::X * (scrolled + correction))
        );
        assert_eq!(
            pan.effects_position(),
            Some(probe.effects + Vec2::X * scrolled)
        );
        assert_eq!(pan.offset, Vec2::X * scrolled);
        world.resource_mut::<Probe>().checks += 1;
    }
    if frame == 653 {
        let (queue, mut route) = world
            .query_filtered::<(&MoveQueue, &mut RouteStepper), With<Player>>()
            .single_mut(world)
            .unwrap();
        assert!(!queue.busy());
        route.set_speed(6);
        route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 24, 25]));
    }
    if (654..=660).contains(&frame) {
        let queue = world
            .query_filtered::<&MoveQueue, With<Player>>()
            .single(world)
            .unwrap();
        assert_eq!(queue.busy(), frame < 657);
    }
    match frame {
        653 => super::super::capture(world, "camera-jump-before"),
        657 => super::super::capture(world, "camera-jump-landed"),
        690 => super::super::capture(world, "camera-jump-pan-finished"),
        _ => {}
    }
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.checks, 70);
    assert_eq!(probe.pixels.load(Ordering::SeqCst), 3);
    info!(
        "jump camera: 70 exact camera/effect/pan states and 54 original picture samples verified"
    );
}

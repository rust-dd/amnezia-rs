use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    position: Vec2,
    effects: Vec2,
    pan: Vec2,
    moving: bool,
    ground: Vec2,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn accelerate(trace: Res<Trace>, mut heroes: Query<&mut RouteStepper, With<Player>>) {
    if trace.0.len() == 10 {
        heroes.single_mut().unwrap().set_speed(6);
    }
}

fn observe(
    camera: Res<CameraPan>,
    map: Res<MapData>,
    heroes: Query<(&Player, &MoveQueue)>,
    mut trace: ResMut<Trace>,
) {
    let (hero, queue) = heroes.single().unwrap();
    trace.0.push(Sample {
        position: camera.position.unwrap(),
        effects: camera.effects_position().unwrap(),
        pan: camera.offset,
        moving: queue.busy(),
        ground: queue.ground_position(hero, &map),
    });
}

fn run(fps: u32) -> Vec<Sample> {
    let (mut app, hero, _) = super::super::walking::fixture((20, 15));
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 1, 0]);
    for _ in 0..33 {
        app.update();
    }
    app.add_plugins((TimingPlugin, LogicalPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<Trace>()
        .add_systems(
            Update,
            (
                accelerate.before(crate::interpreter::ParallelStep),
                observe
                    .after(CameraFollow)
                    .before(crate::dialogue::MessageUpdate),
            ),
        );
    app.update();
    app.world_mut().resource_mut::<Trace>().0.clear();
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(1);
    route.force_route(RouteStepper::from_move_event(&[
        10001, 8, 0, 0, 24, 1, 1, 2, 25,
    ]));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
    }
    app.world_mut().remove_resource::<Trace>().unwrap().0
}

#[test]
fn jumping_pan_and_live_acceleration_have_identical_traces_at_all_render_rates() {
    let expected = run(60);
    assert_eq!(expected.len(), 60);
    let mut reference = Reference {
        display: [11 * 256 + 33 * 4, 8 * 256],
        pan: [9 * 256 - 33 * 4, 7 * 256],
        destination: [22 * 256, 16 * 256],
        delta: [2, 1],
        remaining: 256,
        amount: 8,
    };
    for (index, sample) in expected.iter().take(13).enumerate() {
        if index == 10 {
            reference.amount = 64;
        }
        reference.update();
        assert_eq!(sample.position, reference.camera());
        assert_eq!(sample.moving, reference.remaining > 0);
    }
    assert_ne!(expected[12].position, expected[12].effects);
    assert!(!expected[12].moving);
    for fps in [15, 30, 144] {
        assert_eq!(run(fps), expected, "{fps} FPS");
    }
}

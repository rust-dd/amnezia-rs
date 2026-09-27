use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    position: Vec2,
    pan: Vec2,
    moving: bool,
    ground: Vec2,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn accelerate(trace: Res<Trace>, mut heroes: Query<&mut RouteStepper, With<Player>>) {
    if trace.0.len() == 31 {
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
        pan: camera.offset,
        moving: queue.busy(),
        ground: queue.ground_position(hero, &map),
    });
}

fn run(fps: u32) -> (Vec2, Vec<Sample>) {
    let (mut app, hero, origin) = fixture((20, 15));
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
    force(&mut app, hero, 1, 1);
    let mut pan = app.world_mut().resource_mut::<CameraPan>();
    pan.target = Vec2::Y * 16.0;
    pan.speed = 15.0;
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
    }
    (
        origin,
        app.world_mut().remove_resource::<Trace>().unwrap().0,
    )
}

#[test]
fn walking_pan_and_live_acceleration_have_identical_traces_at_all_render_rates() {
    let (origin, expected) = run(60);
    assert_eq!(expected.len(), 60);
    assert_eq!(expected[30].position, origin + Vec2::splat(7.75));
    assert_eq!(expected[31].position, origin + Vec2::new(15.75, 8.0));
    assert_eq!(expected[32].position, origin + Vec2::new(23.75, 8.25));
    assert_eq!(
        expected.last().unwrap().position,
        origin + Vec2::new(23.75, 15.0)
    );
    for fps in [15, 30, 144] {
        assert_eq!(run(fps).1, expected, "{fps} FPS");
    }
}

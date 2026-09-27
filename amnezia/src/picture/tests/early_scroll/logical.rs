use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    pan: Vec2,
    pictures: Vec<Vec2>,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn observe(camera: Res<CameraPan>, pictures: Query<&Picture>, mut trace: ResMut<Trace>) {
    let mut pictures = pictures.iter().collect::<Vec<_>>();
    pictures.sort_by_key(|picture| picture.id);
    trace.0.push(Sample {
        pan: camera.offset,
        pictures: pictures
            .into_iter()
            .map(|picture| picture.world_anchor.unwrap() - camera.effects_position().unwrap())
            .collect(),
    });
}

fn run(fps: u32) -> Vec<Sample> {
    let (mut app, _) = super::super::scroll::fixture();
    app.add_plugins((TimingPlugin, LogicalPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<Trace>()
        .add_systems(
            Update,
            observe
                .after(crate::player::CameraFollow)
                .before(crate::dialogue::MessageUpdate),
        );
    app.update();
    app.world_mut().resource_mut::<Trace>().0.clear();
    install(
        &mut app,
        vec![
            event(1, (19, 15), Some(1)),
            event(2, (20, 14), Some(2)),
            event(3, (30, 15), None),
        ],
    );
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 4, 0]);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / f64::from(fps),
    )));
    for _ in 0..fps {
        app.update();
    }
    app.world_mut().remove_resource::<Trace>().unwrap().0
}

#[test]
fn early_picture_anchors_and_repeated_pan_have_identical_logical_traces_at_all_render_rates() {
    let expected = run(60);
    assert_eq!(expected.len(), 60);
    assert_eq!(
        expected[0].pictures,
        [
            Vec2::new(-6.0, 0.0),
            Vec2::new(-4.0, 0.0),
            Vec2::new(-2.0, 0.0)
        ]
    );
    assert_eq!(expected[0].pan.x, 6.0);
    assert_eq!(expected[1].pan.x, 12.0);
    assert_eq!(expected[2].pan.x, 16.0);
    assert_eq!(
        expected.last().unwrap().pictures,
        [
            Vec2::new(-16.0, 0.0),
            Vec2::new(-14.0, 0.0),
            Vec2::new(-12.0, 0.0)
        ]
    );
    for fps in [15, 30, 144] {
        assert_eq!(run(fps), expected, "{fps} FPS");
    }
}

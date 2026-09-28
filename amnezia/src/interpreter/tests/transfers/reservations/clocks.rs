use super::*;
use crate::timing::{SceneFrames, TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Sample {
    raw: u32,
    scene: u32,
    counts: [i32; 4],
    age: u32,
    transferring: bool,
}

#[derive(Resource, Default)]
struct Trace(Vec<Sample>);

fn record(
    raw: Res<GameFrames>,
    scene: Res<SceneFrames>,
    variables: Res<Variables>,
    transition: Res<Transition>,
    fade: Res<Fade>,
    mut trace: ResMut<Trace>,
) {
    trace.0.push(Sample {
        raw: raw.frame,
        scene: scene.frame,
        counts: std::array::from_fn(|index| variables.get(index as u32 + 1)),
        age: transition.age(),
        transferring: fade.busy(),
    });
}

#[test]
fn reserved_transfers_start_after_one_full_logical_visit_at_every_render_rate() {
    for fps in [15, 30, 60, 144] {
        let mut app = app();
        app.add_plugins((TimingPlugin, LogicalPlugin))
            .init_resource::<Trace>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        crate::timing::logical::post(&mut app, || record.after(crate::teleport::TransferCommit));
        app.update();
        app.world_mut().resource_mut::<Trace>().0.clear();
        app.insert_resource(CommonEvents(vec![
            common(
                1,
                4,
                0,
                vec![
                    increment(1),
                    cmd(10810, 0, vec![3, 7, 8]),
                    increment(2),
                    cmd(11410, 0, vec![100]),
                ],
            ),
            common(2, 4, 0, vec![increment(3)]),
        ]));
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(7, vec![increment(4)]);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / f64::from(fps),
        )));
        for _ in 0..fps {
            app.update();
        }
        let trace = &app.world().resource::<Trace>().0;
        assert_eq!(trace.len(), 60, "{fps} FPS");
        let expected = (1_u32..=36)
            .map(|raw| Sample {
                raw,
                scene: 1,
                counts: [1; 4],
                age: raw.saturating_sub(2),
                transferring: true,
            })
            .collect::<Vec<_>>();
        assert_eq!(trace[..36], expected, "{fps} FPS");
    }
}

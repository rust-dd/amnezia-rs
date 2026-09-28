use super::*;
use crate::timing::{SceneFrames, TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Resource, Default)]
struct Trace(Vec<(u32, u32, [i32; 3])>);

fn record(
    frames: Res<GameFrames>,
    scene: Res<SceneFrames>,
    vars: Res<Variables>,
    mut trace: ResMut<Trace>,
) {
    trace.0.push((
        frames.frame,
        scene.frame,
        [vars.get(1), vars.get(2), vars.get(3)],
    ));
}

#[test]
fn async_continuations_keep_identical_sixty_tick_traces_at_every_render_rate() {
    for parallel in [false, true] {
        for fps in [15, 30, 60, 144] {
            let mut app = interp_app();
            app.add_plugins((TimingPlugin, LogicalPlugin, TransitionPlugin))
                .init_resource::<Trace>()
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
                .add_systems(Update, record.after(super::super::super::InterpreterStep));
            app.update();
            app.world_mut().resource_mut::<Trace>().0.clear();
            let owner = vec![
                cmd(11010, 0, vec![0]),
                increment(2),
                cmd(11410, 0, vec![100]),
            ];
            let mut events = vec![common(1, 4, 0, vec![increment(1)])];
            if parallel {
                events.push(common(2, 4, 0, owner));
            } else {
                app.world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(7, owner);
            }
            events.push(common(3, 4, 0, vec![increment(3)]));
            app.insert_resource(CommonEvents(events)).insert_resource(
                TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / f64::from(fps))),
            );
            for _ in 0..fps {
                app.update();
            }
            let expected = (1..=60)
                .flat_map(|raw| {
                    let visits = if raw <= 36 { 1 } else { raw - 35 };
                    let later = if parallel && raw <= 36 { 0 } else { visits };
                    let mut entries = Vec::new();
                    if raw == 37 {
                        entries.push((36, 1, [1, 1, 1]));
                    }
                    entries.push((
                        raw,
                        visits,
                        [visits as i32, i32::from(raw >= 37), later as i32],
                    ));
                    entries
                })
                .collect::<Vec<_>>();
            assert_eq!(
                app.world().resource::<Trace>().0,
                expected,
                "{fps} FPS, parallel={parallel}"
            );
        }
    }
}

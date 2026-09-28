use super::*;
use crate::timing::{SceneFrames, TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;

#[derive(Resource, Default)]
struct Trace(Vec<(u32, u32, [i32; 3])>);

fn record(
    frames: Res<GameFrames>,
    scene: Res<SceneFrames>,
    variables: Res<Variables>,
    mut trace: ResMut<Trace>,
) {
    trace.0.push((
        frames.frame,
        scene.frame,
        std::array::from_fn(|i| variables.get(i as u32 + 1)),
    ));
}

#[test]
fn free_inn_continuations_keep_identical_traces_at_every_render_rate() {
    for owner in 0..3 {
        for fps in [15, 30, 60, 144] {
            let mut app = interpreter_app();
            app.add_plugins((TimingPlugin, LogicalPlugin))
                .init_resource::<Trace>()
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
                .add_systems(Update, record.after(crate::interpreter::InterpreterStep));
            app.update();
            app.world_mut().resource_mut::<Trace>().0.clear();
            let commands = vec![
                command(10730, vec![0, 0, 1]),
                increment(2),
                command(11410, vec![100]),
            ];
            let mut events = vec![common(1, vec![increment(1)])];
            match owner {
                0 => app
                    .world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(7, commands),
                1 => events.push(common(2, commands)),
                _ => {
                    app.insert_resource(crate::world::MapEvents {
                        events: vec![map_event(commands)],
                    });
                }
            }
            events.push(common(3, vec![increment(3)]));
            app.insert_resource(CommonEvents(events)).insert_resource(
                TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / f64::from(fps))),
            );
            for _ in 0..fps * 2 {
                app.update();
            }
            let expected = (1..=119)
                .flat_map(|raw| {
                    let visits = if raw <= 71 { 1 } else { raw - 70 };
                    let later = if owner == 1 && raw <= 71 { 0 } else { visits };
                    let mut entries = Vec::new();
                    if raw == 37 {
                        entries.push((36, 1, [1, 0, i32::from(owner != 1)]));
                    }
                    if raw == 72 {
                        entries.push((71, 1, [1, 1, 1]));
                    }
                    entries.push((
                        raw,
                        visits,
                        [visits as i32, i32::from(raw >= 72), later as i32],
                    ));
                    entries
                })
                .collect::<Vec<_>>();
            assert_eq!(
                app.world().resource::<Trace>().0,
                expected,
                "owner={owner}, {fps} FPS"
            );
        }
    }
}

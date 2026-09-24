use super::*;
use crate::timing::{GameFrames, TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[test]
fn parallel_events_execute_once_per_game_tick_not_once_per_render() {
    for fps in [15, 30, 60, 144] {
        let mut app = interp_app();
        app.add_plugins((TimingPlugin, LogicalPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 4, vec![cmd(10220, 0, vec![0, 1, 1, 1, 0, 1])])],
        })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / f64::from(fps),
        )));
        for _ in 0..fps {
            app.update();
        }
        assert_eq!(app.world().resource::<GameFrames>().frame, 60);
        assert_eq!(app.world().resource::<Variables>().get(1), 60, "{fps} FPS");
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        for _ in 0..80 {
            app.update();
        }
        assert_eq!(app.world().resource::<Variables>().get(1), 60);
    }
}

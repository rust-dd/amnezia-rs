use super::*;
use crate::timing::{TimingPlugin, logical};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[test]
fn extra_render_frames_do_not_accumulate_camera_shake() {
    fn follow(mut cameras: Query<&mut Transform, With<MainCamera>>) {
        for mut camera in &mut cameras {
            camera.translation = Vec3::new(100.0, 200.0, 0.0);
        }
    }
    for fps in [15, 60, 144] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TimingPlugin, logical::LogicalPlugin))
            .insert_resource(Fx {
                shake_offset: Vec2::new(5.0, -2.0),
                ..default()
            })
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / f64::from(fps),
            )));
        logical::post(&mut app, || (follow, apply_camera_shake).chain());
        let camera = app
            .world_mut()
            .spawn((MainCamera, Transform::default()))
            .id();
        for _ in 0..=fps {
            app.update();
            assert_eq!(
                app.world().get::<Transform>(camera).unwrap().translation,
                Vec3::new(105.0, 198.0, 0.0),
                "{fps} FPS"
            );
        }
    }
}

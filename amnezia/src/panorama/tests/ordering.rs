use super::*;
use crate::player::CameraPan;

fn app() -> App {
    let (mut app, _) = crate::world::test_support::camera_app((20, 15));
    loading::change(&mut app, "Sky", &[1, 1, 0, 0, 0, 0]);
    app.world_mut()
        .resource_scope(|world, mut panorama: Mut<Panorama>| {
            let map = world.resource::<MapData>();
            let display = world
                .resource::<CameraPan>()
                .background_position(map)
                .unwrap();
            let definition = panorama.definition.clone().unwrap();
            panorama.motion.as_mut().unwrap().initialize(
                "Sky",
                UVec2::new(640, 480),
                Some(&definition),
                map,
                display,
            );
        });
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 1, 0]);
    app
}

fn phase(app: &App) -> [i64; 2] {
    app.world()
        .resource::<Panorama>()
        .motion
        .as_ref()
        .unwrap()
        .phase
}

#[test]
fn a_background_event_consumes_earlier_camera_scroll_before_changing_loop_modes() {
    let mut app = app();
    let before = phase(&app);
    app.world_mut()
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(
            1,
            vec![amnezia_data::EventCommand {
                code: 11720,
                indent: 0,
                string: "Sky".into(),
                params: vec![0; 6],
            }],
        );
    app.update();
    assert_eq!(phase(&app), [before[0] + 4, before[1]]);
    app.update();
    loading::change(&mut app, "Sky", &[1, 1, 0, 0, 0, 0]);
    assert_eq!(phase(&app), [2 * (before[0] + 8), before[1]]);
    let restored_loops = phase(&app);
    app.update();
    loading::change(&mut app, "Sky", &[1, 1, 0, 0, 0, 0]);
    assert_eq!(phase(&app), [restored_loops[0] + 4, before[1]]);
    loading::change(&mut app, "Sky", &[1, 1, 0, 0, 0, 0]);
    assert_eq!(phase(&app), [restored_loops[0] + 4, before[1]]);
}

#[test]
fn a_saved_camera_scroll_pending_at_the_command_boundary_applies_exactly_once() {
    let mut app = app();
    let before = phase(&app);
    app.update();
    let saved = ron::to_string(&(
        app.world().resource::<Panorama>().clone(),
        app.world().resource::<CameraPan>().snapshot(),
    ))
    .unwrap();
    for _ in 0..2 {
        let (panorama, camera) =
            ron::from_str::<(Panorama, crate::player::saved_camera::CameraState)>(&saved).unwrap();
        assert!(panorama.valid());
        assert!(camera.valid());
        app.insert_resource(panorama);
        app.insert_resource(camera.into_pan());
        loading::change(&mut app, "Sky", &[0; 6]);
        assert_eq!(phase(&app), [before[0] + 4, before[1]]);
        loading::change(&mut app, "Sky", &[0; 6]);
        assert_eq!(phase(&app), [before[0] + 4, before[1]]);
    }
}

use super::*;

#[test]
fn waiting_pan_yields_for_its_original_duration_even_when_the_camera_cannot_move() {
    let mut app = interp_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(11060, 0, vec![0, 0, 1, 4, 1]),
            cmd(11060, 0, vec![2, 0, 2, 3, 1]),
            switch_cmd(90, 0, 0),
            cmd(11060, 0, vec![1, 0, 1, 4, 1]),
        ],
    );
    app.update();
    assert!(app.world().resource::<CameraPan>().locked);
    for _ in 0..31 {
        app.update();
    }
    assert!(!app.world().resource::<Switches>().get(90));
    for _ in 0..3 {
        app.update();
    }
    assert!(app.world().resource::<Switches>().get(90));
    assert!(!app.world().resource::<CameraPan>().locked);
}

#[test]
fn non_waiting_pan_does_not_hold_the_event_and_return_changes_the_speed() {
    let mut app = interp_app();
    app.world_mut().resource_mut::<CameraPan>().offset = Vec2::new(0.0, 32.0);
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(11060, 0, vec![2, 1, 2, 2, 0]),
            switch_cmd(90, 0, 0),
            cmd(11060, 0, vec![3, 0, 1, 6, 1]),
            switch_cmd(91, 0, 0),
        ],
    );
    app.update();
    assert!(app.world().resource::<Switches>().get(90));
    assert!(!app.world().resource::<Switches>().get(91));
    assert_eq!(app.world().resource::<CameraPan>().speed, 480.0);
}

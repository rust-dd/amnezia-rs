use super::*;

#[test]
fn snapshots_retain_only_captures_and_restart_after_latest_command() {
    let path =
        std::env::temp_dir().join(format!("amnezia-playtest-history-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    storage::snapshot(&path, 4, "(id: 4, other: 1)", true).unwrap();
    storage::snapshot(&path, 9, "(id: 9, other: 2)", false).unwrap();
    assert!(path.join("state-000004.ron").exists());
    assert!(!path.join("state-000009.ron").exists());
    assert_eq!(storage::next_id(&path), 10);
    std::fs::write(path.join("state-000011.ron"), "partial").unwrap();
    assert_eq!(storage::next_id(&path), 12);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn diagnostic_write_failure_preserves_latest_snapshot_without_panicking() {
    let path = std::env::temp_dir().join(format!(
        "amnezia-playtest-write-error-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).unwrap();
    storage::snapshot(&path, 1, "(id: 1)", false).unwrap();
    std::fs::create_dir(path.join("state.tmp")).unwrap();
    let result = storage::snapshot(&path, 2, "(id: 2)", false);
    assert!(result.is_err());
    storage::report(result);
    assert_eq!(
        std::fs::read_to_string(path.join("state.ron")).unwrap(),
        "(id: 1)"
    );
    storage::report(storage::append(&path.join("state.tmp"), "input"));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn catch_up_frames_do_not_skip_input_pulses() {
    assert!(pulse_due(0, 5, 14));
    assert!(!pulse_due(5, 5, 14));
    assert!(pulse_due(10, 5, 14));
    assert!(pulse_due(14, 1, 14));
    assert!(pulse_due(9, 4, 0));
}

#[test]
fn commands_only_allow_bounded_gameplay_inputs() {
    let (id, frames, interval, keys, capture) = parse("3 120 15 enter capture").unwrap();
    assert_eq!((id, frames, interval), (3, 120, 15));
    assert_eq!(keys, [KeyCode::Enter]);
    assert!(capture);
    for command in ["1 0 0", "1 36001 0", "1 20 0 teleport", "bad"] {
        assert!(parse(command).is_err());
    }
}

#[test]
fn physical_input_cannot_change_scripted_holds_or_taps() {
    let mut app = App::new();
    app.insert_resource(Controller {
        directory: PathBuf::new(),
        id: 1,
        remaining: 3,
        elapsed: 0,
        interval: 2,
        keys: vec![KeyCode::Enter],
        input: default(),
        capture: false,
        last_message: String::new(),
        clock: clock::Clock::new(false),
    })
    .init_resource::<ButtonInput<KeyCode>>()
    .add_systems(Update, input);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    app.update();
    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(keys.just_pressed(KeyCode::Enter));
    assert!(!keys.pressed(KeyCode::ArrowDown));
    app.world_mut().resource_mut::<Controller>().elapsed = 1;
    app.update();
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .just_released(KeyCode::Enter)
    );
    app.world_mut().resource_mut::<Controller>().elapsed = 2;
    app.update();
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Enter)
    );
}

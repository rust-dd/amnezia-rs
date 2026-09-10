use super::*;
use crate::battle::model::testkit::build_party2;

#[test]
fn command_windows_move_eight_integer_frames_in_both_directions_then_unlock_input() {
    let mut battle = build_party2();
    let mut windows = CommandWindows::default();
    windows.observe(&battle);
    assert_eq!(windows.x(Panel::Status), Some(76.0));
    assert!(!windows.moving());
    battle.phase = Phase::Command;
    windows.observe(&battle);
    for x in [0, -9, -19, -28, -38, -47, -57, -66, -76] {
        assert_eq!(windows.x, x);
        assert_eq!(windows.x(Panel::Command), Some((320 + x) as f32));
        assert!(windows.moving());
        windows.advance(1);
    }
    assert!(!windows.moving());
    battle.phase = Phase::PartyCommand;
    windows.observe(&battle);
    for x in [-76, -67, -57, -48, -38, -29, -19, -10, 0] {
        assert_eq!(windows.x, x);
        assert!(windows.moving());
        windows.advance(1);
    }
    assert!(!windows.moving());
}

#[test]
fn returning_from_a_submenu_does_not_add_a_stationary_input_delay() {
    let mut battle = build_party2();
    let mut windows = CommandWindows::default();
    windows.observe(&battle);
    battle.phase = Phase::Command;
    windows.observe(&battle);
    windows.advance(9);
    for menu in [
        MenuLevel::Skill,
        MenuLevel::Command,
        MenuLevel::Item,
        MenuLevel::Command,
    ] {
        battle.menu = menu;
        windows.observe(&battle);
        assert_eq!(windows.x, -76);
        assert!(!windows.moving());
    }
    battle.phase = Phase::Resolve;
    windows.observe(&battle);
    battle.new_round();
    windows.observe(&battle);
    assert_eq!(windows.x, 0);
    assert!(!windows.moving());
}

#[test]
fn movement_reaches_the_same_logical_positions_at_different_fps() {
    for fps in [15, 30, 60, 144] {
        let mut frames = GameFrames::default();
        let mut windows = CommandWindows::default();
        windows.move_to(-76);
        for _ in 0..fps {
            let before = frames.frame;
            frames.advance(1.0 / fps as f64);
            windows.advance(frames.frame - before);
            assert_eq!(windows.x, -76 * frames.frame.min(8) as i32 / 8);
            assert_eq!(windows.moving(), frames.frame <= 8);
        }
    }
}

#[test]
fn actual_command_input_is_ignored_during_the_slide_and_resumes_afterward() {
    use crate::gamedata::GameData;
    use crate::state::Inventory;
    let mut app = App::new();
    app.insert_resource(build_party2())
        .init_resource::<GameFrames>()
        .init_resource::<Inventory>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(GameData {
            actors: vec![],
            items: vec![],
            skills: vec![],
        });
    register(&mut app);
    app.add_systems(
        Update,
        (
            crate::battle::input::command_input.run_if(ready),
            observe.after(crate::battle::input::command_input),
        ),
    );
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert!(app.world().resource::<Battle>().phase == Phase::Command);
    for frame in 1..=8 {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        assert!(app.world().resource::<Battle>().menu == MenuLevel::Command);
    }
    app.world_mut().resource_mut::<GameFrames>().frame = 9;
    app.update();
    assert!(app.world().resource::<Battle>().menu == MenuLevel::Target);
}

#[test]
fn screen_transitions_pause_movement_without_advancing_the_hidden_frames() {
    let mut app = App::new();
    app.init_resource::<GameFrames>()
        .insert_resource(build_party2());
    register(&mut app);
    app.update();
    app.world_mut().resource_mut::<Battle>().phase = Phase::Command;
    app.world_mut().resource_mut::<GameFrames>().frame = 1;
    app.update();
    app.world_mut().resource_mut::<GameFrames>().frame = 2;
    app.update();
    assert_eq!(app.world().resource::<CommandWindows>().x, -9);
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 2, IVec2::ZERO);
    app.insert_resource(transition);
    app.world_mut().resource_mut::<GameFrames>().frame = 10;
    app.update();
    assert_eq!(app.world().resource::<CommandWindows>().x, -9);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.world_mut().resource_mut::<GameFrames>().frame = 11;
    app.update();
    assert_eq!(app.world().resource::<CommandWindows>().x, -19);
}

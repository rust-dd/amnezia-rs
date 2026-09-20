use super::*;
use crate::timing::{GameFrames, SceneFrames, TimingPlugin};
use crate::transitions::Transition;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

fn app() -> App {
    let mut app = flow_app();
    app.add_plugins((TimingPlugin, crate::session::SessionPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<crate::state::Inventory>();
    frame(&mut app, 0);
    frame(&mut app, 35);
    app.world_mut().resource_mut::<TitleState>().cursor = NEW_GAME;
    app.world_mut()
        .resource_mut::<crate::state::Inventory>()
        .add_gold(123);
    let mut frames = app.world_mut().resource_mut::<GameFrames>();
    frames.frame = 6000;
    frames.advance(0.5 / 60.0);
    app
}

fn confirm(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}

#[test]
fn new_game_resets_the_clock_on_the_decision_frame_before_its_six_frame_exit() {
    for key in [KeyCode::Enter, KeyCode::Space] {
        let mut app = app();
        app.world_mut().resource_mut::<SceneFrames>().frame = 1234;
        confirm(&mut app, key);
        assert_eq!(*app.world().resource::<GameFrames>(), GameFrames::default());
        assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
        assert_eq!(
            app.world().resource::<TitleState>().stage,
            Stage::Leaving(TitleAction::NewGame)
        );
        assert!(!app.world().resource::<NewGameRequest>().requested);
        assert_eq!(
            app.world().resource::<crate::state::Inventory>().gold(),
            123
        );
        frame(&mut app, 5);
        assert!(app.world().resource::<Transition>().busy());
        assert!(!app.world().resource::<NewGameRequest>().requested);
        frame(&mut app, 6);
        assert!(!app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<NewGameRequest>().requested);
        app.update();
        assert_eq!(app.world().resource::<GameFrames>().frame, 6);
        assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
        assert_eq!(app.world().resource::<crate::state::Inventory>().gold(), 0);
        assert_eq!(app.world().resource::<PendingTeleport>().0, Some((5, 0, 0)));
    }
}

#[test]
fn session_rebuild_preserves_all_frames_and_fractions_counted_since_new_game_selection() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = app();
        app.world_mut().resource_mut::<GameFrames>().frame = u32::MAX - 2;
        confirm(&mut app, KeyCode::Enter);
        let step = Duration::from_secs_f64(1.0 / fps as f64);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
        let mut expected = GameFrames::default();
        for render in 1..=fps {
            expected.advance(step.as_secs_f64());
            app.update();
            assert_eq!(
                *app.world().resource::<GameFrames>(),
                expected,
                "{fps} FPS, render {render}"
            );
            assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
        }
        assert_eq!(app.world().resource::<crate::state::Inventory>().gold(), 0);
        assert!(!app.world().resource::<NewGameRequest>().requested);
        assert_eq!(app.world().resource::<PendingTeleport>().0, Some((5, 0, 0)));
    }
}

#[test]
fn render_updates_without_clock_ticks_cannot_skip_the_new_game_exit() {
    let mut app = app();
    confirm(&mut app, KeyCode::Enter);
    for _ in 0..40 {
        app.update();
        assert_eq!(*app.world().resource::<GameFrames>(), GameFrames::default());
        assert_eq!(app.world().resource::<Transition>().age(), 0);
        assert!(app.world().resource::<Transition>().busy());
        assert!(!app.world().resource::<NewGameRequest>().requested);
        assert!(app.world().resource::<PendingTeleport>().0.is_none());
    }
}

#[test]
fn other_title_commands_leave_the_existing_clock_and_fraction_unchanged() {
    for cursor in [CONTINUE, SHUTDOWN] {
        let mut app = app();
        let absent = std::env::temp_dir().join(format!(
            "amnezia_new_game_clock_absent_{}_{cursor}/slot1.ron",
            std::process::id()
        ));
        assert!(!absent.parent().unwrap().exists());
        app.insert_resource(crate::save::SaveLocation(absent));
        app.world_mut().resource_mut::<TitleState>().cursor = cursor;
        let before = *app.world().resource::<GameFrames>();
        confirm(&mut app, KeyCode::Enter);
        assert_eq!(*app.world().resource::<GameFrames>(), before);
        assert!(!app.world().resource::<NewGameRequest>().requested);
    }
}

#[test]
fn discarding_a_prepared_session_does_not_suppress_the_next_direct_new_game_reset() {
    let mut app = app();
    confirm(&mut app, KeyCode::Enter);
    crate::session::clear_transient(app.world_mut());
    app.world_mut().resource_mut::<TitleActive>().0 = false;
    app.world_mut().resource_mut::<GameFrames>().advance(10.25);
    app.world_mut().resource_mut::<SceneFrames>().frame = 615;
    app.world_mut().resource_mut::<NewGameRequest>().requested = true;
    app.update();
    assert_eq!(*app.world().resource::<GameFrames>(), GameFrames::default());
    assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    assert_eq!(app.world().resource::<PendingTeleport>().0, Some((5, 0, 0)));
}

#[test]
fn title_delay_and_initial_fade_do_not_count_as_scene_frames() {
    let mut app = app();
    let now = app.world().resource::<GameFrames>().frame;
    app.world_mut().resource_mut::<TitleState>().stage = Stage::Wait(now + 20);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    for _ in 0..55 {
        app.update();
        assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    }
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Ready);
    app.update();
    assert_eq!(app.world().resource::<SceneFrames>().frame, 1);
}

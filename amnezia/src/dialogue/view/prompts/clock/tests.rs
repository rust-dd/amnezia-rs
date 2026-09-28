use super::*;
use crate::timing::GameFrames;

#[test]
fn a_callback_advances_the_owed_cursor_tick_once_before_the_fresh_tick() {
    let mut clock = Clock::default();
    clock.advance(0, false, false, true, false);
    clock.advance(10, false, false, true, false);
    assert_eq!(clock.message[0], 10);
    clock.advance(10, true, false, true, false);
    assert_eq!(clock.message[0], 11);
    assert_eq!(clock.number[0], 11);
    clock.advance(10, true, false, true, false);
    assert_eq!(clock.message[0], 11);
    clock.advance(11, false, false, true, false);
    assert_eq!(clock.message[0], 12);
    assert_eq!(clock.number[0], 12);
}

#[test]
fn hidden_messages_keep_their_phase_but_inactive_number_windows_do_not() {
    let mut clock = Clock::default();
    clock.advance(0, false, false, false, false);
    clock.advance(11, false, false, false, false);
    assert_eq!(clock.source_x(0, false), 96.0);
    assert_eq!(clock.source_x(0, true), 64.0);
    clock.advance(21, false, false, true, false);
    assert_eq!(clock.source_x(0, false), 64.0);
    assert_eq!(clock.source_x(0, true), 64.0);
    clock.advance(22, false, false, true, false);
    assert_eq!(clock.source_x(0, true), 96.0);
}

#[test]
fn paused_window_ticks_are_not_replayed_when_the_scene_returns() {
    let mut clock = Clock::default();
    clock.advance(0, false, false, true, false);
    clock.advance(10, false, false, true, false);
    clock.advance(1000, false, false, true, true);
    assert_eq!(clock.source_x(0, false), 64.0);
    assert_eq!(clock.source_x(0, true), 64.0);
    clock.advance(1001, false, false, true, false);
    assert_eq!(clock.source_x(0, false), 96.0);
    assert_eq!(clock.source_x(0, true), 96.0);
}

#[test]
fn battle_windows_start_fresh_without_erasing_the_suspended_map_phase() {
    let mut clock = Clock::default();
    clock.advance(0, false, false, true, false);
    clock.advance(11, false, false, true, false);
    clock.advance(11, false, true, false, false);
    assert_eq!(clock.source_x(0, true), 96.0);
    assert_eq!(clock.source_x(1, false), 64.0);
    clock.advance(22, false, true, true, false);
    assert_eq!(clock.source_x(1, true), 96.0);
    clock.advance(22, false, false, false, false);
    assert_eq!(clock.source_x(0, false), 96.0);
    clock.advance(22, false, true, true, false);
    assert_eq!(clock.source_x(1, true), 64.0);
}

#[test]
fn cursor_phases_follow_logical_time_at_all_render_rates_and_wrap_safely() {
    for fps in [15, 30, 60, 120, 144] {
        let mut clock = Clock::default();
        let mut frames = GameFrames::default();
        frames.frame = u32::MAX - 30;
        clock.advance(frames.frame, false, false, true, false);
        let start = frames.frame;
        for _ in 0..fps * 2 {
            frames.advance(1.0 / fps as f64);
            clock.advance(frames.frame, false, false, true, false);
            let phase = frames.frame.wrapping_sub(start) % 21;
            let expected = if phase <= 10 { 64.0 } else { 96.0 };
            assert_eq!(clock.source_x(0, false), expected);
            assert_eq!(clock.source_x(0, true), expected);
        }
    }
}

#[test]
fn the_terminal_transition_frame_does_not_tick_a_message_cursor() {
    use crate::transitions::{Kind, Transition, TransitionPlugin};
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        crate::timing::TimingPlugin,
        TransitionPlugin,
    ))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    register(&mut app);
    app.update();
    for _ in 0..10 {
        app.update();
    }
    let raw = app.world().resource::<GameFrames>().frame;
    app.world_mut().resource_mut::<Transition>().start_for(
        Kind::Fade,
        false,
        raw,
        IVec2::new(160, 120),
        6,
    );
    for _ in 0..7 {
        app.update();
        assert_eq!(app.world().resource::<Clock>().source_x(0, false), 64.0);
    }
    assert!(!app.world().resource::<Transition>().busy());
    app.update();
    assert_eq!(app.world().resource::<Clock>().source_x(0, false), 96.0);
}

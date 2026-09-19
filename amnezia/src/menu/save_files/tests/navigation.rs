use super::super::navigation::Navigation;
use bevy::prelude::*;

#[test]
fn scrolling_uses_original_integer_interpolation_and_blocks_through_the_last_tick() {
    for (index, key, top, distance) in [
        (2, KeyCode::ArrowDown, 1, 64),
        (0, KeyCode::ArrowUp, 12, 768),
        (14, KeyCode::ArrowDown, 0, -768),
    ] {
        let mut nav = Navigation::new(index);
        let mut keys = ButtonInput::default();
        keys.press(key);
        assert!(nav.tick(&keys, true, true));
        assert_eq!(nav.top, top);
        keys.reset_all();
        for frame in 1..=7 {
            assert!(nav.moving());
            assert_eq!(
                nav.offset(),
                distance - distance * frame / 7,
                "frame {frame}"
            );
            assert!(!nav.tick(&keys, false, true));
        }
        assert!(!nav.moving());
        assert_eq!(nav.offset(), 0);
    }
}

#[test]
fn held_arrows_repeat_at_tick_twenty_four_then_four_and_do_not_wrap() {
    let mut nav = Navigation::new(0);
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::ArrowDown);
    for frame in 1..=24 {
        nav.tick(&keys, frame == 1, true);
        if frame < 24 {
            assert_eq!(nav.index, 1, "frame {frame}");
        }
    }
    assert_eq!(nav.index, 2);
    for _ in 0..300 {
        nav.tick(&keys, false, true);
    }
    assert_eq!(nav.index, 14);
    assert!(!nav.moving());
    nav.tick(&keys, true, true);
    assert_eq!(nav.index, 0);
}

#[test]
fn page_keys_move_three_slots_and_initial_view_contains_the_latest_slot() {
    for index in 0..15 {
        let nav = Navigation::new(index);
        assert_eq!(nav.top, index.saturating_sub(2));
    }
    let mut nav = Navigation::new(0);
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::PageDown);
    nav.tick(&keys, true, true);
    assert_eq!((nav.index, nav.top), (3, 1));
    for _ in 0..8 {
        nav.tick(&ButtonInput::default(), false, true);
    }
    keys.reset_all();
    keys.press(KeyCode::PageUp);
    nav.tick(&keys, true, true);
    assert_eq!((nav.index, nav.top), (0, 0));
}

#[test]
fn clocks_count_logical_frames_without_advancing_on_extra_render_frames() {
    for fps in [30, 60, 120, 144] {
        let mut clock = crate::timing::GameFrames::default();
        let mut nav = Navigation::new(0);
        for _ in 0..fps {
            let before = clock.frame;
            clock.advance(1.0 / fps as f64);
            let elapsed = clock.frame - before;
            for _ in 0..elapsed.max(1) {
                nav.tick(&ButtonInput::default(), false, elapsed > 0);
            }
        }
        assert_eq!((nav.arrow, nav.cursors[0]), (20, 19), "{fps} FPS");
    }
}

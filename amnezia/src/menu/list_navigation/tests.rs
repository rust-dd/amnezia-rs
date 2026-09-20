use super::{Input, Navigation as WindowNavigation, input::KEYS};
use bevy::prelude::*;

type Navigation = WindowNavigation<12>;

fn press<const ROWS: usize>(nav: &mut WindowNavigation<ROWS>, key: KeyCode) -> u32 {
    nav.tick(std::array::from_fn(|i| KEYS[i] == key), true)
}

#[test]
fn two_columns_keep_odd_last_rows_and_empty_lists_in_bounds() {
    for count in 0..=33 {
        for index in 0..count.max(1) {
            let mut nav = Navigation::new(index, count);
            press(&mut nav, KeyCode::ArrowDown);
            assert_eq!(nav.index, if index + 2 < count { index + 2 } else { index });
            let mut nav = Navigation::new(index, count);
            press(&mut nav, KeyCode::ArrowUp);
            assert_eq!(nav.index, index.saturating_sub(2).max(index % 2));
            let mut nav = Navigation::new(index, count);
            press(&mut nav, KeyCode::ArrowRight);
            assert_eq!(nav.index, (index + 1).min(count.saturating_sub(1)));
        }
    }
}

#[test]
fn one_row_scroll_takes_four_ticks_and_defers_cursor_and_help_updates() {
    let mut nav = Navigation::new(22, 30);
    assert_eq!((nav.offset, nav.cursor_y), (0, 176));
    assert_eq!(press(&mut nav, KeyCode::ArrowDown), 1);
    assert_eq!((nav.index, nav.offset, nav.help_index), (24, 0, 22));
    for step in 1..=4 {
        nav.tick([false; 4], true);
        assert_eq!(nav.offset, step * 4);
        assert_eq!(nav.cursor_y, 176);
        assert_eq!(nav.cursor_index, if step < 4 { 22 } else { 24 });
        assert_eq!(nav.help_index, nav.cursor_index);
    }
    assert_eq!(nav.arrow_frame, 5);
    assert_eq!(nav.arrows, [true, true]);
    assert!(nav.movement.is_none());
}

#[test]
fn upward_scroll_and_navigation_on_the_completion_tick_match_the_original_order() {
    let mut nav = Navigation::new(24, 30);
    nav.refresh(2, 30);
    press(&mut nav, KeyCode::ArrowUp);
    for step in 1..4 {
        press(&mut nav, KeyCode::ArrowRight);
        assert_eq!((nav.index, nav.offset), (0, 16 - step * 4));
        assert_eq!(nav.help_index, 2);
    }
    press(&mut nav, KeyCode::ArrowRight);
    assert_eq!((nav.index, nav.offset, nav.cursor_y), (1, 0, 0));
    assert_eq!(nav.help_index, 1);
}

#[test]
fn held_arrows_repeat_at_twenty_four_then_every_four_logical_frames_at_all_rates() {
    for fps in [15, 30, 60, 120, 144] {
        let mut nav = Navigation::new(0, 100);
        let mut frames = crate::timing::GameFrames::default();
        let mut keys = ButtonInput::default();
        let mut input = Input::default();
        input.advance(0, &keys);
        keys.press(KeyCode::ArrowDown);
        for _ in 0..fps {
            frames.advance(1.0 / fps as f64);
            input.advance(frames.frame, &keys);
            for repeated in input.steps() {
                nav.tick(repeated, input.timed());
            }
            keys.clear();
        }
        assert_eq!(nav.index, 22, "{fps} FPS");
        assert_eq!(nav.cursor_frame, 18);
    }
}

#[test]
fn empty_lists_keep_a_blank_cursor_and_do_not_scroll_or_accept_navigation() {
    let mut nav = Navigation::new(100, 0);
    for key in KEYS {
        assert_eq!(press(&mut nav, key), 0);
        assert_eq!(nav.index, 0);
        assert_eq!(nav.offset, 0);
        assert_eq!(nav.arrows, [false; 2]);
    }
    nav.refresh(24, 25);
    nav.refresh(nav.index, 24);
    assert_eq!(nav.index, 23);
    assert_eq!(nav.help_index, 23);
}

#[test]
fn render_only_updates_do_not_advance_a_scroll_or_blink_cycle() {
    let mut nav = Navigation::new(22, 30);
    press(&mut nav, KeyCode::ArrowDown);
    for _ in 0..40 {
        nav.tick([false; 4], false);
        assert_eq!(nav.offset, 0);
        assert_eq!(nav.cursor_frame, 1);
        assert_eq!(nav.arrow_frame, 0);
        assert_eq!(nav.help_index, 22);
    }
}

#[test]
fn ten_row_skill_list_scrolls_four_pixels_per_tick_and_preserves_its_help_until_done() {
    let mut nav = WindowNavigation::<10>::new(18, 25);
    assert_eq!(press(&mut nav, KeyCode::ArrowDown), 1);
    assert_eq!(
        (nav.index, nav.offset, nav.help_index, nav.cursor_y),
        (20, 0, 18, 144)
    );
    for step in 1..=4 {
        nav.tick([false; 4], true);
        assert_eq!(nav.offset, step * 4);
        assert_eq!(nav.help_index, if step < 4 { 18 } else { 20 });
    }
    assert_eq!((nav.cursor_index, nav.cursor_y), (20, 144));
    assert_eq!(nav.arrows, [true, true]);
}

#[test]
fn skill_cursor_and_arrows_follow_logical_frames_at_all_render_rates() {
    for fps in [15, 30, 60, 120, 144] {
        let mut nav = WindowNavigation::<10>::new(0, 100);
        let mut frames = crate::timing::GameFrames::default();
        let mut keys = ButtonInput::default();
        let mut input = Input::default();
        input.advance(0, &keys);
        keys.press(KeyCode::ArrowDown);
        for _ in 0..fps {
            frames.advance(1.0 / fps as f64);
            input.advance(frames.frame, &keys);
            for repeated in input.steps() {
                nav.tick(repeated, input.timed());
            }
            keys.clear();
        }
        assert_eq!(
            (nav.index, nav.cursor_frame, nav.offset),
            (22, 18, 16),
            "{fps} FPS"
        );
        for _ in 0..4 {
            nav.tick([false; 4], true);
        }
        assert_eq!((nav.cursor_index, nav.offset, nav.help_index), (22, 32, 22));
    }
}

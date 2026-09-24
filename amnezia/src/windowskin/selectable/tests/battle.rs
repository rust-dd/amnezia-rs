use super::*;

#[test]
fn battle_pages_four_rows_and_wraps_held_single_column_arrows() {
    let mut list = List::new(1, 4, true);
    list.refresh(0, 13);
    for (action, index, offset) in [
        (4, 4, 16),
        (4, 8, 80),
        (4, 12, 144),
        (5, 8, 128),
        (5, 4, 64),
        (5, 0, 0),
    ] {
        assert_eq!(input(&mut list, action, false), 1);
        assert_eq!((list.index, list.offset), (index, offset));
    }
    assert_eq!(input(&mut list, 1, false), 1);
    assert_eq!((list.index, list.offset), (12, 144));
    assert_eq!(input(&mut list, 0, false), 1);
    assert_eq!((list.index, list.offset), (0, 0));
}

#[test]
fn battle_cursor_and_arrow_clocks_keep_original_cycles_at_all_render_rates() {
    for fps in [15, 30, 60, 144] {
        let mut frames = crate::timing::GameFrames::default();
        let mut list = List::new(2, 4, true);
        list.refresh(0, 12);
        for _ in 0..fps * 3 {
            let before = frames.frame;
            frames.advance(1.0 / fps as f64);
            for _ in before..frames.frame {
                list.tick([false; 6], [false; 2], true, true);
            }
            assert_eq!(list.cursor_frame, frames.frame % 21);
            assert_eq!(list.arrow_frame, frames.frame % 40);
            assert_eq!(
                list.cursor_x(),
                if frames.frame % 21 <= 10 { 64.0 } else { 96.0 }
            );
        }
    }
}

#[test]
fn battle_viewports_stay_on_whole_rows_and_only_scroll_when_selection_leaves() {
    let mut list = List::new(2, 4, true);
    for index in 0..40 {
        list.refresh(index, 40);
        let first = list.offset as usize / 16 * 2;
        assert!(first.is_multiple_of(2));
        assert!((first..first + 8).contains(&index));
    }
    list.refresh(11, 40);
    list.offset = 32;
    for index in [9, 7, 5] {
        list.refresh(index, 40);
        assert_eq!(list.offset, 32);
    }
    list.refresh(3, 40);
    assert_eq!(list.offset, 16);
}

#[test]
fn scroll_arrows_update_twice_on_the_fourth_motion_frame_and_blink_while_inactive() {
    let mut list = List::new(2, 4, true);
    list.refresh(6, 12);
    list.tick([false; 6], [false; 2], true, true);
    assert_eq!(list.arrows, [false, true]);
    input(&mut list, 0, true);
    assert_eq!(list.arrow_frame, 1);
    for age in 1..=4 {
        list.tick([false; 6], [false; 2], true, true);
        assert_eq!(list.arrow_frame, 1 + age + u32::from(age == 4));
    }
    assert_eq!(list.arrows, [true, true]);
    let cursor = list.cursor_frame;
    for _ in 0..14 {
        list.tick([true; 6], [true; 2], false, true);
    }
    assert_eq!(list.cursor_frame, cursor);
    assert_eq!(list.arrows, [false; 2]);
    for _ in 0..20 {
        list.tick([false; 6], [false; 2], false, true);
    }
    assert_eq!(list.arrows, [true, true]);
}

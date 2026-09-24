use super::List;

mod battle;

fn input(list: &mut List, action: usize, fresh: bool) -> u32 {
    let mut repeated = [false; 6];
    repeated[action] = true;
    list.tick(
        repeated,
        [fresh && action == 0, fresh && action == 1],
        true,
        true,
    )
}

#[test]
fn single_column_wraps_only_fresh_vertical_presses() {
    let mut list = List::new(1, 7, false);
    list.refresh(0, 3);
    assert_eq!(input(&mut list, 1, false), 0);
    assert_eq!(input(&mut list, 1, true), 1);
    assert_eq!(list.index, 2);
    assert_eq!(input(&mut list, 0, false), 0);
    assert_eq!(input(&mut list, 0, true), 1);
    assert_eq!(list.index, 0);
    list.refresh(0, 1);
    assert_eq!(input(&mut list, 0, true), 1);
    assert_eq!(input(&mut list, 0, false), 0);
    list.refresh(0, 0);
    assert_eq!(input(&mut list, 0, true), 0);
    assert_eq!(input(&mut list, 1, true), 0);
}

#[test]
fn selling_moves_two_rows_vertically_and_one_item_horizontally_without_wrap() {
    let mut list = List::new(2, 7, false);
    list.refresh(0, 5);
    assert_eq!(input(&mut list, 1, true), 0);
    assert_eq!(input(&mut list, 3, true), 0);
    assert_eq!(input(&mut list, 2, true), 1);
    assert_eq!((list.index, list.cursor_index, list.cursor_y), (1, 1, 0));
    assert_eq!(input(&mut list, 0, true), 1);
    assert_eq!(list.index, 3);
    assert_eq!(input(&mut list, 0, true), 0);
    assert_eq!(input(&mut list, 2, true), 1);
    assert_eq!(list.index, 4);
    assert_eq!(input(&mut list, 2, true), 0);
    assert_eq!(input(&mut list, 4, true), 0);
    assert_eq!(input(&mut list, 5, true), 0);
    assert_eq!(input(&mut list, 1, true), 1);
    assert_eq!(list.index, 2);
}

#[test]
fn buying_pages_seven_items_and_keeps_selection_visible() {
    let mut list = List::new(1, 7, false);
    list.refresh(0, 19);
    for expected in [(7, 16), (14, 128), (18, 192)] {
        assert_eq!(input(&mut list, 4, true), 1);
        assert_eq!((list.index, list.offset), expected);
        assert!(list.movement.is_none());
    }
    assert_eq!(input(&mut list, 4, true), 0);
    for expected in [11, 4, 0] {
        assert_eq!(input(&mut list, 5, true), 1);
        assert_eq!(list.index, expected);
    }
    assert_eq!(input(&mut list, 5, true), 0);
    assert_eq!(list.offset, 0);
}

#[test]
fn smooth_scroll_keeps_cursor_and_help_until_the_fourth_tick() {
    let mut list = List::new(1, 7, false);
    list.refresh(6, 20);
    list.tick([false; 6], [false; 2], true, true);
    assert_eq!(input(&mut list, 0, true), 1);
    assert_eq!(
        (list.index, list.cursor_index, list.help_index, list.offset),
        (7, 6, 6, 0)
    );
    assert_eq!(list.cursor_y, 96);
    for offset in [4, 8, 12] {
        assert_eq!(list.tick([true; 6], [true; 2], true, true), 0);
        assert_eq!(
            (list.cursor_index, list.help_index, list.offset),
            (6, 6, offset)
        );
    }
    list.tick([false; 6], [false; 2], true, false);
    assert_eq!(list.offset, 12);
    list.tick([false; 6], [false; 2], true, true);
    assert_eq!(
        (list.cursor_index, list.help_index, list.offset),
        (7, 7, 16)
    );
    assert_eq!(list.cursor_y, 96);
    list.refresh(1, 20);
    assert_eq!(input(&mut list, 1, true), 1);
    for offset in [12, 8, 4, 0] {
        list.tick([false; 6], [false; 2], true, true);
        assert_eq!(list.offset, offset);
    }
    assert_eq!(
        (
            list.index,
            list.cursor_index,
            list.help_index,
            list.cursor_y
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn cursor_only_ticks_when_active_but_hidden_list_arrows_keep_blinking() {
    let mut list = List::new(1, 7, false);
    list.refresh(0, 10);
    for frame in 1..=42 {
        list.tick([false; 6], [false; 2], true, true);
        assert_eq!(list.cursor_frame, frame % 21);
        assert_eq!(list.arrows, [false, frame % 40 < 20]);
    }
    for _ in 0..18 {
        list.tick([false; 6], [false; 2], false, true);
    }
    assert_eq!(
        (list.cursor_frame, list.arrow_frame, list.arrows),
        (0, 20, [false; 2])
    );
    list.tick([true; 6], [true; 2], false, false);
    assert_eq!(
        (list.index, list.cursor_frame, list.arrow_frame),
        (0, 0, 20)
    );
}

#[test]
fn simultaneous_inputs_follow_original_down_up_page_right_left_order() {
    let mut list = List::new(1, 7, false);
    list.refresh(0, 8);
    assert_eq!(list.tick([true; 6], [true; 2], true, true), 4);
    assert_eq!(list.index, 0);
    let mut list = List::new(2, 7, false);
    list.refresh(0, 4);
    assert_eq!(list.tick([true; 6], [true; 2], true, true), 4);
    assert_eq!(list.index, 0);
}

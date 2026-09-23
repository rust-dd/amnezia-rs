use super::*;

#[test]
fn original_message_and_gold_heights_follow_seven_open_and_close_updates() {
    for (height, opening, closing) in [
        (80, [5, 11, 17, 22, 28, 34, 40], [34, 28, 22, 17, 11, 5, 0]),
        (32, [2, 4, 6, 9, 11, 13, 16], [13, 11, 9, 6, 4, 2, 0]),
    ] {
        let mut motion = Motion::default();
        motion.open(true);
        assert!(motion.visible() && !motion.ready());
        assert_eq!(motion.half_height(height), 0);
        for expected in opening {
            assert!(!motion.step());
            assert_eq!(motion.half_height(height), expected);
        }
        assert!(motion.ready());
        motion.close(true);
        assert_eq!(motion.half_height(height), closing[0]);
        for (index, expected) in closing.into_iter().enumerate().skip(1) {
            assert_eq!(motion.step(), index == 6);
            assert_eq!(motion.half_height(height), expected);
        }
        assert!(!motion.visible());
    }
}

#[test]
fn reopening_a_visible_window_cancels_closing_without_another_opening() {
    let mut motion = Motion::default();
    motion.open(false);
    assert!(motion.ready());
    motion.close(true);
    assert!(motion.closing());
    motion.open(true);
    assert!(motion.ready());
    motion.close(false);
    assert!(!motion.visible());
}

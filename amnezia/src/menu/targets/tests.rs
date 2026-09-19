use super::*;

#[test]
fn target_repetition_has_identical_positions_and_sound_counts_at_every_render_rate() {
    for fps in [15, 30, 60, 120, 144] {
        let mut navigation = Navigation::default();
        let mut frames = crate::timing::GameFrames::default();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::ArrowDown);
        navigation.elapsed(0);
        let mut cursor = 0;
        let mut sounds = 0;
        for render in 0..fps {
            frames.advance(1.0 / fps as f64);
            let elapsed = navigation.elapsed(frames.frame);
            for tick in 0..elapsed.max(1) {
                let repeated =
                    navigation.sample(&keys, u32::from(elapsed > 0), render == 0 && tick == 0);
                sounds += navigate(&mut cursor, 4, repeated);
            }
            keys.clear();
        }
        assert_eq!((cursor, sounds), (3, 11), "{fps} FPS");
    }
}

#[test]
fn single_actor_arrows_still_sound_empty_rosters_and_side_arrows_do_not() {
    let mut cursor = 0;
    assert_eq!(navigate(&mut cursor, 1, [true, true, true, true]), 2);
    assert_eq!(cursor, 0);
    assert_eq!(navigate(&mut cursor, 0, [true; 4]), 0);
    let mut navigation = Navigation::default();
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::ArrowLeft);
    keys.press(KeyCode::ArrowRight);
    assert_eq!(navigation.sample(&keys, 60, true), [false; 4]);
}

#[test]
fn simultaneous_directions_apply_down_up_page_down_page_up_in_original_order() {
    let mut cursor = 3;
    assert_eq!(navigate(&mut cursor, 4, [true; 4]), 3);
    assert_eq!(cursor, 0);
    let mut cursor = 1;
    assert_eq!(navigate(&mut cursor, 4, [true, true, false, false]), 2);
    assert_eq!(cursor, 1);
}

#[test]
fn sampling_without_navigation_keeps_held_phase_but_does_not_replay_past_repeats() {
    let mut navigation = Navigation::default();
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::ArrowDown);
    assert!(navigation.sample(&keys, 1, true)[0]);
    keys.clear();
    navigation.sample(&keys, 26, false);
    assert!(navigation.sample(&keys, 1, false)[0]);
    assert!(!navigation.sample(&keys, 0, false)[0]);
    keys.release(KeyCode::ArrowDown);
    navigation.sample(&keys, 0, true);
    keys.press(KeyCode::ArrowDown);
    assert!(navigation.sample(&keys, 1, true)[0]);
    keys.clear();
    assert!(!navigation.sample(&keys, 22, false)[0]);
    assert!(navigation.sample(&keys, 1, false)[0]);
}

#[test]
fn frame_rollover_is_elapsed_time_but_a_rewound_session_clock_resets_the_sample() {
    let mut navigation = Navigation::default();
    assert_eq!(navigation.elapsed(u32::MAX - 1), 0);
    assert_eq!(navigation.elapsed(0), 2);
    assert_eq!(navigation.elapsed(0), 0);
    navigation.elapsed(1000);
    navigation.held = [50; 4];
    assert_eq!(navigation.elapsed(5), 0);
    assert_eq!(navigation.held, [0; 4]);
}

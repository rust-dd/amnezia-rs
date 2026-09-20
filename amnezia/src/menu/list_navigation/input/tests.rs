use super::*;

fn down_steps(input: &Input) -> Vec<u32> {
    input
        .steps()
        .enumerate()
        .filter_map(|(step, repeated)| repeated[0].then_some(step as u32 + 1))
        .collect()
}

#[test]
fn batched_ticks_repeat_on_the_same_frames_as_individual_updates() {
    let mut input = Input::default();
    let mut keys = ButtonInput::default();
    input.advance(0, &keys);
    keys.press(KeyCode::ArrowDown);
    input.advance(32, &keys);
    assert_eq!(down_steps(&input), [1, 24, 28, 32]);
    keys.clear();
    input.advance(40, &keys);
    assert_eq!(down_steps(&input), [4, 8]);
}

#[test]
fn ignored_steps_advance_the_hold_without_queueing_moves_for_the_next_window() {
    let mut input = Input::default();
    let mut keys = ButtonInput::default();
    input.advance(0, &keys);
    keys.press(KeyCode::ArrowDown);
    input.advance(1, &keys);
    keys.clear();
    input.advance(30, &keys);
    input.advance(31, &keys);
    assert!(down_steps(&input).is_empty());
    input.advance(32, &keys);
    assert_eq!(down_steps(&input), [1]);
}

#[test]
fn release_and_repress_without_an_intermediate_update_restart_the_delay() {
    let mut input = Input::default();
    let mut keys = ButtonInput::default();
    input.advance(0, &keys);
    keys.press(KeyCode::ArrowDown);
    input.advance(27, &keys);
    keys.clear();
    keys.release(KeyCode::ArrowDown);
    keys.press(KeyCode::ArrowDown);
    input.advance(28, &keys);
    assert_eq!(down_steps(&input), [1]);
    keys.clear();
    input.advance(51, &keys);
    assert_eq!(down_steps(&input), [23]);
}

#[test]
fn render_only_updates_accept_fresh_keys_without_replaying_a_timed_repeat() {
    let mut input = Input::default();
    let mut keys = ButtonInput::default();
    input.advance(0, &keys);
    keys.press(KeyCode::ArrowDown);
    input.advance(24, &keys);
    keys.clear();
    input.advance(24, &keys);
    assert!(!input.timed());
    assert_eq!(input.steps().collect::<Vec<_>>(), [[false; 4]]);
    keys.press(KeyCode::ArrowRight);
    input.advance(24, &keys);
    assert_eq!(
        input.steps().collect::<Vec<_>>(),
        [[false, false, true, false]]
    );
    keys.clear();
    input.advance(28, &keys);
    assert_eq!(down_steps(&input), [4]);
}

#[test]
fn resetting_the_frame_clock_discards_the_old_hold_phase() {
    let mut input = Input::default();
    let mut keys = ButtonInput::default();
    input.advance(0, &keys);
    keys.press(KeyCode::ArrowDown);
    input.advance(24, &keys);
    keys.clear();
    input.advance(0, &keys);
    assert!(input.rewound);
    assert!(!input.timed());
    assert!(down_steps(&input).is_empty());
    input.advance(24, &keys);
    assert!(!input.rewound);
    assert_eq!(down_steps(&input), [24]);
}

#[test]
fn a_genuine_u32_clock_rollover_preserves_the_hold_phase() {
    let mut input = Input::default();
    let mut keys = ButtonInput::default();
    input.advance(u32::MAX - 11, &keys);
    keys.press(KeyCode::ArrowDown);
    input.advance(u32::MAX, &keys);
    assert_eq!(down_steps(&input), [1]);
    keys.clear();
    input.advance(16, &keys);
    assert!(!input.rewound);
    assert_eq!(down_steps(&input), [13, 17]);
}

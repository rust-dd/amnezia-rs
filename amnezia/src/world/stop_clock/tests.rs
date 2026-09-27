use super::*;

#[test]
fn step_turn_and_wait_thresholds_match_every_original_frequency() {
    for (frequency, step_count, turn_count) in [
        (1, 256, 128),
        (2, 128, 64),
        (3, 64, 32),
        (4, 32, 16),
        (5, 16, 8),
        (6, 8, 4),
        (7, 4, 2),
        (8, 0, 0),
    ] {
        assert_eq!(step(frequency), step_count);
        assert_eq!(turn(frequency), turn_count);
        assert_eq!(wait(frequency), 20 + turn_count);
    }
}

#[test]
fn movement_resets_and_paused_zero_counts_increment_once_without_overflow() {
    let mut clock = StopClock {
        count: i32::MAX as u32,
        maximum: 32,
    };
    clock.advance(false, false, true);
    assert_eq!(clock.count, i32::MAX as u32);
    clock.advance(true, false, false);
    assert_eq!(clock.count, 0);
    clock.advance(false, false, false);
    assert_eq!(clock.count, 1);
    clock.advance(false, false, false);
    assert_eq!(clock.count, 1);
    clock.advance(false, true, false);
    assert_eq!(clock.count, 2);
}

#[test]
fn legacy_countdowns_preserve_remaining_whole_ticks_instead_of_float_residue() {
    for (seconds, remaining) in [
        (0.0, 0),
        (-1.0, 0),
        (1.0 / 60.0, 1),
        (0.1, 6),
        (0.731, 44),
        (307.2 / 60.0, 308),
    ] {
        let clock = StopClock::from_legacy(seconds, 64);
        assert!(clock.valid());
        assert_eq!(clock.maximum - clock.count, remaining);
    }
    for value in [f32::NAN, f32::INFINITY, -f32::INFINITY, 6.0] {
        assert!(!legacy_valid(value));
    }
}

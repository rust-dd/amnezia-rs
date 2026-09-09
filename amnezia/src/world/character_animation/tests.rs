use super::*;
use crate::player::Player;

fn character() -> Player {
    Player {
        tile_x: 0,
        tile_y: 0,
        dir: 0,
        frame: 1,
        charset: String::new(),
        index: 0,
    }
}

#[test]
fn every_original_speed_uses_its_stationary_continuous_and_spin_thresholds() {
    for (speed, walk, continuous, spin) in [
        (1, 12, 16, 24),
        (2, 10, 12, 16),
        (3, 8, 10, 12),
        (4, 6, 8, 8),
        (5, 5, 7, 6),
        (6, 4, 6, 4),
    ] {
        for (mode, moving, limit) in [(0, true, walk), (1, false, continuous), (5, false, spin)] {
            let mut animation = CharacterAnimation {
                mode,
                ..Default::default()
            };
            let mut ch = character();
            for _ in 1..limit {
                animation.advance(&mut ch, speed, moving, false, 1.0 / 60.0);
                assert_eq!((ch.frame, ch.dir), (1, 0));
            }
            animation.advance(&mut ch, speed, moving, false, 1.0 / 60.0);
            assert_eq!((ch.frame, ch.dir), if mode == 5 { (1, 1) } else { (2, 0) });
        }
    }
}

#[test]
fn four_animation_states_use_both_middle_poses_and_settle_without_snapping() {
    let mut animation = CharacterAnimation::default();
    let mut ch = character();
    for expected in [2, 3, 0, 1] {
        animation.advance(&mut ch, 4, true, false, 6.0 / 60.0);
        assert_eq!(ch.frame, expected);
    }
    animation.advance(&mut ch, 4, true, false, 6.0 / 60.0);
    assert_eq!(ch.frame, 2);
    animation.advance(&mut ch, 4, false, false, 7.0 / 60.0);
    assert_eq!(ch.frame, 2);
    animation.advance(&mut ch, 4, false, false, 1.0 / 60.0);
    assert_eq!(ch.frame, 3);
    animation.advance(&mut ch, 4, false, false, 1.0);
    assert_eq!(ch.frame, 3);
    assert_eq!(
        crate::tiles::charset_source(0, 2, 1),
        crate::tiles::charset_source(0, 2, 3)
    );
}

#[test]
fn idle_modes_and_fixed_graphics_keep_their_original_distinctions() {
    for mode in 0..=6 {
        let mut animation = CharacterAnimation {
            mode,
            ..Default::default()
        };
        let mut ch = character();
        ch.frame = if matches!(mode, 4..=6) { 2 } else { 1 };
        animation.advance(&mut ch, 4, false, false, 8.0 / 60.0);
        assert_eq!(ch.frame, if matches!(mode, 0 | 2) { 1 } else { 2 });
        assert_eq!(ch.dir, u32::from(mode == 5));
        assert_eq!(animation.keeps_facing(), matches!(mode, 2..=5));
    }
}

#[test]
fn pause_and_jump_reset_walking_but_preserve_fixed_poses_and_do_not_stop_spinners() {
    for (paused, jumping) in [(true, false), (false, true)] {
        for mode in [0, 1, 2, 3, 4, 5] {
            let mut animation = CharacterAnimation {
                mode,
                paused,
                ..Default::default()
            };
            let mut ch = character();
            ch.frame = 2;
            animation.advance(&mut ch, 4, true, jumping, 8.0 / 60.0);
            assert_eq!(ch.frame, if mode >= 4 { 2 } else { 1 });
            assert_eq!(ch.dir, u32::from(mode == 5));
        }
    }
}

#[test]
fn continuous_and_spinning_animation_are_independent_of_render_frame_rate() {
    for mode in [1, 3, 5] {
        let mut states = Vec::new();
        for fps in [15, 30, 60, 120, 144] {
            let mut animation = CharacterAnimation {
                mode,
                ..Default::default()
            };
            let mut ch = character();
            for _ in 0..fps * 3 {
                animation.advance(&mut ch, 3, false, false, 1.0 / fps as f32);
            }
            states.push((ch.frame, ch.dir, animation.count));
        }
        assert!(
            states.windows(2).all(|pair| pair[0] == pair[1]),
            "{states:?}"
        );
    }
}

#[test]
fn vehicle_animation_uses_its_own_thresholds_and_stops_for_landed_airships() {
    for moving in [false, true] {
        let mut animation = CharacterAnimation {
            paused: true,
            ..Default::default()
        };
        let mut ch = character();
        let limit = if moving { 12 } else { 16 };
        animation.advance_vehicle(&mut ch, true, moving, (limit - 1) as f32 / 60.0);
        assert_eq!(ch.frame, 1);
        animation.advance_vehicle(&mut ch, true, moving, 1.0 / 60.0);
        assert_eq!(ch.frame, 2);
        animation.advance_vehicle(&mut ch, false, moving, 1.0 / 60.0);
        assert_eq!(ch.frame, 1);
    }
}

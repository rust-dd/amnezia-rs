//! Integer-pixel 60 Hz shake from EasyRPG `Shake::NextPosition`/`Update`
//! (`reference/easyrpg-player/src/shake.h`). Per-tick cutoff bounds the displacement
//! so new or interrupted shakes ramp smoothly; one pixel equals one world unit.

use std::f64::consts::PI;

/// Seconds per RM2000 logic frame (the fixed 60 fps the recurrence assumes).
#[cfg(test)]
const FRAME_SECS: f32 = 1.0 / 60.0;

/// EasyRPG `Shake::NextPosition`: truncate the sine offset toward zero like C++,
/// then limit its displacement from the previous position by the per-tick cutoff.
pub fn next_position(strength: i32, speed: i32, time_left: i32, position: i32) -> i32 {
    let amplitude = 1 + 2 * strength;
    let phase =
        (i64::from(time_left) * 4 * (i64::from(speed) + 2)).rem_euclid(256) as f64 * PI / 128.0;
    let raw = (-(amplitude as f64 * phase.sin())) as i32;
    let cutoff = (speed * amplitude / 8) + 1;
    raw.clamp(position - cutoff, position + cutoff)
}

#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShakeState {
    strength: i32,
    speed: i32,
    time_left: i32,
    position: i32,
    accumulator: f64,
}

impl ShakeState {
    pub(super) fn valid(&self) -> bool {
        (0..=9).contains(&self.strength)
            && (0..=9).contains(&self.speed)
            && self.time_left >= 0
            && self.position.unsigned_abs() <= 19
            && self.accumulator.is_finite()
            && (0.0..1.0).contains(&self.accumulator)
    }

    pub(super) fn position(&self) -> f32 {
        self.position as f32
    }

    /// Non-positive power/duration disables shaking. Retain position when interrupted
    /// so the replacement shake continues smoothly, as in RPG_RT.
    pub fn start(&mut self, power: i32, speed: i32, secs: f32) {
        if power <= 0 || secs <= 0.0 {
            self.time_left = 0;
            self.position = 0;
            self.accumulator = 0.0;
            return;
        }
        self.strength = power;
        self.speed = speed;
        self.time_left = (secs * 60.0).round() as i32;
        self.accumulator = 0.0;
    }

    /// Advance the shake by `dt` seconds and return the horizontal camera offset
    /// (world units). Steps the 60 fps recurrence for each whole frame elapsed;
    /// returns 0 once the shake has run out.
    pub fn step(&mut self, dt: f32) -> f32 {
        if self.time_left <= 0 {
            self.position = 0;
            self.accumulator = 0.0;
            return 0.0;
        }
        self.accumulator += f64::from(dt) * 60.0;
        while self.accumulator + 1e-6 >= 1.0 && self.time_left > 0 {
            self.advance_frame();
            self.accumulator = (self.accumulator - 1.0).max(0.0);
        }
        if self.time_left == 0 {
            self.accumulator = 0.0;
        }
        self.position()
    }

    /// One 60 fps step of `Shake::Update`: decrement the timer, then either move
    /// to the next position or settle at rest when the shake ends.
    fn advance_frame(&mut self) {
        self.time_left -= 1;
        if self.time_left > 0 {
            self.position = next_position(self.strength, self.speed, self.time_left, self.position);
        } else {
            self.position = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_saved_shake_parameters_and_clocks_are_rejected() {
        for case in 0..7 {
            let mut state = ShakeState::default();
            match case {
                0 => state.strength = i32::MAX,
                1 => state.speed = -1,
                2 => state.time_left = -1,
                3 => state.position = i32::MIN,
                4 => state.accumulator = f64::INFINITY,
                5 => state.accumulator = -0.01,
                _ => state.accumulator = 1.0,
            }
            assert!(!state.valid());
        }
    }

    #[test]
    fn long_saved_shakes_do_not_overflow_the_phase_calculation() {
        let mut shake = ShakeState {
            strength: 9,
            speed: 9,
            time_left: i32::MAX,
            ..Default::default()
        };
        assert!(shake.valid());
        let period = (i64::from(i32::MAX - 1) * 44).rem_euclid(256) as i32;
        let expected = (-19.0 * (f64::from(period) * PI / 128.0).sin()) as i32;
        assert_eq!(shake.step(FRAME_SECS), expected as f32);
    }

    #[test]
    fn eight_frame_damage_shake_keeps_its_timeline_at_low_and_high_fps() {
        let positions = [0.0, 5.0, 5.0, 2.0, -2.0, -6.0, -6.0, -4.0, 0.0];
        for fps in [15, 30, 60, 144] {
            let mut clock = crate::timing::GameFrames::default();
            let mut shake = ShakeState::default();
            shake.start(3, 5, 8.0 / 60.0);
            for update in 0..fps {
                clock.advance(1.0 / fps as f64);
                let position = shake.step(1.0 / fps as f32);
                assert_eq!(
                    position,
                    positions[clock.frame.min(8) as usize],
                    "{fps} FPS, update {update}"
                );
            }
        }
    }

    #[test]
    fn next_position_matches_easyrpg_reference_values() {
        // Hand-computed from Shake::NextPosition:
        // strength 5, speed 5, time_left 10: amplitude 11, phase = 24·π/128,
        // -11·sin ≈ -6.11 → -6; cutoff = 5·11/8 + 1 = 7, so -6 is unclamped.
        assert_eq!(next_position(5, 5, 10, 0), -6);
        // strength 2, speed 3, time_left 5: amplitude 5, -5·sin(100·π/128) ≈ -3.16
        // → -3; cutoff = 3·5/8 + 1 = 2, clamping the jump from 0 to -2.
        assert_eq!(next_position(2, 3, 5, 0), -2);
    }

    #[test]
    fn amplitude_scales_with_power() {
        let weak = next_position(1, 8, 8, 0).abs();
        let strong = next_position(9, 8, 8, 0).abs();
        assert!(strong > weak);
    }

    #[test]
    fn position_stays_within_amplitude() {
        let strength = 6;
        let mut pos = 0;
        for time_left in (1..120).rev() {
            pos = next_position(strength, 5, time_left, pos);
            assert!(pos.abs() <= 1 + 2 * strength);
        }
    }

    #[test]
    fn power_zero_produces_no_offset() {
        let mut shake = ShakeState::default();
        shake.start(0, 5, 1.0);
        assert_eq!(shake.step(0.1), 0.0);
    }

    #[test]
    fn shake_ramps_in_without_snapping_and_ends_at_rest() {
        let mut shake = ShakeState::default();
        shake.start(7, 5, 0.5);
        let first = shake.step(FRAME_SECS).abs();
        let cutoff = (5 * (1 + 2 * 7) / 8) + 1;
        assert!(first <= cutoff as f32);
        assert_eq!(shake.step(2.0), 0.0);
    }

    #[test]
    fn faster_speed_advances_phase_more_per_frame() {
        let slow = next_position(5, 1, 20, 0);
        let fast = next_position(5, 9, 20, 0);
        assert_ne!(slow, fast);
    }
}

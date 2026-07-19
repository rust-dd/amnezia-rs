//! Screen-shake motion, a faithful port of EasyRPG's `Shake::NextPosition` /
//! `Shake::Update` (`reference/easyrpg-player/src/shake.h`). RM2000 drives the
//! shake in integer screen pixels at a fixed 60 fps: an amplitude of
//! `1 + 2 * strength`, a phase that advances `4 * (speed + 2)` steps (out of 256
//! per cycle) every frame, and a per-frame `cutoff` that limits how far the
//! position may jump so a starting or interrupted shake ramps in smoothly instead
//! of snapping.
//!
//! The remake runs on a real-time clock, so [`ShakeState`] accumulates elapsed
//! time and steps the integer recurrence one 60 fps frame at a time. One RM2000
//! pixel is one world unit (as for pictures), so the returned position is added
//! straight to the camera's x translation.

use std::f64::consts::PI;

/// Seconds per RM2000 logic frame (the fixed 60 fps the recurrence assumes).
const FRAME_SECS: f32 = 1.0 / 60.0;

/// The next integer shake position, a direct port of `Shake::NextPosition`.
///
/// `amplitude = 1 + 2 * strength`; the phase is
/// `(time_left * 4 * (speed + 2)) mod 256`, mapped to `[0, 2π)`; the raw offset
/// `-amplitude * sin(phase)` is truncated toward zero (C++ `double`→`int`) and
/// then clamped to within `cutoff = speed * amplitude / 8 + 1` of `position`, so
/// no single frame moves more than `cutoff` pixels.
pub fn next_position(strength: i32, speed: i32, time_left: i32, position: i32) -> i32 {
    let amplitude = 1 + 2 * strength;
    let phase = (time_left * 4 * (speed + 2)).rem_euclid(256) as f64 * PI / 128.0;
    // EasyRPG's `amplitude * sin(phase) * -1`, truncated toward zero (C++ int cast).
    let raw = (-(amplitude as f64 * phase.sin())) as i32;
    let cutoff = (speed * amplitude / 8) + 1;
    raw.clamp(position - cutoff, position + cutoff)
}

/// The live shake: the current command's strength/speed, the remaining frame
/// count, the current integer position, and a sub-frame time accumulator.
#[derive(Default)]
pub struct ShakeState {
    strength: i32,
    speed: i32,
    time_left: i32,
    position: i32,
    accumulator: f32,
}

impl ShakeState {
    /// Begin a `ShakeScreen` of `power`/`speed` lasting `secs`. `power <= 0` (or a
    /// non-positive duration) is treated as no shake, matching the remake's
    /// "power 0 → none" rule. The position is deliberately not reset, so a shake
    /// interrupting another flows on smoothly (as in RPG_RT).
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
            return 0.0;
        }
        self.accumulator += dt;
        while self.accumulator >= FRAME_SECS && self.time_left > 0 {
            self.advance_frame();
            self.accumulator -= FRAME_SECS;
        }
        self.position as f32
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
        // amplitude = 1 + 2·strength: a stronger shake reaches farther. Sampled
        // across a phase where the raw offset is near the extreme.
        let weak = next_position(1, 8, 8, 0).abs();
        let strong = next_position(9, 8, 8, 0).abs();
        assert!(strong > weak);
    }

    #[test]
    fn position_stays_within_amplitude() {
        // Over a full run the position never exceeds 1 + 2·strength in magnitude.
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
        // First frame's move is bounded by the cutoff, so it starts near centre
        // rather than jumping to full amplitude.
        let first = shake.step(FRAME_SECS).abs();
        let cutoff = (5 * (1 + 2 * 7) / 8) + 1;
        assert!(first <= cutoff as f32);
        // After well past its duration the shake settles back to zero.
        assert_eq!(shake.step(2.0), 0.0);
    }

    #[test]
    fn faster_speed_advances_phase_more_per_frame() {
        // The phase step per frame is 4·(speed + 2); a higher speed reaches a
        // different phase from the same starting frame, i.e. oscillates faster.
        let slow = next_position(5, 1, 20, 0);
        let fast = next_position(5, 9, 20, 0);
        assert_ne!(slow, fast);
    }
}

//! The interpreter's random source for event rolls — today the `ControlVariables`
//! "random between" operand (opcode 10220, operand type 3). A little self-contained
//! PCG-style LCG (the same one the battle uses), seeded once from the clock when
//! the resource is created and advanced per draw, so dice/chance events vary
//! between sessions yet stay deterministic within a run — no per-call `Date`/`Math`
//! nondeterminism.

use bevy::prelude::*;

/// The event RNG resource: a 64-bit LCG state advanced on each draw.
#[derive(Resource)]
pub(crate) struct EventRng {
    state: u64,
}

impl Default for EventRng {
    fn default() -> Self {
        Self::seeded(time_seed())
    }
}

impl EventRng {
    /// A deterministic instance for tests; the low bit is forced set so the LCG
    /// never starts from a degenerate all-zero state.
    pub(super) fn seeded(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 33) ^ self.state
    }

    /// A uniform integer in `[lo, hi]` inclusive, mirroring EasyRPG
    /// `Rand::GetRandomNumber` (which orders the bounds and includes both ends).
    pub(super) fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        let width = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % width) as i64) as i32
    }
}

/// A time-derived seed, read once when [`EventRng`] is first created (never per
/// draw). Falls back to a fixed golden-ratio constant if the clock is unavailable.
fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn range_stays_within_inclusive_bounds_and_orders_them() {
        let mut rng = EventRng::seeded(0xABCDEF);
        let mut seen = HashSet::new();
        for _ in 0..500 {
            let v = rng.range(8, 3); // deliberately reversed bounds
            assert!((3..=8).contains(&v), "draw {v} outside [3, 8]");
            seen.insert(v);
        }
        // Both endpoints are reachable and the value is not pinned to one number.
        assert!(seen.contains(&3) && seen.contains(&8));
        assert!(seen.len() > 1);
    }

    #[test]
    fn a_singleton_range_returns_its_only_value() {
        let mut rng = EventRng::seeded(1);
        assert_eq!(rng.range(7, 7), 7);
    }
}

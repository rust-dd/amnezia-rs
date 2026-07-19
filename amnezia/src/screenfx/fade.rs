//! Erase/Show screen-fade timing, from EasyRPG's transition defaults
//! (`reference/easyrpg-player/src/transition.cpp` `GetDefaultFrames`) and the
//! `EraseScreen`/`ShowScreen` command's `params[0]` transition-type selector
//! (`game_interpreter.cpp`). RM2000 runs each transition for a fixed number of
//! 60 fps frames; the remake renders every type as a plain black fade but honours
//! its duration, which is the visible difference between them.

/// Seconds a `params[0]` erase/show transition should take. The remake fades to
/// black for all of them; only the duration varies by type.
pub fn transition_secs(transition_type: i32) -> f32 {
    transition_frames(transition_type) as f32 / 60.0
}

/// The RM2000 default frame count for the transition selected by `params[0]` of
/// `EraseScreen`/`ShowScreen`. `-1` is the map-transfer default (a fade); `0` is
/// the explicit fade; `1..=18` are the block/stripe/scroll effects the remake
/// approximates with a plain fade of the same 41-frame length; `19` is the near-
/// instant cut; anything else is "none" (an instant switch).
fn transition_frames(transition_type: i32) -> i32 {
    match transition_type {
        -1 | 0 => 35,
        1..=18 => 41,
        19 => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_transition_is_thirty_five_frames() {
        // The default fade (params[0] == 0) and the transfer default (-1) both run
        // 35 frames ≈ 0.583 s — 2.3× the old fixed 0.25 s.
        assert!((transition_secs(0) - 35.0 / 60.0).abs() < 1e-6);
        assert!((transition_secs(-1) - 35.0 / 60.0).abs() < 1e-6);
    }

    #[test]
    fn other_effects_run_forty_one_frames() {
        for tt in 1..=18 {
            assert!((transition_secs(tt) - 41.0 / 60.0).abs() < 1e-6);
        }
    }

    #[test]
    fn cut_is_one_frame_and_none_is_instant() {
        assert!((transition_secs(19) - 1.0 / 60.0).abs() < 1e-6);
        assert_eq!(transition_secs(20), 0.0);
        assert_eq!(transition_secs(99), 0.0);
    }
}

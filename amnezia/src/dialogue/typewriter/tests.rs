use super::*;

fn writer(raw: &str) -> Typewriter {
    Typewriter::new(raw, "Ron", &Variables::default())
}

fn timeline(raw: &str, complete: u32, changes: &[(u32, &str)]) {
    let mut tw = writer(raw);
    let mut shown = "";
    for tick in 1..=complete {
        tw.tick();
        if let Some((_, text)) = changes.iter().find(|(frame, _)| *frame == tick) {
            shown = text;
        }
        assert_eq!(tw.text(), shown, "{raw:?} at {tick}");
        assert_eq!(tw.is_complete(), tick == complete, "{raw:?} at {tick}");
    }
}

#[test]
fn original_line_page_and_speed_boundaries_have_exact_waits() {
    timeline("abcdef", 5, &[(1, "ab"), (2, "abcd"), (3, "abcdef")]);
    timeline("ab\ncd", 4, &[(1, "ab"), (2, "ab\ncd")]);
    timeline("a\nbc", 4, &[(1, "a"), (2, "a\nbc")]);
    timeline("a\n\nbc", 5, &[(1, "a"), (2, "a\n\n"), (3, "a\n\nbc")]);
    timeline(
        "\\S[5]abcd",
        12,
        &[(1, "a"), (4, "ab"), (7, "abc"), (10, "abcd")],
    );
    timeline("a\\S[5]bc", 7, &[(1, "a"), (2, "ab"), (5, "abc")]);
    timeline("ab\\S[5]cd", 7, &[(1, "ab"), (2, "abc"), (5, "abcd")]);
}

#[test]
fn pauses_and_automatic_page_close_preserve_control_code_parity() {
    timeline("a\\|b", 65, &[(1, "a"), (63, "ab")]);
    timeline("ab\\|c", 65, &[(1, "ab"), (63, "abc")]);
    timeline("ab\\|c\\^", 67, &[(1, "ab"), (63, "abc")]);
    timeline("a\\^", 5, &[(1, "a")]);
    timeline("ab\\^", 5, &[(1, "ab")]);
    timeline("\\S[5]a\\^", 11, &[(1, "a")]);
    timeline("", 3, &[]);
}

#[test]
fn instant_speed_stops_at_the_newline_and_keeps_the_final_page_wait() {
    timeline("\\>ab\ncdef", 4, &[(1, "ab\ncd"), (2, "ab\ncdef")]);
    timeline("\\>abcd", 3, &[(1, "abcd")]);
}

#[test]
fn the_quarter_pause_grows_at_the_four_slowest_speeds() {
    for speed in 1_u8..=20 {
        let mut tw = writer(&format!("\\S[{speed}]\\.a"));
        let delay = 16 + u32::from(speed.saturating_sub(16));
        for _ in 0..delay {
            tw.tick();
            assert_eq!(tw.text(), "");
        }
        tw.tick();
        assert_eq!(tw.text(), "a", "speed {speed}");
    }
}

#[test]
fn original_default_cadence_draws_two_half_width_glyphs_per_logical_tick() {
    let mut tw = writer("abcdef");
    tw.tick();
    assert_eq!(tw.text(), "ab");
    tw.tick();
    assert_eq!(tw.text(), "abcd");
}

#[test]
fn the_last_glyph_waits_two_ticks_before_the_page_is_complete() {
    let mut tw = writer("abcd");
    tw.tick();
    assert_eq!(tw.text(), "ab");
    tw.tick();
    assert_eq!(tw.text(), "abcd");
    assert!(!tw.is_complete());
    tw.tick();
    assert!(!tw.is_complete());
    tw.tick();
    assert!(tw.is_complete());
}

#[test]
fn completes_after_the_last_glyph() {
    let mut tw = writer("hi");
    for _ in 0..8 {
        tw.tick();
    }
    assert!(tw.is_complete());
    assert_eq!(tw.text(), "hi");
}

#[test]
fn fast_forward_reveals_the_whole_page() {
    let mut tw = writer("a longer line");
    tw.tick();
    tw.fast_forward();
    assert_eq!(tw.text(), "a longer line");
    assert!(tw.is_complete());
}

#[test]
fn full_pause_delays_the_next_glyph() {
    let mut tw = writer("a\\|b");
    tw.tick();
    assert_eq!(tw.text(), "a");
    for _ in 0..FULL_PAUSE_FRAMES {
        assert_eq!(tw.text(), "a");
        tw.tick();
    }
    tw.tick();
    assert_eq!(tw.text(), "ab");
}

#[test]
fn kill_page_marks_completion_without_a_key() {
    let mut tw = writer("x\\^");
    for _ in 0..6 {
        tw.tick();
    }
    assert!(tw.is_complete());
    assert!(tw.kill_page());
    assert_eq!(tw.text(), "x");
}

#[test]
fn wait_key_pauses_until_resumed() {
    let mut tw = writer("a\\!b");
    for _ in 0..6 {
        tw.tick();
    }
    assert!(tw.waiting_for_key());
    assert!(!tw.is_complete());
    assert_eq!(tw.text(), "a");
    tw.resume();
    for _ in 0..6 {
        tw.tick();
    }
    assert_eq!(tw.text(), "ab");
    assert!(tw.is_complete());
}

#[test]
fn higher_speed_reveals_more_slowly() {
    let mut tw = writer("\\s[10]ab");
    tw.tick();
    assert_eq!(tw.text(), "a");
    for _ in 0..6 {
        tw.tick();
    }
    assert_eq!(tw.text(), "ab");
}

#[test]
fn instant_run_reveals_without_waiting() {
    let mut tw = writer("\\>abcdef\\<");
    tw.tick();
    assert_eq!(tw.text(), "abcdef");
}

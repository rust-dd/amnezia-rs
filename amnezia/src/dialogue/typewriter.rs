//! The per-page letter-by-letter reveal. RM2000 prints a message one glyph at a
//! time, spacing the glyphs by a frame budget the `\s[n]` speed code scales, and
//! pausing on the timing codes (`\.`, `\|`, `\!`). This mirrors EasyRPG's
//! `Window_Message::UpdateMessage`: a per-frame [`Typewriter::tick`] consumes a
//! wait budget and appends glyphs, the confirm key [`Typewriter::fast_forward`]s
//! to the full page, and `\^` marks the page to [`Typewriter::kill_page`] itself.

use crate::state::Variables;
use crate::text::{Segment, parse_segments};
use bevy::prelude::*;

/// `\.` reveal pause, in frames (EasyRPG waits 16 despite the "quarter second").
const QUARTER_PAUSE_FRAMES: u32 = 16;
/// `\|` reveal pause, in frames (EasyRPG waits 61 despite the "one second").
const FULL_PAUSE_FRAMES: u32 = 61;

/// Frames a single glyph occupies at a given RM2000 speed. Speed 1 (the default)
/// reveals about one glyph per frame; higher speeds slow the reveal, matching
/// EasyRPG's `speed * width / 2 + 1` for half-width glyphs.
fn glyph_frames(speed: u8) -> u32 {
    let speed = speed.clamp(1, 20) as u32;
    if speed <= 1 { 1 } else { speed / 2 + 1 }
}

/// The reveal state of one message page: the parsed [`Segment`] stream, how far
/// into it the reveal has progressed, the glyphs shown so far, and the RM2000
/// timing state (current speed, remaining wait, instant-speed and pause flags).
#[derive(Debug, Default)]
pub(super) struct Typewriter {
    segments: Vec<Segment>,
    cursor: usize,
    revealed: String,
    speed: u8,
    wait: u32,
    instant: bool,
    waiting_key: bool,
    kill_page: bool,
    done: bool,
}

impl Typewriter {
    /// Parse `raw` (one page, its lines joined by `\n`) into a fresh reveal,
    /// expanding `\N`/`\V` against `hero`/`variables`.
    pub(super) fn new(raw: &str, hero: &str, variables: &Variables) -> Self {
        Self {
            segments: parse_segments(raw, hero, variables),
            speed: 1,
            ..Default::default()
        }
    }

    /// The glyphs revealed so far.
    pub(super) fn text(&self) -> &str {
        &self.revealed
    }

    /// Whether the whole page has been revealed and no reveal work remains.
    pub(super) fn is_complete(&self) -> bool {
        self.done
    }

    /// Whether a `\!` code paused the reveal waiting for the confirm key.
    pub(super) fn waiting_for_key(&self) -> bool {
        self.waiting_key
    }

    /// Whether a `\^` code asked the page to close itself once revealed.
    pub(super) fn kill_page(&self) -> bool {
        self.kill_page
    }

    /// Advance the reveal by one frame: spend a frame of the current wait, or
    /// consume the next segment (append a glyph, apply a speed/pause code, or stop
    /// at a `\!` key-wait). Instant runs (`\>`) and completion append with no wait.
    pub(super) fn tick(&mut self) {
        if self.done || self.waiting_key {
            return;
        }
        loop {
            if self.wait > 0 {
                self.wait -= 1;
                return;
            }
            let Some(segment) = self.segments.get(self.cursor) else {
                self.done = true;
                return;
            };
            self.cursor += 1;
            match segment {
                Segment::Char(c) => {
                    self.revealed.push(*c);
                    if !self.instant {
                        self.wait = glyph_frames(self.speed);
                    }
                }
                Segment::Speed(n) => self.speed = (*n).clamp(1, 20),
                Segment::QuarterPause => {
                    if !self.instant {
                        self.wait = QUARTER_PAUSE_FRAMES;
                    }
                }
                Segment::FullPause => {
                    if !self.instant {
                        self.wait = FULL_PAUSE_FRAMES;
                    }
                }
                Segment::WaitKey => {
                    self.waiting_key = true;
                    return;
                }
                Segment::KillPage => self.kill_page = true,
                Segment::InstantOn => self.instant = true,
                Segment::InstantOff => self.instant = false,
            }
        }
    }

    /// Reveal the rest of the page at once (the confirm key pressed mid-reveal),
    /// stopping at a `\!` key-wait so a mid-text pause still holds. Applies the
    /// speed/instant/kill-page codes it passes and drops the reveal pauses.
    pub(super) fn fast_forward(&mut self) {
        while let Some(segment) = self.segments.get(self.cursor) {
            self.cursor += 1;
            match segment {
                Segment::Char(c) => self.revealed.push(*c),
                Segment::Speed(n) => self.speed = (*n).clamp(1, 20),
                Segment::WaitKey => {
                    self.waiting_key = true;
                    self.wait = 0;
                    return;
                }
                Segment::KillPage => self.kill_page = true,
                Segment::InstantOn => self.instant = true,
                Segment::InstantOff => self.instant = false,
                Segment::QuarterPause | Segment::FullPause => {}
            }
        }
        self.wait = 0;
        self.done = true;
    }

    /// Release a `\!` key-wait so the reveal resumes on the next tick.
    pub(super) fn resume(&mut self) {
        self.waiting_key = false;
    }
}

/// Drive the active page's reveal: build a [`Typewriter`] for the current box the
/// first frame it shows, tick it forward, and — when a `\^` page finishes —
/// advance past it automatically without waiting for the confirm key.
pub(super) fn drive_reveal(
    mut dialogue: ResMut<super::Dialogue>,
    hero: Res<crate::text::HeroName>,
    variables: Res<Variables>,
) {
    if !dialogue.active {
        return;
    }
    if dialogue.reveal.is_none() {
        let index = dialogue.index;
        if let Some(raw) = dialogue.boxes.get(index).map(|b| b.lines.join("\n")) {
            dialogue.reveal = Some(Typewriter::new(&raw, &hero.0, &variables));
        }
    }
    if let Some(reveal) = dialogue.reveal.as_mut() {
        reveal.tick();
    }
    let auto_close = dialogue
        .reveal
        .as_ref()
        .is_some_and(|r| r.is_complete() && r.kill_page());
    if auto_close {
        dialogue.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn writer(raw: &str) -> Typewriter {
        Typewriter::new(raw, "Ron", &Variables::default())
    }

    #[test]
    fn reveals_one_glyph_per_tick_at_default_speed() {
        let mut tw = writer("abcd");
        tw.tick();
        assert_eq!(tw.text(), "a");
        tw.tick();
        assert_eq!(tw.text(), "ab");
        tw.tick();
        tw.tick();
        assert_eq!(tw.text(), "abcd");
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
        tw.tick(); // reveal 'a'
        assert_eq!(tw.text(), "a");
        tw.tick(); // hit the pause: schedules the full-second wait
        for _ in 0..(FULL_PAUSE_FRAMES - 1) {
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
        // Speed 10 budgets glyph_frames(10) = 6 frames per glyph, so one tick in
        // the first glyph is on screen but the second has not started.
        assert_eq!(tw.text(), "a");
        for _ in 0..glyph_frames(10) {
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
}

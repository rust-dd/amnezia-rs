//! Western message reveal on the shared 60 Hz clock, including paired half-width
//! glyphs, control-code parity, line/page waits and automatic page completion.

use crate::state::Variables;
use crate::text::{Segment, parse_segments};
use bevy::prelude::*;

mod cadence;
mod pause_arrow;
pub(crate) mod smoke;
use cadence::Cadence;
use pause_arrow::PauseArrow;
#[cfg(test)]
mod campaign_tests;

/// `\.` reveal pause, in frames (EasyRPG waits 16 despite the "quarter second").
const QUARTER_PAUSE_FRAMES: u32 = 16;
/// `\|` reveal pause, in frames (EasyRPG waits 61 despite the "one second").
const FULL_PAUSE_FRAMES: u32 = 61;

/// The reveal state of one message page: the parsed [`Segment`] stream, how far
/// into it the reveal has progressed, the glyphs shown so far, and the RM2000
/// timing state (current speed, remaining wait, instant-speed and pause flags).
#[derive(Debug, Default)]
pub(super) struct Typewriter {
    segments: Vec<Segment>,
    cursor: usize,
    revealed: String,
    cadence: Cadence,
    pause: PauseArrow,
    wait: u32,
    last_frame: Option<u32>,
    finishing: bool,
    waiting_key: bool,
    kill_page: bool,
    end_prompt: bool,
    done: bool,
}

impl Typewriter {
    /// Parse `raw` (one page, its lines joined by `\n`) into a fresh reveal,
    /// expanding `\N`/`\V` against `hero`/`variables`.
    pub(super) fn new(raw: &str, hero: &str, variables: &Variables) -> Self {
        Self {
            segments: parse_segments(raw, hero, variables),
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

    pub(super) fn arrow_visible(&self) -> bool {
        self.pause.visible()
    }

    pub(super) fn expect_prompt(&mut self) {
        self.end_prompt = true;
    }

    pub(super) fn page_finished(&self) -> bool {
        self.finishing
    }

    /// Spend one logical tick, consuming segments until a delay or key-wait.
    pub(super) fn tick(&mut self) {
        self.pause.advance(1);
        if self.done {
            return;
        }
        loop {
            if self.wait > 0 {
                self.wait -= 1;
                return;
            }
            if self.waiting_key {
                return;
            }
            let Some(segment) = self.segments.get(self.cursor) else {
                if self.finishing {
                    self.done = true;
                    return;
                }
                self.finishing = true;
                self.pause.set(!self.kill_page && !self.end_prompt);
                self.wait = self.cadence.newline(true);
                continue;
            };
            self.cursor += 1;
            match segment {
                Segment::Char('\n') => {
                    self.revealed.push('\n');
                    self.finishing = self.cursor == self.segments.len();
                    if self.finishing {
                        self.pause.set(!self.kill_page && !self.end_prompt);
                    }
                    self.wait = self.cadence.newline(self.finishing);
                }
                Segment::Char(c) if !c.is_control() => {
                    self.revealed.push(*c);
                    let next = self.segments.get(self.cursor);
                    let last_line = next.is_none_or(|next| matches!(next, Segment::Char('\n')));
                    let last_page =
                        next.is_none() || (last_line && self.cursor + 1 == self.segments.len());
                    self.wait = self.cadence.character(last_line, last_page);
                }
                Segment::Char(_) => {}
                Segment::Speed(n) => {
                    self.wait = self.cadence.control(0);
                    self.cadence.speed = (*n).clamp(1, 20);
                }
                Segment::QuarterPause => {
                    self.wait = self.cadence.control(
                        QUARTER_PAUSE_FRAMES + u32::from(self.cadence.speed.saturating_sub(16)),
                    );
                }
                Segment::FullPause => self.wait = self.cadence.control(FULL_PAUSE_FRAMES),
                Segment::WaitKey => {
                    self.wait = self.cadence.control(0);
                    self.waiting_key = true;
                    self.pause.set(true);
                }
                Segment::KillPage => {
                    self.kill_page = true;
                    self.wait = self.cadence.control(u32::from(self.cadence.speed));
                }
                Segment::InstantOn => {
                    self.wait = self.cadence.control(0);
                    self.cadence.instant = true;
                }
                Segment::InstantOff => {
                    self.cadence.instant = false;
                    self.wait = self.cadence.control(u32::from(self.cadence.speed));
                }
            }
        }
    }

    /// Release a `\!` key-wait so the reveal resumes on the next tick.
    pub(super) fn resume(&mut self) {
        self.waiting_key = false;
        self.pause.set(false);
    }
}

/// Consume logical ticks without charging a new page for time before it opened.
pub(in crate::dialogue) fn drive_reveal(
    mut dialogue: ResMut<super::Dialogue>,
    hero: Res<crate::text::HeroName>,
    variables: Res<Variables>,
    frames: Res<crate::timing::GameFrames>,
    scene: crate::world::ScenePause,
) {
    if !dialogue.active {
        return;
    }
    let ticks = dialogue.reveal.as_mut().map_or(1, |reveal| {
        let previous = reveal.last_frame.replace(frames.frame);
        previous.map_or(1, |previous| frames.frame.wrapping_sub(previous))
    });
    if scene.screen_effects_paused() {
        return;
    }
    for tick in 0..ticks {
        if dialogue.reveal.is_none() {
            let Some(raw) = dialogue
                .boxes
                .get(dialogue.index)
                .map(|page| page.lines.join("\n"))
            else {
                dialogue.close();
                break;
            };
            let mut reveal = Typewriter::new(&raw, &hero.0, &variables);
            if dialogue.embedded_prompt().is_some() {
                reveal.expect_prompt();
            }
            reveal.last_frame = Some(frames.frame);
            dialogue.reveal = Some(reveal);
        }
        let reveal = dialogue.reveal.as_mut().unwrap();
        reveal.tick();
        if reveal.is_complete() && reveal.kill_page() && !reveal.end_prompt {
            dialogue.advance();
            if !dialogue.active {
                break;
            }
        } else if reveal.is_complete() || reveal.waiting_for_key() {
            reveal.pause.advance(ticks - tick - 1);
            break;
        }
    }
}

#[cfg(test)]
mod clock_tests;
#[cfg(test)]
mod tests;

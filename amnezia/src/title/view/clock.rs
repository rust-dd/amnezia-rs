use super::*;
use crate::timing::GameFrames;

#[derive(Resource, Default)]
pub(in crate::title) struct Clock {
    last: Option<u32>,
    visible: bool,
    pub(super) opened: u32,
    cursor: u32,
}

impl Clock {
    fn advance(&mut self, now: u32, visible: bool, running: bool) {
        let delta = self
            .last
            .replace(now)
            .map_or(0, |last| now.wrapping_sub(last));
        if !visible || !self.visible {
            self.opened = 0;
            self.cursor = 0;
        } else if running {
            self.opened = self.opened.saturating_add(delta).min(8);
            self.cursor = (self.cursor + delta % 21) % 21;
        }
        self.visible = visible;
    }

    pub(super) fn source_x(&self) -> f32 {
        if self.cursor <= 10 { 64.0 } else { 96.0 }
    }
}

pub(in crate::title) fn tick(
    title: Res<TitleActive>,
    state: Res<TitleState>,
    frames: Res<GameFrames>,
    pause: crate::transitions::TransitionPause,
    mut clock: ResMut<Clock>,
) {
    clock.advance(
        frames.frame,
        title.0 && state.stage.visible(),
        state.stage == flow::Stage::Ready && !pause.paused(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_and_cursor_start_after_the_title_fade_at_every_frame_rate() {
        for fps in [15, 30, 60, 144] {
            let mut clock = Clock::default();
            clock.advance(0, true, false);
            clock.advance(35, true, false);
            let mut frames = GameFrames::default();
            for _ in 0..fps {
                frames.advance(1.0 / fps as f64);
                clock.advance(35 + frames.frame, true, true);
                assert_eq!(clock.opened, frames.frame.min(8));
                assert_eq!(clock.cursor, frames.frame % 21);
            }
        }
    }

    #[test]
    fn transitions_hold_both_clocks_and_reentry_restarts_the_opening() {
        let mut clock = Clock::default();
        clock.advance(0, true, false);
        clock.advance(4, true, true);
        clock.advance(500, true, false);
        assert_eq!((clock.opened, clock.cursor), (4, 4));
        clock.advance(501, true, true);
        assert_eq!((clock.opened, clock.cursor), (5, 5));
        clock.advance(502, false, false);
        clock.advance(2000, true, false);
        assert_eq!((clock.opened, clock.cursor), (0, 0));
        clock.advance(2001, true, true);
        assert_eq!((clock.opened, clock.cursor), (1, 1));
    }

    #[test]
    fn wrapping_the_global_clock_does_not_finish_the_opening_early() {
        let mut clock = Clock::default();
        clock.advance(u32::MAX - 2, true, false);
        clock.advance(1, true, true);
        assert_eq!((clock.opened, clock.cursor), (4, 4));
        clock.advance(8, true, true);
        assert_eq!(clock.source_x(), 96.0);
    }
}

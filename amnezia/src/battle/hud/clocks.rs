use super::*;
use crate::gamedata::GameData;
use crate::state::Inventory;
use crate::terms::Terms;
use crate::timing::GameFrames;

#[derive(Resource, Default)]
pub(super) struct WindowClocks {
    generation: u64,
    last: Option<u32>,
    cursor: [u32; 8],
    arrow: [u32; 8],
    counts: [usize; 8],
}

impl WindowClocks {
    fn advance(&mut self, frames: u32, active: Option<Panel>) {
        for i in 0..8 {
            if active.is_some_and(|active| active as usize == i) {
                self.cursor[i] = (self.cursor[i] + frames % 21) % 21;
            }
            let capacity = if i == Panel::Item as usize || i == Panel::Skill as usize {
                8
            } else {
                4
            };
            if self.counts[i] > capacity {
                self.arrow[i] = (self.arrow[i] + frames % 40) % 40;
            }
        }
    }

    pub fn cursor_x(&self, panel: Panel) -> f32 {
        if self.cursor[panel as usize] <= 10 {
            64.0
        } else {
            96.0
        }
    }

    pub fn arrows(&self, panel: Panel, first: usize) -> [bool; 2] {
        let id = panel as usize;
        let capacity = panel.columns() * 4;
        if self.arrow[id] >= 20 {
            return [false; 2];
        }
        [first > 0, first + capacity < self.counts[id]]
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn tick(
    frames: Res<GameFrames>,
    battle: Res<Battle>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    pause: crate::transitions::TransitionPause,
    windows: Res<motion::CommandWindows>,
    mut clocks: ResMut<WindowClocks>,
) {
    if clocks.generation != battle.generation {
        *clocks = WindowClocks {
            generation: battle.generation,
            ..default()
        };
    }
    let delta = clocks
        .last
        .replace(frames.frame)
        .map_or(0, |last| frames.frame.wrapping_sub(last));
    if pause.paused() || battle.phase == Phase::Inactive {
        return;
    }
    for panel in Panel::ALL {
        if layout::rectangle(panel, &battle).is_some() {
            let id = panel as usize;
            clocks.counts[id] = if panel == Panel::Status {
                battle.members.len()
            } else {
                content::rows(panel, &battle, &data, &inventory, &terms).len()
            };
        }
    }
    let active = if windows.moving() {
        None
    } else {
        Panel::active(&battle)
    };
    clocks.advance(delta, active);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_and_scroll_arrows_keep_their_original_cycles_at_different_fps() {
        for fps in [15, 30, 60, 144] {
            let mut clock = GameFrames::default();
            let mut windows = WindowClocks::default();
            windows.counts[Panel::Skill as usize] = 12;
            for _ in 0..fps * 3 {
                let before = clock.frame;
                clock.advance(1.0 / fps as f64);
                windows.advance(clock.frame - before, Some(Panel::Skill));
                assert_eq!(windows.cursor[Panel::Skill as usize], clock.frame % 21);
                assert_eq!(windows.arrow[Panel::Skill as usize], clock.frame % 40);
                assert_eq!(windows.cursor[Panel::Status as usize], 0);
            }
        }
    }

    #[test]
    fn scroll_arrows_point_only_to_hidden_rows_and_share_the_twenty_frame_blink() {
        let mut clock = WindowClocks::default();
        clock.counts[Panel::Skill as usize] = 12;
        assert_eq!(clock.arrows(Panel::Skill, 0), [false, true]);
        assert_eq!(clock.arrows(Panel::Skill, 2), [true, true]);
        assert_eq!(clock.arrows(Panel::Skill, 4), [true, false]);
        clock.advance(20, Some(Panel::Skill));
        assert_eq!(clock.arrows(Panel::Skill, 2), [false, false]);
        clock.advance(20, None);
        assert_eq!(clock.arrows(Panel::Skill, 2), [true, true]);
        assert_eq!(clock.cursor[Panel::Skill as usize], 20);
    }
}

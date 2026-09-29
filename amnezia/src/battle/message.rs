use bevy::prelude::*;

pub(in crate::battle) mod encounter;
mod escape;
pub(in crate::battle) mod smoke;
#[cfg(test)]
mod tests;
mod wait;

pub(super) use wait::{Controls, Wait};

#[derive(Default)]
pub(in crate::battle) struct Console {
    lines: Vec<String>,
    first: usize,
    painted: Vec<String>,
}

impl Console {
    pub(in crate::battle) fn clear(&mut self) {
        self.lines.clear();
        self.first = 0;
    }

    pub(super) fn push(&mut self, text: &str) {
        self.lines.extend(text.split('\n').map(str::to_owned));
    }

    pub(super) fn full(&self) -> bool {
        self.lines.len() >= 4
    }

    pub(in crate::battle) fn len(&self) -> usize {
        self.lines.len()
    }

    pub(in crate::battle) fn pop_until(&mut self, lines: usize) {
        self.lines.truncate(lines);
        self.scroll_to_end();
    }

    pub(in crate::battle) fn scroll_to_end(&mut self) {
        self.first = self.lines.len().saturating_sub(4);
    }

    pub(in crate::battle) fn visible(&self) -> &[String] {
        &self.painted
    }

    fn contents(&self) -> &[String] {
        &self.lines[self.first..self.lines.len().min(self.first + 4)]
    }

    pub(in crate::battle) fn update(&mut self) {
        self.painted = self.contents().to_vec();
    }
}

#[derive(Default)]
pub(in crate::battle) struct Messages {
    pub(in crate::battle) console: Console,
    wait: Wait,
    encounter: encounter::Encounter,
    escaped: bool,
    last_frame: Option<u32>,
}

pub(super) fn tick(
    frames: Res<crate::timing::SceneFrames>,
    pause: Res<crate::timing::SceneWait>,
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<super::Battle>,
) {
    use super::Phase;
    if battle.phase == Phase::Inactive {
        return;
    }
    let delta = battle
        .messages
        .last_frame
        .replace(frames.frame)
        .map_or(1, |last| frames.frame.wrapping_sub(last));
    if pause.0 {
        return;
    }
    let controls = Controls::from_keys(&keys);
    for _ in 0..delta {
        let finished = match battle.phase {
            Phase::Encounter => encounter::advance(&mut battle, controls),
            Phase::Escape => escape::advance(&mut battle, controls),
            _ => {
                battle.messages.console.update();
                true
            }
        };
        if finished {
            break;
        }
    }
}

impl super::Battle {
    pub(super) fn begin_encounter(&mut self) {
        self.phase = super::Phase::Encounter;
        self.messages = Messages::default();
        self.log.clear();
    }
}

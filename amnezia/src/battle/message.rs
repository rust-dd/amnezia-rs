use bevy::prelude::*;

pub(in crate::battle) mod encounter;
pub(in crate::battle) mod smoke;
#[cfg(test)]
mod tests;
mod wait;

pub(super) use encounter::tick;
pub(super) use wait::{Controls, Wait};

#[derive(Default)]
pub(in crate::battle) struct Console {
    lines: Vec<String>,
    first: usize,
    painted: Vec<String>,
}

impl Console {
    pub(super) fn clear(&mut self) {
        self.lines.clear();
        self.first = 0;
    }

    pub(super) fn push(&mut self, text: &str) {
        self.lines.extend(text.split('\n').map(str::to_owned));
    }

    pub(super) fn full(&self) -> bool {
        self.lines.len() >= 4
    }

    pub(in crate::battle) fn visible(&self) -> &[String] {
        &self.painted
    }

    fn contents(&self) -> &[String] {
        &self.lines[self.first..self.lines.len().min(self.first + 4)]
    }

    fn update(&mut self) {
        self.painted = self.contents().to_vec();
    }
}

#[derive(Default)]
pub(in crate::battle) struct Messages {
    pub(in crate::battle) console: Console,
    wait: Wait,
    encounter: encounter::Encounter,
    last_frame: Option<u32>,
}

impl super::Battle {
    pub(super) fn begin_encounter(&mut self) {
        self.phase = super::Phase::Encounter;
        self.messages = Messages::default();
        self.log.clear();
    }
}

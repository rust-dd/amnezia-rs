use super::Dialogue;
use crate::choice::Choice;
use crate::inputnumber::InputNumber;
use bevy::prelude::*;

pub(crate) mod smoke;
#[cfg(test)]
mod tests;

pub(crate) enum MessagePrompt {
    Choice {
        labels: Vec<String>,
        indent: u32,
        cancel: i32,
    },
    Number {
        digits: u32,
        variable: u32,
    },
}

pub(super) struct Embedded {
    page: usize,
    pub(super) line: usize,
    kind: MessagePrompt,
    started: bool,
}

impl Embedded {
    pub(super) fn number(&self) -> bool {
        matches!(self.kind, MessagePrompt::Number { .. })
    }
}

impl Dialogue {
    pub(crate) fn open_prompt(&mut self, kind: MessagePrompt) -> bool {
        let face = self.face.graphic();
        let page = crate::events::MessageBox {
            face: face.map(|(name, _)| name.to_string()),
            face_index: face.map_or(0, |(_, index)| index),
            lines: Vec::new(),
        };
        self.open(vec![page]);
        if self.append_prompt(kind) {
            true
        } else {
            self.close();
            false
        }
    }

    pub(crate) fn append_prompt(&mut self, mut kind: MessagePrompt) -> bool {
        let Some(page) = self.boxes.last_mut() else {
            return false;
        };
        let count = match &mut kind {
            MessagePrompt::Choice { labels, .. } => {
                labels.truncate(4);
                labels.len()
            }
            MessagePrompt::Number { .. } => 1,
        };
        if count == 0 || page.lines.len() + count > 4 {
            return false;
        }
        let line = page.lines.len();
        if let MessagePrompt::Choice { labels, .. } = &kind {
            page.lines.extend(labels.iter().cloned());
        }
        self.prompt = Some(Embedded {
            page: self.boxes.len() - 1,
            line,
            kind,
            started: false,
        });
        true
    }

    pub(super) fn embedded_prompt(&self) -> Option<&Embedded> {
        self.prompt
            .as_ref()
            .filter(|prompt| self.active && self.index == prompt.page)
    }

    pub(crate) fn prompt_input_ready(&self) -> bool {
        self.embedded_prompt().is_none()
            || self
                .reveal
                .as_ref()
                .is_some_and(|reveal| reveal.is_complete())
    }
}

pub(super) fn update(
    mut dialogue: ResMut<Dialogue>,
    mut choice: Option<ResMut<Choice>>,
    mut number: Option<ResMut<InputNumber>>,
    hero: Res<crate::text::HeroName>,
    variables: Res<crate::state::Variables>,
    pause: super::MessagePause,
) {
    let Some(prompt) = dialogue.embedded_prompt() else {
        return;
    };
    if pause.paused() {
        return;
    }
    if prompt.started {
        let active = if prompt.number() {
            number.as_ref().is_some_and(|number| number.active())
        } else {
            choice.as_ref().is_some_and(|choice| choice.active())
        };
        if !active {
            dialogue.close();
        }
        return;
    }
    if !dialogue
        .reveal
        .as_ref()
        .is_some_and(|reveal| reveal.page_finished())
    {
        return;
    }
    match &prompt.kind {
        MessagePrompt::Choice {
            labels,
            indent,
            cancel,
        } => {
            let Some(choice) = choice.as_mut() else {
                return;
            };
            let labels = labels
                .iter()
                .map(|label| crate::text::substitute(label, &hero.0, &variables))
                .collect();
            choice.open(labels, *indent, *cancel);
        }
        MessagePrompt::Number { digits, variable } => {
            let Some(number) = number.as_mut() else {
                return;
            };
            number.open(*digits, *variable);
        }
    }
    dialogue.prompt.as_mut().unwrap().started = true;
}

use super::{Dialogue, visible_if};
use crate::choice::Choice;
use crate::font::bitmap::{DEFAULT, DISABLED, Run};
use crate::inputnumber::InputNumber;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod clock;
pub(crate) mod smoke;
pub(crate) use clock::Clock;
pub(in crate::dialogue) use clock::register;

#[derive(SystemParam)]
pub(in crate::dialogue) struct Presentation<'w> {
    choice: Option<Res<'w, Choice>>,
    number: Option<Res<'w, InputNumber>>,
}

impl Presentation<'_> {
    pub(in crate::dialogue) fn key(&self) -> (u8, u64) {
        if let Some(choice) = self.choice.as_deref().filter(|choice| choice.active()) {
            (1, choice.generation)
        } else if let Some(number) = self.number.as_deref().filter(|number| number.active()) {
            (2, number.generation)
        } else {
            (0, 0)
        }
    }

    pub(in crate::dialogue) fn active(&self) -> bool {
        self.key().0 != 0
    }

    pub(super) fn face<'a>(&self, dialogue: &'a Dialogue) -> Option<(&'a str, u32)> {
        if dialogue.active {
            dialogue
                .boxes
                .get(dialogue.index)
                .and_then(|page| page.face.as_deref().map(|name| (name, page.face_index)))
        } else if self.active() {
            dialogue.face.graphic()
        } else {
            None
        }
    }

    pub(super) fn runs(&self, face: bool) -> Vec<Run> {
        let left = if face { 84 } else { 12 };
        if let Some(choice) = self.choice.as_deref().filter(|choice| choice.active()) {
            choice
                .options
                .iter()
                .take(4)
                .enumerate()
                .map(|(row, label)| {
                    let color = if choice.disabled.contains(&row) {
                        DISABLED
                    } else {
                        DEFAULT
                    };
                    Run::new(label, left, 2 + row as i32 * 16, color)
                })
                .collect()
        } else if let Some(number) = self.number.as_deref().filter(|number| number.active()) {
            number
                .slots()
                .iter()
                .enumerate()
                .map(|(digit, value)| {
                    Run::new(value.to_string(), left + digit as i32 * 12, 2, DEFAULT)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub(super) fn embedded_runs(&self, dialogue: &Dialogue, face: bool) -> Vec<Run> {
        let prompt = dialogue.embedded_prompt().unwrap();
        let left = if face { 72 } else { 0 };
        let mut runs = dialogue
            .reveal
            .as_ref()
            .map_or("", |reveal| reveal.text())
            .split('\n')
            .enumerate()
            .filter(|(_, text)| !text.is_empty())
            .map(|(row, text)| {
                let indent = i32::from(!prompt.number() && row >= prompt.line) * 12;
                let color = if row >= prompt.line && prompt.disabled.contains(&(row - prompt.line))
                {
                    DISABLED
                } else {
                    DEFAULT
                };
                Run::new(text, left + indent, 2 + row as i32 * 16, color)
            })
            .collect::<Vec<_>>();
        if prompt.number() && self.active() {
            runs.extend(self.runs(face).into_iter().map(|mut run| {
                run.position.y += prompt.line as i32 * 16;
                run
            }));
        }
        runs
    }

    fn cursor(&self, face: bool) -> Option<(u32, u32, u32, bool)> {
        let left = if face { 72 } else { 0 };
        if let Some(choice) = self.choice.as_deref().filter(|choice| choice.active()) {
            (!choice.options.is_empty()).then_some((
                left + 10,
                8 + choice.cursor.min(3) as u32 * 16,
                300 - left,
                false,
            ))
        } else {
            self.number
                .as_deref()
                .filter(|number| number.active())
                .map(|number| (left + 16 + number.cursor() as u32 * 12, 8, 14, true))
        }
    }
}

#[derive(Component)]
pub(in crate::dialogue) struct Cursor;

pub(super) fn spawn(panel: &mut ChildSpawnerCommands, skin: &Handle<Image>) {
    panel
        .spawn((Node::default(), Cursor, Visibility::Hidden))
        .with_children(|cursor| crate::windowskin::cursor(cursor, skin));
}

pub(in crate::dialogue) fn render_cursor(
    dialogue: Res<Dialogue>,
    prompts: Presentation,
    clock: Res<Clock>,
    battle: Option<Res<crate::battle::BattleActive>>,
    mut cursors: Query<(&mut Node, &mut Visibility, &Children), With<Cursor>>,
    mut images: Query<&mut ImageNode>,
) {
    let cursor = prompts.cursor(prompts.face(&dialogue).is_some());
    let bank = usize::from(battle.is_some_and(|battle| battle.0));
    for (mut node, mut visibility, children) in &mut cursors {
        *visibility = visible_if(cursor.is_some());
        let Some((left, mut top, width, number)) = cursor else {
            continue;
        };
        if let Some(prompt) = dialogue.embedded_prompt() {
            top += prompt.line as u32 * 16;
        }
        *node = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left as f32 * 3.0),
            top: Val::Px(top as f32 * 3.0),
            width: Val::Px(width as f32 * 3.0),
            height: Val::Px(48.0),
            ..default()
        };
        for child in children {
            if let Ok(mut image) = images.get_mut(*child) {
                crate::windowskin::cursor_phase(&mut image, clock.source_x(bank, number));
            }
        }
    }
}

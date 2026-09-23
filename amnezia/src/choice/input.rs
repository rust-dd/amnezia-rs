use super::Choice;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::dialogue::MessagePause;
use crate::menu::DirectionInput;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(super) struct Gates<'w> {
    pause: MessagePause<'w>,
    dialogue: Option<Res<'w, crate::dialogue::Dialogue>>,
}

pub(super) fn update(
    keys: Res<ButtonInput<KeyCode>>,
    directions: Res<DirectionInput>,
    mut choice: ResMut<Choice>,
    gates: Gates,
    mut audio: MessageWriter<AudioRequest>,
    sounds: Option<Res<SystemSounds>>,
) {
    let count = choice.options.len().min(4);
    if !choice.active
        || count == 0
        || gates.pause.paused()
        || gates
            .dialogue
            .is_some_and(|dialogue| !dialogue.prompt_input_ready())
    {
        return;
    }
    if choice.cursor >= count {
        choice.cursor = count - 1;
    }
    for step in directions.slot_steps() {
        for (action, pressed) in [step[0], step[1], step[4], step[5]].into_iter().enumerate() {
            if !pressed {
                continue;
            }
            let next = match action {
                0 => (choice.cursor + 1) % count,
                1 => (choice.cursor + count - 1) % count,
                2 => (choice.cursor + 4).min(count - 1),
                _ => choice.cursor.saturating_sub(4),
            };
            if action < 2 || next != choice.cursor {
                choice.cursor = next;
                if let Some(sounds) = sounds.as_deref() {
                    play_system_se(&mut audio, &sounds.cursor);
                }
            }
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        if choice.cancel_type > 0 {
            if let Some(sounds) = sounds.as_deref() {
                play_system_se(&mut audio, &sounds.cancel);
            }
            choice.result = Some(choice.cancel_type - 1);
            choice.active = false;
        }
        return;
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        if choice.disabled.contains(&choice.cursor) {
            if let Some(sounds) = sounds.as_deref() {
                play_system_se(&mut audio, &sounds.buzzer);
            }
            return;
        }
        if let Some(sounds) = sounds.as_deref() {
            play_system_se(&mut audio, &sounds.decision);
        }
        choice.result = Some(choice.cursor as i32);
        choice.active = false;
    }
}

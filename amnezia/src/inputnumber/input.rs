use super::InputNumber;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::menu::{DirectionInput, MenuOpen};
use crate::teleport::Fade;
use crate::transitions::TransitionPause;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(super) struct Gates<'w> {
    pause: TransitionPause<'w>,
    menu: Option<Res<'w, MenuOpen>>,
    fade: Option<Res<'w, Fade>>,
}

pub(super) fn update(
    keys: Res<ButtonInput<KeyCode>>,
    directions: Res<DirectionInput>,
    mut input: ResMut<InputNumber>,
    gates: Gates,
    mut audio: MessageWriter<AudioRequest>,
    sounds: Option<Res<SystemSounds>>,
) {
    if !input.active
        || gates.pause.paused()
        || gates.menu.is_some_and(|v| v.0)
        || gates.fade.is_some_and(|v| v.busy())
    {
        return;
    }
    for step in directions.steps() {
        let moves = navigate(&mut input, step);
        if let Some(sounds) = sounds.as_deref() {
            for _ in 0..moves {
                play_system_se(&mut audio, &sounds.cursor);
            }
        }
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        if let Some(sounds) = sounds.as_deref() {
            play_system_se(&mut audio, &sounds.decision);
        }
        input.result = Some(input.value);
        input.active = false;
    }
}

fn navigate(input: &mut InputNumber, [down, up, right, left]: [bool; 4]) -> u32 {
    let count = input.slots.len();
    if count == 0 {
        return 0;
    }
    let mut moves = u32::from(up || down);
    if up {
        input.adjust(1);
    }
    if down {
        input.adjust(-1);
    }
    if right && count >= 2 {
        input.cursor = (input.cursor + 1) % count;
        moves += 1;
    }
    if left {
        input.cursor = (input.cursor + count - 1) % count;
        moves += 1;
    }
    moves
}

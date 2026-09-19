use super::{Mode, SaveFiles};
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::menu::MenuOpen;
use crate::save::{preview, slots::ActiveSlot};
use crate::timing::GameFrames;
use bevy::prelude::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    keys: Res<ButtonInput<KeyCode>>,
    frames: Res<GameFrames>,
    pause: crate::transitions::TransitionPause,
    open: Res<MenuOpen>,
    mut files: ResMut<SaveFiles>,
    mut slot: ResMut<ActiveSlot>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if files.suspended {
        return;
    }
    if !open.0 && !files.active() {
        *files = default();
        return;
    }
    if files.entries.is_none() || files.finished.is_some() {
        return;
    }
    let elapsed = frames.frame.wrapping_sub(files.last_frame);
    files.last_frame = frames.frame;
    if pause.paused() || (files.mode == Mode::Save && files.stage != super::scene::Stage::Ready) {
        return;
    }
    let selected = files.navigation.index;
    let choice = decision(&files, &keys, sounds.as_deref(), &mut audio);
    let mut moved = false;
    for tick in 0..elapsed.max(1) {
        moved |= files.navigation.tick(&keys, tick == 0, elapsed != 0);
    }
    if moved && let Some(sounds) = sounds {
        play_system_se(&mut audio, &sounds.cursor);
    }
    if let Some(confirmed) = choice {
        if confirmed {
            *slot = ActiveSlot::new(selected as u8 + 1).unwrap();
        }
        files.finished = Some(confirmed);
    }
}

fn decision(
    files: &SaveFiles,
    keys: &ButtonInput<KeyCode>,
    sounds: Option<&SystemSounds>,
    audio: &mut MessageWriter<AudioRequest>,
) -> Option<bool> {
    if files.navigation.moving() {
        return None;
    }
    let (choice, sound) = if keys.just_pressed(KeyCode::Escape) {
        (Some(false), sounds.map(|sounds| &sounds.cancel))
    } else if crate::menu::nav::confirm_pressed(keys) {
        if files.mode == Mode::Load
            && !matches!(
                files.entries.as_ref().unwrap()[files.navigation.index].contents,
                preview::Contents::Party(_)
            )
        {
            (None, sounds.map(|sounds| &sounds.buzzer))
        } else {
            (Some(true), sounds.map(|sounds| &sounds.decision))
        }
    } else {
        return None;
    };
    if let Some(sound) = sound {
        play_system_se(audio, sound);
    }
    choice
}

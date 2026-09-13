use super::{SaveFiles, navigation::Navigation};
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::menu::MenuOpen;
use crate::save::{SaveLocation, SaveRequest, preview, slots::ActiveSlot};
use crate::timing::GameFrames;
use bevy::prelude::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    keys: Res<ButtonInput<KeyCode>>,
    frames: Res<GameFrames>,
    open: Res<MenuOpen>,
    data: Res<GameData>,
    location: Res<SaveLocation>,
    mut files: ResMut<SaveFiles>,
    mut slot: ResMut<ActiveSlot>,
    mut request: ResMut<SaveRequest>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !open.0 {
        *files = default();
        return;
    }
    if files.requested {
        let entries = preview::catalog(&location.0, &data);
        files.navigation = Navigation::new(preview::latest(&entries));
        files.entries = Some(entries);
        files.requested = false;
        files.last_frame = frames.frame;
        return;
    }
    if files.entries.is_none() {
        return;
    }
    let elapsed = frames.frame.wrapping_sub(files.last_frame);
    files.last_frame = frames.frame;
    if !files.navigation.moving() {
        if keys.just_pressed(KeyCode::Escape) {
            if let Some(sounds) = sounds {
                play_system_se(&mut audio, &sounds.cancel);
            }
            *files = default();
            return;
        }
        if crate::menu::nav::confirm_pressed(&keys) {
            if let Some(sounds) = sounds {
                play_system_se(&mut audio, &sounds.decision);
            }
            *slot = ActiveSlot::new(files.navigation.index as u8 + 1).unwrap();
            request.0 = true;
            *files = default();
            return;
        }
    }
    let mut moved = false;
    for tick in 0..elapsed.max(1) {
        moved |= files.navigation.tick(&keys, tick == 0, elapsed != 0);
    }
    if moved && let Some(sounds) = sounds {
        play_system_se(&mut audio, &sounds.cursor);
    }
}

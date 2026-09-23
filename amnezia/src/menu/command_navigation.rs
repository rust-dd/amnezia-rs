use super::{
    MenuOpen, MenuScreen, MenuState, command, list_navigation::Input, save_files::SaveFiles,
};
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::state::Party;
use bevy::prelude::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    input: Res<Input>,
    open: Res<MenuOpen>,
    mut state: ResMut<MenuState>,
    party: Res<Party>,
    files: Res<SaveFiles>,
    pause: crate::menu::scene::Pause,
    fade: Res<crate::teleport::Fade>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !open.0 || files.active() || pause.paused() || fade.busy() {
        return;
    }
    let state = &mut *state;
    let (cursor, count, page) = match &mut state.screen {
        MenuScreen::Command => (&mut state.cursor, command::COMMANDS.len(), 5),
        MenuScreen::MemberSelect { cursor, .. } => (cursor, party.snapshot().len(), 14),
        MenuScreen::EndGame { cursor } => (cursor, 2, 2),
        _ => return,
    };
    if count == 0 {
        return;
    }
    *cursor = (*cursor).min(count - 1);
    for repeated in input.slot_steps() {
        for (action, pressed) in repeated.into_iter().enumerate() {
            if !pressed {
                continue;
            }
            *cursor = match action {
                0 => (*cursor + 1) % count,
                1 => (*cursor + count - 1) % count,
                4 if *cursor < count - 1 => cursor.saturating_add(page).min(count - 1),
                5 if *cursor > 0 => cursor.saturating_sub(page),
                _ => continue,
            };
            if let Some(sounds) = &sounds {
                play_system_se(&mut audio, &sounds.cursor);
            }
        }
    }
}

pub(crate) mod smoke;

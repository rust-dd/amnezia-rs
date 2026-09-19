use super::{MenuOpen, MenuScreen, MenuState, save_files::SaveFiles};
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::state::Party;
use bevy::prelude::*;

pub(crate) mod smoke;

const KEYS: [KeyCode; 4] = [
    KeyCode::ArrowDown,
    KeyCode::ArrowUp,
    KeyCode::PageDown,
    KeyCode::PageUp,
];

#[derive(Resource, Default)]
pub(super) struct Navigation {
    last_frame: Option<u32>,
    held: [u32; 4],
}

impl Navigation {
    fn elapsed(&mut self, now: u32) -> u32 {
        let elapsed = self
            .last_frame
            .replace(now)
            .map_or(0, |last| now.wrapping_sub(last));
        if elapsed > i32::MAX as u32 {
            self.held = [0; 4];
            0
        } else {
            elapsed
        }
    }

    fn sample(&mut self, keys: &ButtonInput<KeyCode>, elapsed: u32, triggered: bool) -> [bool; 4] {
        std::array::from_fn(|index| {
            let fresh = triggered && keys.just_pressed(KEYS[index]);
            if !keys.pressed(KEYS[index]) || fresh {
                self.held[index] = 0;
            }
            if keys.pressed(KEYS[index]) {
                self.held[index] = self.held[index].saturating_add(elapsed);
            }
            fresh || (elapsed > 0 && self.held[index] >= 24 && self.held[index].is_multiple_of(4))
        })
    }
}

fn navigate(cursor: &mut usize, count: usize, repeated: [bool; 4]) -> u32 {
    if count == 0 {
        return 0;
    }
    *cursor = (*cursor).min(count - 1);
    let mut sounds = 0;
    for (action, repeated) in repeated.into_iter().enumerate() {
        if !repeated {
            continue;
        }
        *cursor = match action {
            0 => (*cursor + 1) % count,
            1 => (*cursor + count - 1) % count,
            2 if *cursor < count - 1 => (*cursor + 14).min(count - 1),
            3 if *cursor > 0 => cursor.saturating_sub(14),
            _ => continue,
        };
        sounds += 1;
    }
    sounds
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    frames: Res<crate::timing::GameFrames>,
    keys: Res<ButtonInput<KeyCode>>,
    open: Res<MenuOpen>,
    mut state: ResMut<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    files: Res<SaveFiles>,
    pause: crate::transitions::TransitionPause,
    fade: Res<crate::teleport::Fade>,
    mut navigation: ResMut<Navigation>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    let elapsed = navigation.elapsed(frames.frame);
    let movable = match state.screen {
        MenuScreen::ItemTarget { item_id, .. } => {
            data.item(item_id).is_some_and(|item| item.scope == 0)
        }
        MenuScreen::SkillTarget { skill_id, .. } => data
            .skills
            .iter()
            .any(|skill| skill.id == skill_id && skill.scope == 3),
        _ => false,
    };
    if !open.0 || !movable || files.active() || pause.paused() || fade.busy() {
        navigation.sample(&keys, elapsed, true);
        return;
    }
    let cursor = match &mut state.screen {
        MenuScreen::ItemTarget { cursor, .. } | MenuScreen::SkillTarget { cursor, .. } => cursor,
        _ => return,
    };
    let count = party.snapshot().len();
    for tick in 0..elapsed.max(1) {
        let repeated = navigation.sample(&keys, u32::from(elapsed > 0), tick == 0);
        let moves = navigate(cursor, count, repeated);
        if let Some(sounds) = &sounds {
            for _ in 0..moves {
                play_system_se(&mut audio, &sounds.cursor);
            }
        }
    }
}

#[cfg(test)]
mod tests;

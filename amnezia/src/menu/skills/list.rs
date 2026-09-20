use super::known_skills;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState, list_navigation::Navigation, save_files::SaveFiles,
};
use crate::progression::Progression;
use crate::state::Party;
use crate::timing::GameFrames;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(in crate::menu) struct List {
    pub navigation: Navigation<10>,
    last_frame: Option<u32>,
    member: Option<usize>,
    visible: bool,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    frames: Res<GameFrames>,
    keys: Res<ButtonInput<KeyCode>>,
    open: Res<MenuOpen>,
    mut state: ResMut<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    files: Res<SaveFiles>,
    pause: crate::transitions::TransitionPause,
    fade: Res<crate::teleport::Fade>,
    mut list: ResMut<List>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    let elapsed = list
        .last_frame
        .replace(frames.frame)
        .map_or(0, |last| frames.frame.wrapping_sub(last));
    if !open.0
        || elapsed > i32::MAX as u32
        || !matches!(
            state.screen,
            MenuScreen::SkillList { .. } | MenuScreen::SkillTarget { .. }
        )
    {
        *list = List {
            last_frame: Some(frames.frame),
            ..default()
        };
        return;
    }
    let MenuScreen::SkillList { member, cursor } = state.screen else {
        list.visible = false;
        list.navigation.suspend();
        return;
    };
    let count = known_skills(member, &data, &party, &progression).len();
    if list.member != Some(member) {
        list.navigation = Navigation::new(cursor, count);
        list.member = Some(member);
    } else if !list.visible
        || list.navigation.index != cursor
        || list.navigation.count() != count.max(1)
    {
        list.navigation.refresh(cursor, count);
    }
    list.visible = true;
    if pause.paused() || fade.busy() || files.active() {
        list.navigation.suspend();
        return;
    }
    for tick in 0..elapsed.max(1) {
        let moves = list.navigation.tick(&keys, tick == 0, elapsed > 0);
        if let Some(sounds) = &sounds {
            for _ in 0..moves {
                play_system_se(&mut audio, &sounds.cursor);
            }
        }
    }
    state.screen = MenuScreen::SkillList {
        member,
        cursor: list.navigation.index,
    };
}

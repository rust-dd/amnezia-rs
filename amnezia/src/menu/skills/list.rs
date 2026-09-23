use super::known_skills;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState,
    list_navigation::{Input, Navigation},
    save_files::SaveFiles,
};
use crate::progression::Progression;
use crate::state::Party;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(in crate::menu) struct List {
    pub navigation: Navigation<10>,
    member: Option<usize>,
    visible: bool,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    input: Res<Input>,
    open: Res<MenuOpen>,
    mut state: ResMut<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    files: Res<SaveFiles>,
    pause: crate::menu::scene::Pause,
    fade: Res<crate::teleport::Fade>,
    mut list: ResMut<List>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !open.0
        || input.rewound
        || !matches!(
            state.screen,
            MenuScreen::SkillList { .. } | MenuScreen::SkillTarget { .. }
        )
    {
        *list = List::default();
        return;
    }
    let MenuScreen::SkillList { member, cursor } = state.screen else {
        list.visible = false;
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
        return;
    }
    for repeated in input.steps() {
        let moves = list.navigation.tick(repeated, input.timed());
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

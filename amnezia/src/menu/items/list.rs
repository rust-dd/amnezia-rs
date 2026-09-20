use super::selectable;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState,
    list_navigation::{Input, Navigation},
    save_files::SaveFiles,
};
use crate::state::Inventory;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(in crate::menu) struct List {
    pub navigation: Navigation<12>,
    initialized: bool,
    visible: bool,
}

impl List {
    pub(in crate::menu) fn return_to_list(
        &mut self,
        data: &GameData,
        inventory: &Inventory,
    ) -> MenuScreen {
        self.navigation
            .refresh(self.navigation.index, selectable(data, inventory));
        MenuScreen::ItemList {
            cursor: self.navigation.index,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    input: Res<Input>,
    open: Res<MenuOpen>,
    mut state: ResMut<MenuState>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
    files: Res<SaveFiles>,
    pause: crate::transitions::TransitionPause,
    fade: Res<crate::teleport::Fade>,
    mut list: ResMut<List>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !open.0
        || input.rewound
        || !matches!(
            state.screen,
            MenuScreen::ItemList { .. } | MenuScreen::ItemTarget { .. }
        )
    {
        *list = List::default();
        return;
    }
    let MenuScreen::ItemList { cursor } = state.screen else {
        list.visible = false;
        return;
    };
    let count = selectable(&data, &inventory);
    if !list.initialized {
        list.navigation = Navigation::new(cursor, count);
        list.initialized = true;
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
    state.screen = MenuScreen::ItemList {
        cursor: list.navigation.index,
    };
}

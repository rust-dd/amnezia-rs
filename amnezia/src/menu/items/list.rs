use super::{navigation::Navigation, selectable};
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::menu::{MenuOpen, MenuScreen, MenuState, save_files::SaveFiles};
use crate::state::Inventory;
use crate::timing::GameFrames;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(in crate::menu) struct List {
    pub navigation: Navigation,
    last_frame: Option<u32>,
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
    frames: Res<GameFrames>,
    keys: Res<ButtonInput<KeyCode>>,
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
    let elapsed = list
        .last_frame
        .replace(frames.frame)
        .map_or(0, |last| frames.frame.wrapping_sub(last));
    if !open.0
        || !matches!(
            state.screen,
            MenuScreen::ItemList { .. } | MenuScreen::ItemTarget { .. }
        )
    {
        *list = List {
            last_frame: Some(frames.frame),
            ..default()
        };
        return;
    }
    let MenuScreen::ItemList { cursor } = state.screen else {
        list.visible = false;
        list.navigation.suspend();
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
    state.screen = MenuScreen::ItemList {
        cursor: list.navigation.index,
    };
}

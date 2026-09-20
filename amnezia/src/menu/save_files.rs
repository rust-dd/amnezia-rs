use crate::gamedata::GameData;
use crate::save::{
    SaveLocation,
    preview::{self, Entry},
    slots::ActiveSlot,
};
use bevy::prelude::*;

mod events;
mod input;
mod navigation;
mod scene;
pub(crate) mod smoke;
#[cfg(test)]
mod tests;
mod view;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Save,
    Load,
}

#[derive(Resource, Default)]
pub(crate) struct SaveFiles {
    mode: Mode,
    suspended: bool,
    requested: bool,
    entries: Option<Vec<Entry>>,
    navigation: navigation::Navigation,
    last_frame: u32,
    event_menu: Option<bool>,
    finished: Option<bool>,
    stage: scene::Stage,
}

impl SaveFiles {
    pub(crate) fn waiting_for_scene(&self) -> bool {
        self.active()
            && self.mode == Mode::Save
            && !matches!(self.stage, scene::Stage::Inactive | scene::Stage::Ready)
    }

    pub(super) fn request(&mut self) {
        self.requested = true;
    }

    pub(crate) fn active(&self) -> bool {
        !self.suspended
            && (self.requested || self.entries.is_some() || self.stage != scene::Stage::Inactive)
    }

    pub(crate) fn blocks_io(&self) -> bool {
        self.active() && self.stage != scene::Stage::Writing
    }

    pub(crate) fn open_load(&mut self, location: &SaveLocation, data: &GameData, frame: u32) {
        *self = Self {
            mode: Mode::Load,
            ..default()
        };
        self.prepare(location, data, frame);
    }

    fn prepare(&mut self, location: &SaveLocation, data: &GameData, frame: u32) {
        let entries = preview::catalog(&location.0, data);
        self.navigation = navigation::Navigation::new(preview::latest(&entries));
        self.entries = Some(entries);
        self.requested = false;
        self.last_frame = frame;
    }

    pub(crate) fn decision(&self) -> Option<bool> {
        self.finished
    }

    pub(crate) fn suspend(&mut self) {
        self.suspended = true;
        self.finished = None;
    }

    pub(crate) fn reject_load(&mut self, frame: u32, slot: ActiveSlot) {
        self.suspended = false;
        self.last_frame = frame;
        if let Some(entries) = &mut self.entries {
            entries[slot.index()].contents = preview::Contents::Corrupt;
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_flow(app);
    app.add_systems(Startup, view::spawn).add_systems(
        Update,
        view::update.after(FileInput).after(crate::title::TitleFlow),
    );
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FileInput;

pub(crate) fn register_flow(app: &mut App) {
    app.init_resource::<SaveFiles>()
        .init_resource::<crate::transitions::Transition>()
        .add_systems(
            PreUpdate,
            (events::update, scene::authorize_write)
                .chain()
                .before(crate::save::SaveSet),
        )
        .add_systems(
            Update,
            (input::update, scene::update)
                .chain()
                .in_set(FileInput)
                .after(super::MenuInput)
                .before(super::view::clocks::update),
        );
}

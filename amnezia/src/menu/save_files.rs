use crate::save::preview::Entry;
use bevy::prelude::*;

mod events;
mod input;
mod navigation;
pub(crate) mod smoke;
#[cfg(test)]
mod tests;
mod view;

#[derive(Resource, Default)]
pub(crate) struct SaveFiles {
    requested: bool,
    entries: Option<Vec<Entry>>,
    navigation: navigation::Navigation,
    last_frame: u32,
    event_menu: Option<bool>,
    finished: Option<bool>,
}

impl SaveFiles {
    pub(super) fn request(&mut self) {
        self.requested = true;
    }

    pub(crate) fn active(&self) -> bool {
        self.requested || self.entries.is_some()
    }
}

pub(super) fn register(app: &mut App) {
    register_flow(app);
    app.add_systems(Startup, view::spawn)
        .add_systems(Update, view::update.after(FileInput));
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FileInput;

pub(crate) fn register_flow(app: &mut App) {
    app.init_resource::<SaveFiles>()
        .add_systems(PreUpdate, events::update.before(crate::save::SaveSet))
        .add_systems(
            Update,
            input::update
                .in_set(FileInput)
                .after(super::MenuInput)
                .before(super::view::clocks::update),
        );
}

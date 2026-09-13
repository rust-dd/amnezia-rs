use crate::save::preview::Entry;
use bevy::prelude::*;

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
    app.init_resource::<SaveFiles>()
        .add_systems(Startup, view::spawn)
        .add_systems(
            Update,
            (input::update, view::update)
                .chain()
                .after(super::MenuInput)
                .before(super::view::clocks::update),
        );
}

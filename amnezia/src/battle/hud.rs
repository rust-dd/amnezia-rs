//! RM2000 battle windows, using native geometry and the original windowskin.

mod arrows;
mod content;
mod cursor_smoke;
mod layout;
mod motion;
mod navigation;
mod pixels;
mod status;
mod view;

pub(crate) use arrows::snapshot as arrow_snapshot;
pub(super) use arrows::verify as verify_arrows;
pub(crate) use cursor_smoke::{snapshot as cursor_snapshot, verify_finished as verify_cursors};
pub(super) use motion::ready as commands_ready;
pub(super) use motion::smoke::label as movement_label;
pub(crate) use motion::smoke::snapshot as movement_snapshot;
pub(super) use navigation::smoke as navigation_smoke;
pub(super) use view::verify_bounds;
pub(super) use view::verify_layers;

use super::model::{Battle, MenuLevel, Phase};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Option,
    Help,
    Item,
    Skill,
    Status,
    Command,
    Message,
    Target,
}

impl Panel {
    const ALL: [Self; 8] = [
        Self::Option,
        Self::Help,
        Self::Item,
        Self::Skill,
        Self::Status,
        Self::Command,
        Self::Message,
        Self::Target,
    ];

    fn columns(self) -> usize {
        if matches!(self, Self::Skill | Self::Item) {
            2
        } else {
            1
        }
    }

    fn size(self) -> UVec2 {
        let (width, height) = match self {
            Self::Option | Self::Command => (76, 80),
            Self::Help => (320, 32),
            Self::Status => (244, 80),
            Self::Target => (136, 80),
            Self::Item | Self::Skill | Self::Message => (320, 80),
        };
        UVec2::new(width, height)
    }

    fn active(battle: &Battle) -> Option<Self> {
        match battle.phase {
            Phase::PartyCommand => Some(Self::Option),
            Phase::Command => Some(match battle.menu {
                MenuLevel::Command => Self::Command,
                MenuLevel::Skill => Self::Skill,
                MenuLevel::Item => Self::Item,
                MenuLevel::Target => Self::Target,
                MenuLevel::AllyTarget => Self::Status,
            }),
            _ => None,
        }
    }

    fn cursor(self, battle: &Battle) -> usize {
        if Self::active(battle) == Some(self) {
            return battle.cursor;
        }
        match self {
            Self::Command => battle.menu_cursors[MenuLevel::Command as usize],
            Self::Skill => battle.menu_cursors[MenuLevel::Skill as usize],
            Self::Item => battle.menu_cursors[MenuLevel::Item as usize],
            Self::Status => battle.turn,
            _ => 0,
        }
    }
}

pub fn register(app: &mut App) {
    motion::register(app);
    app.init_resource::<navigation::Windows>()
        .add_systems(
            Update,
            navigation::tick
                .after(crate::menu::MenuInput)
                .after(motion::tick)
                .after(super::message::tick)
                .before(super::input::command_input),
        )
        .add_systems(Startup, view::spawn.after(crate::world::setup_cameras))
        .add_systems(
            Update,
            (
                motion::observe,
                navigation::observe,
                view::panels,
                view::rows,
                view::cursors,
                arrows::update,
            )
                .chain()
                .after(super::input::command_input)
                .after(super::systems::outcome_input),
        );
}

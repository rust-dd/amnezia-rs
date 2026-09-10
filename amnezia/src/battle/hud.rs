//! RM2000 battle windows, using native geometry and the original windowskin.

mod arrows;
mod clocks;
mod content;
mod layout;
mod motion;
mod status;
mod view;

pub(crate) use arrows::snapshot as arrow_snapshot;
pub(super) use arrows::verify as verify_arrows;
pub(super) use motion::ready as commands_ready;
pub(super) use motion::smoke::label as movement_label;
pub(crate) use motion::smoke::snapshot as movement_snapshot;
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

#[derive(Resource, Default)]
struct ListScroll {
    generation: u64,
    first: [usize; 8],
}

fn first_visible(first: usize, cursor: usize, columns: usize) -> usize {
    if cursor < first {
        cursor / columns * columns
    } else if cursor >= first + 4 * columns {
        (cursor / columns).saturating_sub(3) * columns
    } else {
        first
    }
}

fn scroll(battle: Res<Battle>, mut scroll: ResMut<ListScroll>) {
    if !battle.is_changed() {
        return;
    }
    if scroll.generation != battle.generation {
        *scroll = ListScroll {
            generation: battle.generation,
            ..default()
        };
    }
    if let Some(panel) = Panel::active(&battle) {
        let first = &mut scroll.first[panel as usize];
        *first = first_visible(*first, battle.cursor, panel.columns());
    }
}

pub fn register(app: &mut App) {
    motion::register(app);
    app.init_resource::<ListScroll>()
        .init_resource::<clocks::WindowClocks>()
        .add_systems(Startup, view::spawn.after(super::systems::spawn_hud_camera))
        .add_systems(
            Update,
            (
                motion::observe,
                scroll,
                clocks::tick,
                view::panels,
                view::rows,
                view::cursors,
                arrows::update,
            )
                .chain()
                .after(super::input::command_input),
        );
}

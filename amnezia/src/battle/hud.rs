//! RM2000 battle windows, using native geometry and the original windowskin.

mod arrows;
mod clocks;
mod content;
mod layout;
mod status;
mod view;

pub(crate) use arrows::snapshot as arrow_snapshot;
pub(super) use arrows::verify as verify_arrows;
pub(super) use view::verify_bounds;

use super::model::{Battle, MenuLevel, Phase};
use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Option,
    Command,
    Status,
    Message,
    Help,
}

impl Panel {
    const ALL: [Self; 5] = [
        Self::Option,
        Self::Command,
        Self::Status,
        Self::Message,
        Self::Help,
    ];
}

fn list_columns(battle: &Battle) -> usize {
    if matches!(battle.menu, MenuLevel::Skill | MenuLevel::Item) {
        2
    } else {
        1
    }
}

#[derive(Resource, Default)]
struct ListScroll {
    context: Option<(u64, Phase, MenuLevel, usize)>,
    first: usize,
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
    let context = Some((battle.generation, battle.phase, battle.menu, battle.turn));
    if scroll.context != context {
        scroll.context = context;
        scroll.first = 0;
    }
    scroll.first = first_visible(scroll.first, battle.cursor, list_columns(&battle));
}

pub fn register(app: &mut App) {
    app.init_resource::<ListScroll>()
        .init_resource::<clocks::WindowClocks>()
        .add_systems(Startup, view::spawn.after(super::systems::spawn_hud_camera))
        .add_systems(
            Update,
            (
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

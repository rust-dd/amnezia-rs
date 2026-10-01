//! RM2000 main-menu scenes. [`MenuOpen`] pauses the world; nested screens retain
//! their own cursors. Skills use runtime progression, and equipment uses the shared
//! [`crate::equipment::Equipment`] store.

mod command;
mod command_navigation;
mod derive;
mod equip;
mod input;
mod items;
mod list_navigation;
pub(crate) mod name_smoke;
mod nav;
mod render;
pub(crate) mod save_files;
mod scene;
mod skills;
mod status;
mod targets;
mod use_item;
mod view;

pub(crate) use command_navigation::smoke as navigation_smoke;
pub(crate) use equip::Switch as EquipmentSwitch;
pub(crate) use equip::layout_smoke as equipment_smoke;
pub(crate) use input::map::Calling;
pub(crate) use items::smoke as item_smoke;
pub(crate) use list_navigation::Input as DirectionInput;
#[cfg(test)]
pub(crate) use list_navigation::update_input as update_directions;
pub(crate) use scene::Flow as SceneFlow;
pub(crate) use skills::smoke as skill_smoke;
pub(crate) use targets::smoke as target_navigation_smoke;
pub(crate) use view::end_game::smoke as end_smoke;
pub(crate) use view::smoke as layout_smoke;
pub(crate) use view::target::smoke as target_smoke;
pub(crate) use view::text_smoke as font_smoke;

#[cfg(test)]
mod name_tests;
#[cfg(test)]
mod testkit;

use bevy::prelude::*;

/// Whether the menu overlay is showing. The pause guard reads this; this module
/// owns the toggle (Escape).
#[derive(Resource, Default)]
pub struct MenuOpen(pub bool);

/// Menu-opening permission (opcode 11960); disabling it does not close or lock
/// an already-open menu, matching RPG_RT `SetAllowMenu`.
#[derive(Resource)]
pub struct MenuAccess(pub bool);

impl Default for MenuAccess {
    fn default() -> Self {
        Self(true)
    }
}

/// What a member-select prompt is choosing a party member *for*: to cast a skill,
/// to inspect equipment, or to view status detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberAction {
    Skill,
    Equip,
    Status,
}

/// The menu's active screen. The top-level `Command` list uses the cursor stored
/// in [`MenuState`]; every drilled-in screen carries its own cursor (and the
/// member/item/skill it is bound to), so backing out restores the command cursor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum MenuScreen {
    #[default]
    Command,
    ItemList {
        cursor: usize,
    },
    ItemTarget {
        item_id: u32,
        cursor: usize,
    },
    MemberSelect {
        action: MemberAction,
        cursor: usize,
    },
    SkillList {
        member: usize,
        cursor: usize,
    },
    SkillTarget {
        member: usize,
        skill_id: u32,
        cursor: usize,
    },
    Equip {
        member: usize,
        /// The highlighted equipment slot (0..5, in `ActorDef` slot order).
        slot: usize,
        /// `None` while choosing which slot to change; `Some(cursor)` while
        /// choosing the item to put in `slot` (the candidate-list cursor).
        picking: Option<usize>,
    },
    Status {
        member: usize,
    },
    EndGame {
        cursor: usize,
    },
}

#[derive(Resource, Default)]
struct MenuState {
    cursor: usize,
    screen: MenuScreen,
}

pub(crate) fn playtest_state(world: &World) -> String {
    let state = world.resource::<MenuState>();
    format!("{} {:?}", state.cursor, state.screen)
}

pub struct MenuPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MenuInput;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MapMenuRequest;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct MenuView;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        save_files::register(app);
        equip::register_switching(app);
        scene::register(app);
        crate::windowskin::background::register(app);
        register_input(app);
        app.init_resource::<view::clocks::Clock>()
            .init_resource::<view::end_game::Clock>()
            .init_resource::<view::target::Clock>()
            .add_systems(Startup, view::spawn_ui)
            .add_systems(
                Update,
                (
                    view::update_ui,
                    view::item_list::update,
                    view::skill_list::update,
                    view::equipment::update,
                    view::target::update,
                    view::clocks::update,
                    view::end_game::update,
                )
                    .in_set(MenuView)
                    .after(MenuInput),
            );
    }
}

fn register_input(app: &mut App) {
    input::map::register(app);
    app.init_resource::<MenuOpen>()
        .init_resource::<MenuAccess>()
        .init_resource::<MenuState>()
        .init_resource::<list_navigation::Input>()
        .init_resource::<items::List>()
        .init_resource::<skills::List>()
        .init_resource::<equip::Scene>()
        .init_resource::<targets::Navigation>()
        .add_systems(
            Update,
            (
                list_navigation::update_input,
                items::update,
                skills::update,
                equip::update,
                targets::update,
                command_navigation::update,
                input::menu_input,
                equip::refresh_actor,
            )
                .chain()
                .in_set(MenuInput),
        );
}

#[cfg(test)]
pub(crate) fn register_map_input(app: &mut App) {
    register_input(app);
    scene::register(app);
    app.init_resource::<save_files::SaveFiles>();
}

//! The in-game menu: a windowskin overlay toggled with Escape. It browses the
//! party, the held items, and the known skills across three tabs, and drills into
//! two interactive sub-screens: using a field item on a chosen party member, and
//! a read-only status view of a member. Left/right switch tab, up/down move the
//! row cursor, Enter/Space confirm, and Escape backs out of a sub-screen (or, from
//! the browse view, closes the menu). The movement/interpreter pause guard that
//! freezes the world while it is open (keyed on [`MenuOpen`]) is wired by the main
//! session; this module owns the toggle and the UI.

mod browse;
mod derive;
mod status;
mod use_item;
mod view;

#[cfg(test)]
mod testkit;

use crate::battle::BattleActive;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::save::SaveRequest;
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party};
use crate::title::TitleActive;
use crate::vitals::Vitals;
use bevy::prelude::*;
use view::{MenuPanel, MenuText};

/// Whether the menu overlay is showing. The pause guard reads this; this module
/// owns the toggle (Escape).
#[derive(Resource, Default)]
pub struct MenuOpen(pub bool);

/// Which screen the menu is on: the three-tab browse viewer, the item-target
/// picker (a held field item awaiting a recipient), or a member's status block.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum MenuScreen {
    #[default]
    Browse,
    ItemTarget {
        item_id: u32,
        cursor: usize,
    },
    Status {
        member: usize,
    },
}

/// Which tab is shown (`0` party, `1` items, `2` skills), where the row cursor
/// sits within it, and which sub-screen (if any) is open.
#[derive(Resource, Default)]
struct MenuState {
    tab: usize,
    cursor: usize,
    screen: MenuScreen,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuOpen>()
            .init_resource::<MenuState>()
            .add_systems(Startup, view::spawn_ui)
            .add_systems(Update, (menu_input, update_ui));
    }
}

/// Whether a confirm key (Space or Enter) was pressed this frame.
fn confirm_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
}

/// The `(open, screen)` after Escape: from a sub-screen, back to Browse with the
/// menu still open; from Browse, close the menu; from closed, open it on Browse.
fn escape_transition(open: bool, screen: MenuScreen) -> (bool, MenuScreen) {
    if open && screen != MenuScreen::Browse {
        (true, MenuScreen::Browse)
    } else {
        (!open, MenuScreen::Browse)
    }
}

/// The sub-screen a confirm on the browse `cursor` opens, or `None` when the
/// selection takes no action (empty row, a non-field-usable item, the skills tab).
fn browse_target(
    tab: usize,
    cursor: usize,
    data: &GameData,
    party: &Party,
    inventory: &Inventory,
) -> Option<MenuScreen> {
    match tab {
        0 => (cursor < party.snapshot().len()).then_some(MenuScreen::Status { member: cursor }),
        1 => {
            let &item_id = use_item::held_item_ids(data, inventory).get(cursor)?;
            let item = data.item(item_id)?;
            use_item::field_usable(item).then_some(MenuScreen::ItemTarget { item_id, cursor: 0 })
        }
        _ => None,
    }
}

/// Toggle the menu on Escape (backing out of a sub-screen first), and drive the
/// active screen: switch tab / move the cursor and confirm into a sub-screen on
/// Browse, move the recipient cursor and apply on ItemTarget.
#[allow(clippy::too_many_arguments)]
fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    mut inventory: ResMut<Inventory>,
    mut vitals: ResMut<Vitals>,
    shop: Res<ShopOpen>,
    battle: Res<BattleActive>,
    title: Res<TitleActive>,
    mut open: ResMut<MenuOpen>,
    mut state: ResMut<MenuState>,
    mut save_request: ResMut<SaveRequest>,
) {
    // A shop, battle, or the title screen owns the input while up, so the menu
    // can't open over it.
    if !open.0 && (shop.0 || battle.0 || title.0) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        let was_open = open.0;
        let (next_open, next_screen) = escape_transition(open.0, state.screen);
        open.0 = next_open;
        state.screen = next_screen;
        if next_open && !was_open {
            state.cursor = 0;
        }
        return;
    }
    if !open.0 {
        return;
    }
    // Save the game from the menu (the F5 path); the save module honours it.
    if keys.just_pressed(KeyCode::KeyS) {
        save_request.0 = true;
    }
    let confirm = confirm_pressed(&keys);
    match state.screen {
        MenuScreen::Browse => {
            if keys.just_pressed(KeyCode::ArrowRight) {
                state.tab = (state.tab + 1) % browse::TABS.len();
                state.cursor = 0;
            }
            if keys.just_pressed(KeyCode::ArrowLeft) {
                state.tab = (state.tab + browse::TABS.len() - 1) % browse::TABS.len();
                state.cursor = 0;
            }
            let max = browse::tab_rows(state.tab, &data, &party, &inventory)
                .len()
                .saturating_sub(1);
            if keys.just_pressed(KeyCode::ArrowDown) {
                state.cursor = (state.cursor + 1).min(max);
            }
            if keys.just_pressed(KeyCode::ArrowUp) {
                state.cursor = state.cursor.saturating_sub(1);
            }
            if confirm
                && let Some(screen) =
                    browse_target(state.tab, state.cursor, &data, &party, &inventory)
            {
                state.screen = screen;
            }
        }
        MenuScreen::ItemTarget { item_id, cursor } => {
            let max = party.snapshot().len().saturating_sub(1);
            if keys.just_pressed(KeyCode::ArrowDown) {
                let cursor = (cursor + 1).min(max);
                state.screen = MenuScreen::ItemTarget { item_id, cursor };
            }
            if keys.just_pressed(KeyCode::ArrowUp) {
                let cursor = cursor.saturating_sub(1);
                state.screen = MenuScreen::ItemTarget { item_id, cursor };
            }
            if confirm
                && use_item::apply_field_item(
                    item_id,
                    cursor,
                    &data,
                    &party,
                    &progression,
                    &mut inventory,
                    &mut vitals,
                )
            {
                state.screen = MenuScreen::Browse;
            }
        }
        MenuScreen::Status { .. } => {}
    }
}

/// Reflect the menu state into the panel: show or hide it, and recompose the
/// active screen whenever the state, the inventory, or the vitals change.
#[allow(clippy::too_many_arguments)]
fn update_ui(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    progression: Res<Progression>,
    vitals: Res<Vitals>,
    mut panels: Query<&mut Visibility, With<MenuPanel>>,
    mut texts: Query<&mut Text, With<MenuText>>,
) {
    if !open.is_changed() && !state.is_changed() && !inventory.is_changed() && !vitals.is_changed()
    {
        return;
    }
    if let Ok(mut visibility) = panels.single_mut() {
        *visibility = if open.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if open.0
        && let Ok(mut text) = texts.single_mut()
    {
        **text = match state.screen {
            MenuScreen::Browse => {
                let rows = browse::tab_rows(state.tab, &data, &party, &inventory);
                browse::compose(state.tab, state.cursor, &rows)
            }
            MenuScreen::ItemTarget { item_id, cursor } => {
                use_item::compose_target(item_id, cursor, &data, &party, &progression, &vitals)
            }
            MenuScreen::Status { member } => {
                status::compose_status(member, &data, &party, &progression, &vitals)
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit::{self, ITEM_HERB};

    #[test]
    fn escape_backs_out_of_a_subscreen_then_closes_the_menu() {
        let br = MenuScreen::Browse;
        let status = MenuScreen::Status { member: 0 };
        let target = MenuScreen::ItemTarget {
            item_id: 5,
            cursor: 1,
        };
        assert_eq!(escape_transition(false, br), (true, br)); // closed -> open
        assert_eq!(escape_transition(true, status), (true, br)); // sub -> browse
        assert_eq!(escape_transition(true, target), (true, br)); // sub -> browse
        assert_eq!(escape_transition(true, br), (false, br)); // browse -> closed
    }

    #[test]
    fn confirm_opens_status_from_party_and_item_target_from_a_usable_item() {
        let d = testkit::data();
        let party = Party::default();
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 1);
        let status0 = Some(MenuScreen::Status { member: 0 });
        let target = Some(MenuScreen::ItemTarget {
            item_id: ITEM_HERB,
            cursor: 0,
        });
        assert_eq!(browse_target(0, 0, &d, &party, &inv), status0);
        assert_eq!(browse_target(1, 0, &d, &party, &inv), target);
    }

    #[test]
    fn confirm_does_nothing_on_equipment_or_the_skills_tab() {
        let mut d = testkit::data();
        d.items = vec![testkit::weapon(5, "Kard", 4)]; // id 5 held, but equipment
        let (party, mut inv) = (Party::default(), Inventory::default());
        inv.add_item(5, 1);
        assert_eq!(browse_target(1, 0, &d, &party, &inv), None); // equipment
        assert_eq!(browse_target(2, 0, &d, &party, &inv), None); // skills tab
    }
}

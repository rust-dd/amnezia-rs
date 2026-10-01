//! Side-effect-free menu navigation, tested without a Bevy app.

use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::vitals::Vitals;
use bevy::input::ButtonInput;
use bevy::prelude::KeyCode;

use super::{MemberAction, MenuScreen, items, skills, use_item};

pub(super) fn confirm_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
}

/// The `(open, screen)` after Escape. From a sub-screen, back out to its parent
/// with the menu still open; from the command list, close the menu; from closed,
/// open it on the command list.
pub(super) fn escape_transition(open: bool, screen: MenuScreen) -> (bool, MenuScreen) {
    match screen {
        MenuScreen::Command => (!open, MenuScreen::Command),
        MenuScreen::ItemList { .. } => (open, MenuScreen::Command),
        MenuScreen::ItemTarget { .. } => (open, MenuScreen::ItemList { cursor: 0 }),
        MenuScreen::MemberSelect { .. } => (open, MenuScreen::Command),
        MenuScreen::SkillList { .. } => (open, MenuScreen::Command),
        MenuScreen::SkillTarget { member, .. } => {
            (open, MenuScreen::SkillList { member, cursor: 0 })
        }
        MenuScreen::Equip {
            member,
            slot,
            picking: Some(_),
        } => (
            open,
            MenuScreen::Equip {
                member,
                slot,
                picking: None,
            },
        ),
        MenuScreen::Equip { .. } => (open, MenuScreen::Command),
        MenuScreen::Status { member } => (
            open,
            MenuScreen::MemberSelect {
                action: MemberAction::Status,
                cursor: member,
            },
        ),
        MenuScreen::EndGame { .. } => (open, MenuScreen::Command),
    }
}

/// End Game returns `(menu_open, title_visible)`; cursor 0 confirms, all others cancel.
pub(super) fn end_game_transition(cursor: usize, title: bool) -> (bool, bool) {
    if cursor == 0 {
        (false, true)
    } else {
        (true, title)
    }
}

/// The screen a confirm on the item list opens: the member picker for a held,
/// field-usable item under `cursor`, or `None` for a spacer/gold row or a
/// non-usable item.
pub(super) fn item_target(
    cursor: usize,
    data: &GameData,
    inventory: &Inventory,
) -> Option<MenuScreen> {
    let item_id = items::item_at(cursor, data, inventory)?;
    let item = data.item(item_id)?;
    use_item::field_usable(item).then_some(MenuScreen::ItemTarget { item_id, cursor: 0 })
}

/// The screen a confirm on `member`'s skill list opens: the ally picker for a
/// field-usable skill under `cursor` (indexed into the member's known skills), or
/// `None` for an unknown, unavailable or unaffordable skill.
pub(super) fn skill_target(
    member: usize,
    cursor: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
    equipment: &Equipment,
) -> Option<MenuScreen> {
    let skill = skills::skill_at(member, cursor, data, party, progression)?;
    skills::can_use(member, skill, data, party, progression, vitals, equipment).then_some(
        MenuScreen::SkillTarget {
            member,
            skill_id: skill.id,
            cursor: 0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit::{self, ITEM_HERB};

    #[test]
    fn escape_backs_out_of_each_subscreen_then_closes_the_menu() {
        assert_eq!(
            escape_transition(false, MenuScreen::Command),
            (true, MenuScreen::Command)
        );
        assert_eq!(
            escape_transition(true, MenuScreen::Command),
            (false, MenuScreen::Command)
        );
        assert_eq!(
            escape_transition(true, MenuScreen::ItemList { cursor: 3 }),
            (true, MenuScreen::Command)
        );
        assert_eq!(
            escape_transition(
                true,
                MenuScreen::ItemTarget {
                    item_id: 5,
                    cursor: 1
                }
            ),
            (true, MenuScreen::ItemList { cursor: 0 })
        );
        assert_eq!(
            escape_transition(
                true,
                MenuScreen::SkillList {
                    member: 2,
                    cursor: 4
                }
            ),
            (true, MenuScreen::Command)
        );
        assert_eq!(
            escape_transition(
                true,
                MenuScreen::Equip {
                    member: 1,
                    slot: 2,
                    picking: None
                }
            ),
            (true, MenuScreen::Command)
        );
        assert_eq!(
            escape_transition(
                true,
                MenuScreen::Equip {
                    member: 1,
                    slot: 2,
                    picking: Some(3)
                }
            ),
            (
                true,
                MenuScreen::Equip {
                    member: 1,
                    slot: 2,
                    picking: None
                }
            )
        );
        assert_eq!(
            escape_transition(true, MenuScreen::Status { member: 0 }),
            (
                true,
                MenuScreen::MemberSelect {
                    action: MemberAction::Status,
                    cursor: 0
                }
            )
        );
    }

    #[test]
    fn end_game_igen_returns_to_title_and_nem_cancels() {
        assert_eq!(end_game_transition(0, false), (false, true));
        assert_eq!(end_game_transition(1, false), (true, false));
    }

    #[test]
    fn item_list_confirm_reaches_the_field_use_flow() {
        let d = testkit::data();
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 1);
        assert_eq!(
            item_target(0, &d, &inv),
            Some(MenuScreen::ItemTarget {
                item_id: ITEM_HERB,
                cursor: 0
            })
        );
        assert_eq!(item_target(1, &d, &inv), None);
    }

    #[test]
    fn item_list_confirm_ignores_equipment() {
        let mut d = testkit::data();
        d.items = vec![testkit::weapon(5, "Kard", 4)];
        let mut inv = Inventory::default();
        inv.add_item(5, 1);
        assert_eq!(item_target(0, &d, &inv), None);
    }

    #[test]
    fn skill_list_confirm_reaches_targets_only_for_field_skills() {
        let mut d = testkit::data();
        d.skills = vec![
            testkit::heal_skill(2, "Gyógyítás", 8, 40),
            testkit::skill(3, "Tűzgolyó", 12),
        ];
        d.actors[0].learnings = vec![
            amnezia_data::Learning {
                level: 1,
                skill_id: 2,
            },
            amnezia_data::Learning {
                level: 1,
                skill_id: 3,
            },
        ];
        let party = Party::default();
        let prog = Progression::default();
        assert_eq!(
            skill_target(
                0,
                0,
                &d,
                &party,
                &prog,
                &Vitals::default(),
                &Equipment::default()
            ),
            Some(MenuScreen::SkillTarget {
                member: 0,
                skill_id: 2,
                cursor: 0
            })
        );
        assert_eq!(
            skill_target(
                0,
                1,
                &d,
                &party,
                &prog,
                &Vitals::default(),
                &Equipment::default()
            ),
            None
        );
    }
}

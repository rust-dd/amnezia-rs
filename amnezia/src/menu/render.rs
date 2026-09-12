//! Party figures for the main menu and the remaining text-based subscreens.

use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::vitals::Vitals;

use super::{MenuScreen, derive, equip, items, skills, status, use_item};

/// The End Game confirmation rows, in cursor order (Igen = yes returns to title).
pub(super) const END_GAME_ROWS: [&str; 2] = ["Igen", "Nem"];

/// One party member's status-window figures: the FaceSet portrait, identity, and
/// the numbers the status window prints beside the face. HP/SP are kept as raw
/// current/maximum values so [`super::view`] can tint the low ones the RM2000 way.
pub(super) struct MemberView {
    pub face_name: String,
    pub face_index: u32,
    pub name: String,
    pub title: String,
    pub level: u32,
    pub condition: String,
    pub condition_color: Option<u32>,
    pub exp: Option<(u32, u32)>,
    pub hp: i32,
    pub max_hp: i32,
    pub sp: i32,
    pub max_sp: i32,
}

/// The party roster as status-window rows, one per roster slot (unknown ids fall
/// back to a `#id` placeholder so the slot index still lines up with the
/// member-select cursor). Every figure is derived exactly as the battle system
/// derives it (see [`super::derive`]).
pub(super) fn members(
    hero_name: &crate::text::HeroName,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
) -> Vec<MemberView> {
    party
        .snapshot()
        .iter()
        .map(|&id| match data.actor(id) {
            Some(def) => {
                let level = progression.level(def);
                let (max_hp, max_sp) = derive::max_hp_sp(def, level);
                let (hp, sp) = vitals.get_stored(id).unwrap_or((max_hp, max_sp));
                let total = progression.total(def);
                let exp = derive::exp_to_next(def, total, level)
                    .map(|remaining| (total, total + remaining));
                let active = vitals.states(id);
                let condition = crate::conditions::definitions()
                    .iter()
                    .filter(|state| active.contains(&state.id))
                    .max_by_key(|state| (state.id == 1, state.priority, state.id));
                MemberView {
                    face_name: def.face_name.clone(),
                    face_index: def.face_index,
                    name: i18n::tr(hero_name.actor(def)),
                    title: i18n::tr(&def.title),
                    level,
                    condition: condition.map_or_else(String::new, |state| i18n::tr(&state.name)),
                    condition_color: condition.map(|state| state.color),
                    exp,
                    hp,
                    max_hp,
                    sp,
                    max_sp,
                }
            }
            None => MemberView {
                face_name: String::new(),
                face_index: 0,
                name: format!("#{id}"),
                title: String::new(),
                level: 0,
                condition: String::new(),
                condition_color: None,
                exp: None,
                hp: 0,
                max_hp: 0,
                sp: 0,
                max_sp: 0,
            },
        })
        .collect()
}

/// A content screen's text plus — for the scrolling item and skill lists — the
/// composed-text line its windowskin cursor highlights. `None` leaves the content
/// cursor hidden (the read-only and confirmation screens).
pub(super) struct ContentView {
    pub text: String,
    pub cursor_line: Option<usize>,
}

/// Compose the content window for a non-main screen. The Command and MemberSelect
/// screens are drawn as the three-window main menu instead, so they return an
/// empty view here (the content window is hidden for them).
#[allow(clippy::too_many_arguments)]
pub(super) fn content(
    hero_name: &crate::text::HeroName,
    screen: MenuScreen,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    inventory: &Inventory,
    vitals: &Vitals,
    equipment: &Equipment,
    terms: &Terms,
) -> ContentView {
    match screen {
        MenuScreen::Command | MenuScreen::MemberSelect { .. } => ContentView {
            text: String::new(),
            cursor_line: None,
        },
        MenuScreen::ItemList { cursor } => {
            let (text, cursor_line) = items::compose_list(cursor, data, inventory);
            ContentView { text, cursor_line }
        }
        MenuScreen::ItemTarget { item_id, cursor } => ContentView {
            text: use_item::compose_target(
                hero_name,
                item_id,
                cursor,
                data,
                party,
                progression,
                vitals,
            ),
            cursor_line: None,
        },
        MenuScreen::SkillList { member, cursor } => {
            let (text, cursor_line) = skills::compose_list(
                hero_name,
                member,
                cursor,
                data,
                party,
                progression,
                equipment,
            );
            ContentView { text, cursor_line }
        }
        MenuScreen::SkillTarget {
            member,
            skill_id,
            cursor,
        } => ContentView {
            text: skills::compose_target(
                hero_name,
                member,
                skill_id,
                cursor,
                data,
                party,
                progression,
                vitals,
            ),
            cursor_line: None,
        },
        MenuScreen::Equip {
            member,
            slot,
            picking,
        } => {
            let (text, cursor_line) = equip::compose(
                hero_name,
                member,
                slot,
                picking,
                data,
                party,
                progression,
                inventory,
                equipment,
            );
            ContentView { text, cursor_line }
        }
        MenuScreen::Status { member } => ContentView {
            text: status::compose_status(
                hero_name,
                member,
                data,
                party,
                progression,
                vitals,
                equipment,
                terms,
            ),
            cursor_line: None,
        },
        MenuScreen::Saved => ContentView {
            text: compose_saved(),
            cursor_line: None,
        },
        MenuScreen::EndGame { cursor } => ContentView {
            text: compose_end_game(cursor),
            cursor_line: None,
        },
    }
}

/// The Save confirmation shown after the single-slot save is requested.
fn compose_saved() -> String {
    String::from("Mentés kész.\n\n[Enter] vissza   [Esc] vissza")
}

/// The End Game (return-to-title) confirmation, marking the cursor row.
fn compose_end_game(cursor: usize) -> String {
    let mut out = String::from("Visszatérsz a címképernyőre?\n\n");
    for (i, label) in END_GAME_ROWS.iter().enumerate() {
        let marker = if i == cursor { "▶ " } else { "  " };
        out.push_str(&format!("{marker}{label}\n"));
    }
    out.push_str("\n[Esc] vissza");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    fn condition(vitals: &Vitals) -> String {
        members(
            &crate::text::HeroName("Ron".into()),
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            vitals,
        )
        .remove(0)
        .condition
    }

    #[test]
    fn normal_menu_condition_does_not_invent_a_good_status_label() {
        assert!(condition(&Vitals::default()).is_empty());
    }

    #[test]
    fn menu_condition_shows_only_the_highest_priority_original_state() {
        let mut vitals = Vitals::default();
        vitals.set_states(1, vec![2, 3]);
        assert_eq!(condition(&vitals), "Vakság");
    }

    #[test]
    fn knockout_menu_condition_keeps_the_original_empty_name() {
        let mut vitals = Vitals::default();
        vitals.set(1, 0, 0);
        assert!(condition(&vitals).is_empty());
    }

    #[test]
    fn members_report_each_roster_slot_with_face_and_vitals() {
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);
        let views = members(
            &crate::text::HeroName("Ron".into()),
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &vitals,
        );
        assert_eq!(views.len(), 1, "one-member default party");
        let ron = &views[0];
        assert_eq!(ron.name, "Ron");
        assert_eq!(ron.face_name, "Ron", "portrait plumbed from the actor def");
        assert_eq!(ron.face_index, 6);
        assert_eq!((ron.hp, ron.max_hp), (20, 63));
        assert_eq!((ron.sp, ron.max_sp), (5, 37));
        assert_eq!(ron.condition, "");
        assert_eq!(ron.condition_color, None);
    }

    #[test]
    fn content_item_list_reports_the_cursor_line() {
        let mut inv = Inventory::default();
        inv.add_item(testkit::ITEM_HERB, 3);
        let view = content(
            &crate::text::HeroName("Ron".into()),
            MenuScreen::ItemList { cursor: 0 },
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &inv,
            &Vitals::default(),
            &Equipment::default(),
            &Terms::default(),
        );
        assert!(view.text.contains("Gyógyfű ×3"), "item row: {}", view.text);
        assert_eq!(view.cursor_line, Some(2), "windowskin cursor line");
    }

    #[test]
    fn content_end_game_lists_igen_then_nem_with_an_esc_hint() {
        let view = content(
            &crate::text::HeroName("Ron".into()),
            MenuScreen::EndGame { cursor: 1 },
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Inventory::default(),
            &Vitals::default(),
            &Equipment::default(),
            &Terms::default(),
        );
        assert!(
            view.text.contains("▶ Nem"),
            "cursor defaults to Nem: {}",
            view.text
        );
        assert!(view.text.contains("Igen"), "Igen present: {}", view.text);
        assert!(
            view.text.contains("[Esc] vissza"),
            "esc hint: {}",
            view.text
        );
    }

    #[test]
    fn content_saved_screen_carries_both_dismissal_hints() {
        let view = content(
            &crate::text::HeroName("Ron".into()),
            MenuScreen::Saved,
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Inventory::default(),
            &Vitals::default(),
            &Equipment::default(),
            &Terms::default(),
        );
        assert!(
            view.text.contains("[Enter] vissza"),
            "enter hint: {}",
            view.text
        );
        assert!(
            view.text.contains("[Esc] vissza"),
            "esc hint: {}",
            view.text
        );
    }
}

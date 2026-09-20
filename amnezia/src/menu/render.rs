//! Party figures for the main menu and the remaining text-based subscreens.

use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;
use crate::terms::Terms;
use crate::vitals::Vitals;

use super::{MenuScreen, derive, status};

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

/// Legacy subscreen text and its optional highlighted line.
pub(super) struct ContentView {
    pub text: String,
    pub cursor_line: Option<usize>,
}

/// Compose the internal compatibility status view.
#[allow(clippy::too_many_arguments)]
pub(super) fn content(
    hero_name: &crate::text::HeroName,
    screen: MenuScreen,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
    equipment: &Equipment,
    terms: &Terms,
) -> ContentView {
    match screen {
        MenuScreen::Command
        | MenuScreen::MemberSelect { .. }
        | MenuScreen::EndGame { .. }
        | MenuScreen::ItemList { .. }
        | MenuScreen::ItemTarget { .. }
        | MenuScreen::SkillList { .. }
        | MenuScreen::SkillTarget { .. }
        | MenuScreen::Equip { .. } => ContentView {
            text: String::new(),
            cursor_line: None,
        },
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
    }
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
    fn original_party_experience_columns_use_the_actual_level_thresholds() {
        let data = GameData {
            actors: crate::assets::load_ron(&format!("{}/actors.ron", crate::assets::asset_root())),
            ..testkit::data()
        };
        let mut party = Party::default();
        party.restore(vec![1, 2, 3, 4]);
        let members = members(
            &crate::text::HeroName("Ron".into()),
            &data,
            &party,
            &Progression::default(),
            &Vitals::default(),
        );
        assert_eq!(
            members.iter().map(|member| member.exp).collect::<Vec<_>>(),
            vec![
                Some((30, 83)),
                Some((80, 165)),
                Some((2033, 2651)),
                Some((1091, 1515))
            ]
        );
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
    fn item_list_does_not_compose_legacy_headers_gold_or_cursor_text() {
        let view = content(
            &crate::text::HeroName("Ron".into()),
            MenuScreen::ItemList { cursor: 0 },
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Vitals::default(),
            &Equipment::default(),
            &Terms::default(),
        );
        assert!(view.text.is_empty());
        assert_eq!(view.cursor_line, None);
    }

    #[test]
    fn target_screens_do_not_compose_non_original_headers_or_text_arrows() {
        for screen in [
            MenuScreen::ItemTarget {
                item_id: 5,
                cursor: 0,
            },
            MenuScreen::SkillTarget {
                member: 0,
                skill_id: 1,
                cursor: 0,
            },
        ] {
            let view = content(
                &crate::text::HeroName("Ron".into()),
                screen,
                &testkit::data(),
                &Party::default(),
                &Progression::default(),
                &Vitals::default(),
                &Equipment::default(),
                &Terms::default(),
            );
            assert!(view.text.is_empty(), "{screen:?}: {}", view.text);
            assert_eq!(view.cursor_line, None);
        }
    }

    #[test]
    fn end_game_does_not_compose_invented_full_screen_text() {
        let view = content(
            &crate::text::HeroName("Ron".into()),
            MenuScreen::EndGame { cursor: 1 },
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Vitals::default(),
            &Equipment::default(),
            &Terms::default(),
        );
        assert!(view.text.is_empty());
        assert_eq!(view.cursor_line, None);
    }
}

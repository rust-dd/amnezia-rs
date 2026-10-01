//! Runtime known-skill lists and field recovery for self, ally or party targets.
//! Displayed and paid SP costs share the equipment-adjusted rule.

use crate::equipment::{Equipment, EquipmentEffects};
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::Party;
use crate::vitals::Vitals;
use amnezia_data::SkillDef;

use super::derive;

mod availability;
mod cast;
mod list;
pub(crate) mod smoke;
pub(super) use availability::{can_use, cost};
pub(super) use cast::apply_field_skill;
pub(super) use list::{List, update};

/// Whether `skill` can be cast on an ally from the field menu: a normal
/// recovery skill aimed at the caster or allies.
pub(super) fn field_usable(skill: &SkillDef) -> bool {
    skill.skill_type == 0
        && (2..=4).contains(&skill.scope)
        && (skill.affect_hp
            || skill.affect_sp
            || crate::conditions::definitions()
                .iter()
                .any(|state| state.persistence == 1 && skill.affected_states.contains(&state.id)))
}

/// Runtime known skills in database order, shared by rendering and cursor lookup.
pub(super) fn known_skills<'a>(
    member: usize,
    data: &'a GameData,
    party: &Party,
    progression: &Progression,
) -> Vec<&'a SkillDef> {
    let Some(def) = party.snapshot().get(member).and_then(|&id| data.actor(id)) else {
        return Vec::new();
    };
    let ids = progression.known_skill_ids(def);
    let mut known = data
        .skills
        .iter()
        .filter(|skill| ids.contains(&skill.id))
        .collect::<Vec<_>>();
    known.sort_unstable_by_key(|skill| skill.id);
    known
}

/// The skill under `cursor` in the `member`'s known-skill list.
pub(super) fn skill_at<'a>(
    member: usize,
    cursor: usize,
    data: &'a GameData,
    party: &Party,
    progression: &Progression,
) -> Option<&'a SkillDef> {
    known_skills(member, data, party, progression)
        .into_iter()
        .nth(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::EventRng;
    use crate::menu::testkit;

    #[test]
    fn moonstone_cost_is_shared_by_the_field_list_and_actual_cast() {
        let mut data = testkit::data();
        data.skills = vec![testkit::heal_skill(2, "Gyógyítás", 9, 10)];
        data.actors[0].learnings = vec![amnezia_data::Learning {
            level: 1,
            skill_id: 2,
        }];
        data.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
        let mut equipment = Equipment::default();
        equipment.set_slot(&data.actors[0], 1, 152);
        equipment.set_slot(&data.actors[0], 2, 157);
        let party = Party::default();
        let progression = Progression::default();
        assert_eq!(cost(&data.actors[0], &data.skills[0], &data, &equipment), 5);
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 4);
        assert!(!apply_field_skill(
            0,
            0,
            2,
            &data,
            &party,
            &progression,
            &mut vitals,
            &equipment,
            &mut EventRng::seeded(1)
        ));
        vitals.set(1, 20, 5);
        assert!(apply_field_skill(
            0,
            0,
            2,
            &data,
            &party,
            &progression,
            &mut vitals,
            &equipment,
            &mut EventRng::seeded(1)
        ));
        assert_eq!(vitals.get_stored(1), Some((30, 0)));
    }

    #[test]
    fn field_usable_accepts_ally_hp_heals_and_rejects_battle_skills() {
        assert!(field_usable(&testkit::heal_skill(2, "Gyógyítás", 8, 40)));
        assert!(!field_usable(&testkit::skill(3, "Csapás", 10)));
    }

    #[test]
    fn skill_list_hides_skills_the_member_has_not_learned() {
        use amnezia_data::Learning;
        let mut d = testkit::data();
        d.skills = vec![
            testkit::heal_skill(2, "Gyógyítás", 8, 40),
            testkit::skill(3, "Tűzgolyó", 12),
        ];
        d.actors[0].learnings = vec![
            Learning {
                level: 1,
                skill_id: 2,
            },
            Learning {
                level: 5,
                skill_id: 3,
            },
        ];
        let known = known_skills(0, &d, &Party::default(), &Progression::default());
        assert_eq!(known.iter().map(|skill| skill.id).collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn apply_heals_the_target_and_spends_the_casters_sp() {
        let mut d = testkit::data();
        d.skills = vec![testkit::heal_skill(2, "Gyógyítás", 8, 40)];
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 30);
        let ok = apply_field_skill(
            0,
            0,
            2,
            &d,
            &Party::default(),
            &Progression::default(),
            &mut vitals,
            &Equipment::default(),
            &mut EventRng::seeded(1),
        );
        assert!(ok, "an affordable field heal applies");
        assert_eq!(vitals.get_stored(1), Some((60, 22)));
    }

    #[test]
    fn apply_refuses_when_sp_is_too_low_or_the_skill_is_battle_only() {
        let mut d = testkit::data();
        d.skills = vec![
            testkit::heal_skill(2, "Gyógyítás", 8, 40),
            testkit::skill(3, "Tűzgolyó", 12),
        ];
        let party = Party::default();
        let prog = Progression::default();

        let mut broke = Vitals::default();
        broke.set(1, 20, 3);
        assert!(!apply_field_skill(
            0,
            0,
            2,
            &d,
            &party,
            &prog,
            &mut broke,
            &Equipment::default(),
            &mut EventRng::seeded(1)
        ));
        assert_eq!(
            broke.get_stored(1),
            Some((20, 3)),
            "unchanged when unaffordable"
        );

        let mut full = Vitals::default();
        full.set(1, 20, 30);
        assert!(
            !apply_field_skill(
                0,
                0,
                3,
                &d,
                &party,
                &prog,
                &mut full,
                &Equipment::default(),
                &mut EventRng::seeded(1)
            ),
            "a battle-only skill can't be cast in the field"
        );
    }
}

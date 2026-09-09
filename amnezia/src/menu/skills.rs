//! The Skill command: a chosen party member's known skills with their SP costs,
//! and — for field-usable recovery skills — applying one to a chosen ally. A
//! member knows the skills its actor `learnings` grant at or below its current
//! level (see [`Progression::known_skill_ids`]); the list is those, not the whole
//! database, and the caster member matters because casting spends *their* SP.
//!
//! Field recovery supports self, single-ally and whole-party targets. Equipment
//! modifies the displayed and paid SP cost through the same shared rule.

use crate::equipment::{Equipment, EquipmentEffects};
use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;
use crate::vitals::Vitals;
use amnezia_data::SkillDef;

use super::derive;
use super::items::viewport_start;

mod cast;
pub(super) use cast::apply_field_skill;

/// How many skill rows fit before the list scrolls with the cursor.
const VISIBLE_ROWS: usize = 8;

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

/// The `member`'s known skills in database (id) order: the skill defs whose ids
/// the member has learned by its current level (see
/// [`Progression::known_skill_ids`]). Both the list the menu draws and the cursor
/// it moves index into this, not the whole skill database.
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
    data.skills.iter().filter(|s| ids.contains(&s.id)).collect()
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

/// Compose the caster's skill list and the composed-text line its windowskin
/// cursor sits on (`None` when the caster knows no skills). Battle-only skills are
/// tagged "(harc)" to read as inert; the list scrolls within a viewport and the
/// line indexes into the returned text so [`super::view`] can place the cursor.
pub(super) fn compose_list(
    member: usize,
    cursor: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    equipment: &Equipment,
) -> (String, Option<usize>) {
    let effects = party
        .snapshot()
        .get(member)
        .and_then(|&id| data.actor(id))
        .map(|def| EquipmentEffects::from_slots(equipment.slots(def), &data.items))
        .unwrap_or_default();
    let caster = party
        .snapshot()
        .get(member)
        .and_then(|&id| data.actor(id))
        .map(|def| i18n::tr(&def.name))
        .unwrap_or_default();
    let known = known_skills(member, data, party, progression);
    let mut lines = vec![format!("- Képességek -  {caster}"), String::new()];
    if known.is_empty() {
        lines.push(String::from("(nincs képesség)"));
        lines.push(String::new());
        lines.push(String::from("[Esc] vissza"));
        return (lines.join("\n"), None);
    }
    let start = viewport_start(cursor, known.len(), VISIBLE_ROWS);
    let mut cursor_line = None;
    for (i, &skill) in known.iter().enumerate().skip(start).take(VISIBLE_ROWS) {
        if i == cursor {
            cursor_line = Some(lines.len());
        }
        let tag = if field_usable(skill) { "" } else { "  (harc)" };
        lines.push(format!(
            "{}  SP {}{tag}",
            i18n::tr(&skill.name),
            effects.skill_cost(skill.sp_cost)
        ));
    }
    lines.push(String::new());
    lines.push(String::from("[Esc] vissza"));
    (lines.join("\n"), cursor_line)
}

/// Compose the ally-target picker for a field skill: the skill being cast plus the
/// party roster with the selection cursor and each member's current/maximum HP.
pub(super) fn compose_target(
    member: usize,
    skill_id: u32,
    cursor: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
) -> String {
    let name = data
        .skills
        .iter()
        .find(|s| s.id == skill_id)
        .map(|s| i18n::tr(&s.name))
        .unwrap_or_default();
    let caster = party
        .snapshot()
        .get(member)
        .and_then(|&id| data.actor(id))
        .map(|def| i18n::tr(&def.name))
        .unwrap_or_default();
    let mut out = format!("{caster}: {name}          [Esc] vissza\n\n");
    for (row, &id) in party.snapshot().iter().enumerate() {
        let marker = if row == cursor { "▶ " } else { "  " };
        match data.actor(id) {
            Some(def) => {
                let level = progression.level(def);
                let (max_hp, max_sp) = derive::max_hp_sp(def, level);
                let (hp, sp) = vitals.get_stored(id).unwrap_or((max_hp, max_sp));
                out.push_str(&format!(
                    "{marker}{} — HP {hp}/{max_hp}   SP {sp}/{max_sp}\n",
                    i18n::tr(&def.name)
                ));
            }
            None => out.push_str(&format!("{marker}#{id}\n")),
        }
    }
    out
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
        let (text, _) = compose_list(0, 0, &data, &party, &progression, &equipment);
        assert!(text.contains("SP 5"), "{text}");
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
        // A plain attack skill (scope 0) is battle-only.
        assert!(!field_usable(&testkit::skill(3, "Csapás", 10)));
    }

    #[test]
    fn skill_list_shows_costs_and_tags_battle_only_skills() {
        use amnezia_data::Learning;
        let mut d = testkit::data();
        d.skills = vec![
            testkit::heal_skill(2, "Gyógyítás", 8, 40),
            testkit::skill(3, "Tűzgolyó", 12),
        ];
        // The hero (level 2) has learned both skills by level 1.
        d.actors[0].learnings = vec![
            Learning {
                level: 1,
                skill_id: 2,
            },
            Learning {
                level: 1,
                skill_id: 3,
            },
        ];
        let (text, cursor_line) = compose_list(
            0,
            0,
            &d,
            &Party::default(),
            &Progression::default(),
            &Equipment::default(),
        );
        assert!(text.contains("Gyógyítás  SP 8"), "heal, no tag: {text}");
        assert!(
            text.contains("Tűzgolyó  SP 12  (harc)"),
            "attack tagged battle-only: {text}"
        );
        // Header line 0, blank line 1, first skill on line 2.
        assert_eq!(cursor_line, Some(2), "cursor over the first skill: {text}");
    }

    #[test]
    fn skill_list_hides_skills_the_member_has_not_learned() {
        use amnezia_data::Learning;
        let mut d = testkit::data();
        d.skills = vec![
            testkit::heal_skill(2, "Gyógyítás", 8, 40),
            testkit::skill(3, "Tűzgolyó", 12),
        ];
        // Only the heal (skill 2) is learned by level 2; the fireball needs level 5.
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
        let (text, _) = compose_list(
            0,
            0,
            &d,
            &Party::default(),
            &Progression::default(),
            &Equipment::default(),
        );
        assert!(text.contains("Gyógyítás"), "learned skill shown: {text}");
        assert!(!text.contains("Tűzgolyó"), "unlearned skill hidden: {text}");
    }

    #[test]
    fn apply_heals_the_target_and_spends_the_casters_sp() {
        let mut d = testkit::data();
        d.skills = vec![testkit::heal_skill(2, "Gyógyítás", 8, 40)];
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 30);
        // One-member party: the caster is also the sole target.
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
        // HP 20 + 40 power = 60 (below the 63 max); SP 30 - 8 cost = 22.
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
        broke.set(1, 20, 3); // only 3 SP, heal costs 8
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

//! The Skill command: a chosen party member's known skills with their SP costs,
//! and — for field-usable recovery skills — applying one to a chosen ally. The
//! game's data model carries no per-actor skill learning, so the list is the whole
//! skill database (see the module note in `menu`); the caster member still matters
//! because casting spends *their* SP.
//!
//! A skill counts as field-usable here when it is a normal (`skill_type` 0)
//! HP-recovery skill that targets an ally (`scope` 3 one ally, 4 all allies) with
//! positive `power`. Everything else — attack, self, SP-only, teleport/escape/
//! switch skills — is shown inert (tagged "(harc)", battle-only). Applying a field
//! skill is deliberately simplified: it heals a flat `power` HP (not the full
//! spirit/magical-rate battle formula) to the single chosen ally (an all-ally
//! scope still resolves to one target), spending the caster's SP.

use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;
use crate::vitals::Vitals;
use amnezia_data::SkillDef;

use super::derive;
use super::items::viewport_start;

/// How many skill rows fit before the list scrolls with the cursor.
const VISIBLE_ROWS: usize = 12;

/// Whether `skill` can be cast on an ally from the field menu: a normal
/// HP-recovery skill aimed at an ally (scope 3/4) with positive power.
pub(super) fn field_usable(skill: &SkillDef) -> bool {
    skill.skill_type == 0
        && skill.affect_hp
        && (skill.scope == 3 || skill.scope == 4)
        && skill.power > 0
}

/// The skill under `cursor` in the database order the list shows.
pub(super) fn skill_at(cursor: usize, data: &GameData) -> Option<&SkillDef> {
    data.skills.get(cursor)
}

/// Apply a field-usable skill cast by `caster` on `target`: spend the caster's SP
/// (defaulting to full when unrecorded) and heal the target a flat `power` HP,
/// clamped to their maximum. Returns `false` — changing nothing — when the skill
/// isn't field-usable or the caster can't afford its SP cost.
pub(super) fn apply_field_skill(
    caster: usize,
    target: usize,
    skill_id: u32,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &mut Vitals,
) -> bool {
    let roster = party.snapshot();
    let (Some(&caster_id), Some(&target_id)) = (roster.get(caster), roster.get(target)) else {
        return false;
    };
    let Some(skill) = data.skills.iter().find(|s| s.id == skill_id) else {
        return false;
    };
    if !field_usable(skill) {
        return false;
    }
    let Some(caster_def) = data.actor(caster_id) else {
        return false;
    };
    let caster_level = progression.level(caster_def);
    let (caster_max_hp, caster_max_sp) = derive::max_hp_sp(caster_def, caster_level);
    let (caster_hp, caster_sp) = vitals
        .get_stored(caster_id)
        .unwrap_or((caster_max_hp, caster_max_sp));
    let cost = skill.sp_cost as i32;
    if caster_sp < cost {
        return false;
    }
    let caster_sp = caster_sp - cost;
    let power = skill.power as i32;
    if caster_id == target_id {
        let hp = (caster_hp + power).min(caster_max_hp);
        vitals.set(caster_id, hp, caster_sp);
        return true;
    }
    let Some(target_def) = data.actor(target_id) else {
        return false;
    };
    let target_level = progression.level(target_def);
    let (target_max_hp, target_max_sp) = derive::max_hp_sp(target_def, target_level);
    let (target_hp, target_sp) = vitals
        .get_stored(target_id)
        .unwrap_or((target_max_hp, target_max_sp));
    let target_hp = (target_hp + power).min(target_max_hp);
    vitals.set(caster_id, caster_hp, caster_sp);
    vitals.set(target_id, target_hp, target_sp);
    true
}

/// Compose the caster's skill list, marking the cursor row within the scrolling
/// viewport. Battle-only skills are tagged "(harc)" to read as greyed/inert.
pub(super) fn compose_list(member: usize, cursor: usize, data: &GameData, party: &Party) -> String {
    let caster = party
        .snapshot()
        .get(member)
        .and_then(|&id| data.actor(id))
        .map(|def| i18n::tr(&def.name))
        .unwrap_or_default();
    let mut out = format!("- Képességek -  {caster}\n\n");
    if data.skills.is_empty() {
        out.push_str("  (nincs képesség)\n");
    }
    let start = viewport_start(cursor, data.skills.len(), VISIBLE_ROWS);
    for (i, skill) in data
        .skills
        .iter()
        .enumerate()
        .skip(start)
        .take(VISIBLE_ROWS)
    {
        let marker = if i == cursor { "▶ " } else { "  " };
        let tag = if field_usable(skill) { "" } else { "  (harc)" };
        out.push_str(&format!(
            "{marker}{}  SP {}{tag}\n",
            i18n::tr(&skill.name),
            skill.sp_cost
        ));
    }
    out.push_str("\n[Esc] vissza");
    out
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
    use crate::menu::testkit;

    #[test]
    fn field_usable_accepts_ally_hp_heals_and_rejects_battle_skills() {
        assert!(field_usable(&testkit::heal_skill(2, "Gyógyítás", 8, 40)));
        // A plain attack skill (scope 0) is battle-only.
        assert!(!field_usable(&testkit::skill(3, "Csapás", 10)));
    }

    #[test]
    fn skill_list_shows_costs_and_tags_battle_only_skills() {
        let mut d = testkit::data();
        d.skills = vec![
            testkit::heal_skill(2, "Gyógyítás", 8, 40),
            testkit::skill(3, "Tűzgolyó", 12),
        ];
        let text = compose_list(0, 0, &d, &Party::default());
        assert!(text.contains("▶ Gyógyítás  SP 8"), "heal, no tag: {text}");
        assert!(
            text.contains("Tűzgolyó  SP 12  (harc)"),
            "attack tagged battle-only: {text}"
        );
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
        assert!(!apply_field_skill(0, 0, 2, &d, &party, &prog, &mut broke));
        assert_eq!(
            broke.get_stored(1),
            Some((20, 3)),
            "unchanged when unaffordable"
        );

        let mut full = Vitals::default();
        full.set(1, 20, 30);
        assert!(
            !apply_field_skill(0, 0, 3, &d, &party, &prog, &mut full),
            "a battle-only skill can't be cast in the field"
        );
    }
}

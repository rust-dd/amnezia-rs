//! The Status sub-screen: a read-only detail block for one party member. It
//! shows the member's level, experience, current/maximum HP and SP, the four
//! battle stats, and the five equipment slots — every figure derived exactly as
//! the battle system derives it (see [`super::derive`]), so the numbers match a
//! real fight.

use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;
use crate::terms::Terms;
use crate::vitals::Vitals;

use super::derive;

/// The five equipment slot labels, in `ActorDef` slot order — the faithful
/// Hungarian fallbacks used when the real `weapon`/`shield`/`armor`/`helmet`/
/// `accessory` terms are blank.
const SLOT_FALLBACKS: [&str; 5] = ["Fegyver", "Pajzs", "Vért", "Sisak", "Kiegészítő"];

/// Compose the status block for the `member`-th roster entry. Falls back to a
/// short placeholder when the member index or actor id is unknown. The field
/// labels (level, HP/SP, the four battle stats, the five equipment slots) come
/// from the real RM2000 Terms, each falling back to its Hungarian placeholder.
#[allow(clippy::too_many_arguments)]
pub(super) fn compose_status(
    member: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
    equipment: &Equipment,
    terms: &Terms,
) -> String {
    let roster = party.snapshot();
    let Some(&id) = roster.get(member) else {
        return "(nincs karakter)\n".to_string();
    };
    let Some(def) = data.actor(id) else {
        return format!("#{id} (ismeretlen)\n");
    };

    let t = &terms.0;
    let level = progression.level(def);
    let (max_hp, max_sp) = derive::max_hp_sp(def, level);
    let (hp, sp) = vitals.get_stored(id).unwrap_or((max_hp, max_sp));
    // The runtime loadout is the source of truth, so the stats and slot names
    // here reflect a gear change made on the equip screen.
    let slots = equipment.slots(def);
    let stats = derive::stats_with_slots(def, level, &data.items, slots);
    let total = progression.total(def);
    let exp_label = terms.label(&t.exp_short, "EXP");
    let exp = match derive::exp_to_next(def, total, level) {
        Some(rem) => format!("{exp_label} {total} (köv: {rem})"),
        None => format!("{exp_label} {total} (max)"),
    };

    let mut out = format!("{} — {}\n", i18n::tr(&def.name), i18n::tr(&def.title));
    out.push_str(&format!(
        "{} {level}   {exp}\n\n",
        terms.label(&t.level, "Szint")
    ));
    out.push_str(&format!(
        "{} {hp}/{max_hp}   {} {sp}/{max_sp}\n\n",
        terms.label(&t.hp_short, "HP"),
        terms.label(&t.sp_short, "SP")
    ));
    out.push_str(&format!(
        "{} {}   {} {}\n{} {}   {} {}\n\n",
        terms.label(&t.attack, "Támadás"),
        stats[0],
        terms.label(&t.defense, "Védelem"),
        stats[1],
        terms.label(&t.spirit, "Szellem"),
        stats[2],
        terms.label(&t.agility, "Gyorsaság"),
        stats[3],
    ));

    let slot_terms = [&t.weapon, &t.shield, &t.armor, &t.helmet, &t.accessory];
    let slot_labels: Vec<String> = slot_terms
        .iter()
        .zip(SLOT_FALLBACKS.iter())
        .map(|(term, fallback)| terms.label(term, fallback))
        .collect();
    for (label, &slot) in slot_labels.iter().zip(slots.iter()) {
        let gear = if slot == 0 {
            "—".to_string()
        } else {
            data.item(slot)
                .map(|item| i18n::tr(&item.name))
                .unwrap_or_else(|| format!("#{slot}"))
        };
        out.push_str(&format!("{label}: {gear}\n"));
    }
    out.push_str("\n[Esc] vissza\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    #[test]
    fn status_block_has_level_hp_sp_stats_and_equipment_names() {
        let mut d = testkit::data();
        // Equip the hero and register the gear so the slot names resolve.
        d.actors[0].weapon = 10;
        d.actors[0].armor = 11;
        d.items.push(testkit::weapon(10, "Rövidkard", 4));
        let mut vest = testkit::blank_item(11, 3);
        vest.name = "Bőrvért".into();
        vest.def = 3;
        d.items.push(vest);

        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);

        let text = compose_status(
            0,
            &d,
            &Party::default(),
            &Progression::default(),
            &vitals,
            &Equipment::default(),
            &Terms::default(),
        );

        assert!(text.contains("Szint 2"), "level: {text}");
        assert!(text.contains("HP 20/63"), "hp cur/max: {text}");
        assert!(text.contains("SP 5/37"), "sp cur/max: {text}");
        // Level-2 linear fallback attack 16 + 2*6 = 28, plus the +4 weapon = 32.
        assert!(text.contains("Támadás 32"), "attack: {text}");
        assert!(text.contains("Védelem"), "defense label: {text}");
        assert!(text.contains("Rövidkard"), "weapon name: {text}");
        assert!(text.contains("Bőrvért"), "armor name: {text}");
        assert!(text.contains("Pajzs: —"), "empty shield slot: {text}");
    }

    #[test]
    fn full_health_member_without_stored_vitals_shows_max() {
        let text = compose_status(
            0,
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Vitals::default(),
            &Equipment::default(),
            &Terms::default(),
        );
        assert!(text.contains("HP 63/63"), "defaults to full: {text}");
    }

    #[test]
    fn status_reflects_a_runtime_equipment_change() {
        let mut d = testkit::data();
        d.actors[0].weapon = 10; // ActorDef starting weapon
        d.items.push(testkit::weapon(10, "Rövidkard", 4));
        d.items.push(testkit::weapon(12, "Hosszúkard", 12));
        // The runtime store swapped in the long-sword; status must show it.
        let mut eq = Equipment::default();
        eq.set_slot(d.actor(1).unwrap(), 0, 12);

        let text = compose_status(
            0,
            &d,
            &Party::default(),
            &Progression::default(),
            &Vitals::default(),
            &eq,
            &Terms::default(),
        );
        assert!(text.contains("Hosszúkard"), "runtime weapon name: {text}");
        assert!(
            !text.contains("Rövidkard"),
            "not the ActorDef default: {text}"
        );
        // Level-2 base 28 + the +12 long-sword = 40.
        assert!(
            text.contains("Támadás 40"),
            "runtime-derived attack: {text}"
        );
    }
}

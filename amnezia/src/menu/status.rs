//! The Status sub-screen: a read-only detail block for one party member. It
//! shows the member's level, experience, current/maximum HP and SP, the four
//! battle stats, and the five equipment slots — every figure derived exactly as
//! the battle system derives it (see [`super::derive`]), so the numbers match a
//! real fight.

use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;
use crate::vitals::Vitals;

use super::derive;

/// The five equipment slot labels, in `ActorDef` slot order.
const SLOT_LABELS: [&str; 5] = ["Fegyver", "Pajzs", "Vért", "Sisak", "Kiegészítő"];

/// Compose the status block for the `member`-th roster entry. Falls back to a
/// short placeholder when the member index or actor id is unknown.
pub(super) fn compose_status(
    member: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
) -> String {
    let roster = party.snapshot();
    let Some(&id) = roster.get(member) else {
        return "(nincs karakter)\n".to_string();
    };
    let Some(def) = data.actor(id) else {
        return format!("#{id} (ismeretlen)\n");
    };

    let level = progression.level(def);
    let (max_hp, max_sp) = derive::max_hp_sp(def, level);
    let (hp, sp) = vitals.get_stored(id).unwrap_or((max_hp, max_sp));
    let stats = derive::stats_at(def, level, &data.items);
    let total = progression.total(def);
    let exp = match derive::exp_to_next(def, total, level) {
        Some(rem) => format!("EXP {total} (köv: {rem})"),
        None => format!("EXP {total} (max)"),
    };

    let mut out = format!("{} — {}\n", i18n::tr(&def.name), i18n::tr(&def.title));
    out.push_str(&format!("Szint {level}   {exp}\n\n"));
    out.push_str(&format!("HP {hp}/{max_hp}   SP {sp}/{max_sp}\n\n"));
    out.push_str(&format!(
        "Támadás {}   Védelem {}\nSzellem {}   Gyorsaság {}\n\n",
        stats[0], stats[1], stats[2], stats[3]
    ));

    let slots = [def.weapon, def.shield, def.armor, def.helmet, def.accessory];
    for (label, &slot) in SLOT_LABELS.iter().zip(slots.iter()) {
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

        let text = compose_status(0, &d, &Party::default(), &Progression::default(), &vitals);

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
        );
        assert!(text.contains("HP 63/63"), "defaults to full: {text}");
    }
}

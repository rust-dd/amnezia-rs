//! The Equipment command's read-only view for one party member: the five gear
//! slots (weapon / shield / armor / helmet / accessory) with the equipped item
//! names, and the four battle stats that gear produces, derived exactly as the
//! battle system derives them (see [`super::derive`]). Changing gear is a separate
//! follow-up (runtime equipment plus battle integration); this view only reports
//! the member's current loadout.

use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;

use super::derive;

/// The five equipment slot labels, in `ActorDef` slot order.
const SLOT_LABELS: [&str; 5] = ["Fegyver", "Pajzs", "Vért", "Sisak", "Kiegészítő"];

/// Compose the equipment block for the `member`-th roster entry: their five slots
/// with the equipped item names and the resulting attack/defense/spirit/agility.
pub(super) fn compose(
    member: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
) -> String {
    let roster = party.snapshot();
    let Some(&id) = roster.get(member) else {
        return "(nincs karakter)\n".to_string();
    };
    let Some(def) = data.actor(id) else {
        return format!("#{id} (ismeretlen)\n");
    };

    let level = progression.level(def);
    let stats = derive::stats_at(def, level, &data.items);
    let mut out = format!("- Felszerelés -  {}\n\n", i18n::tr(&def.name));

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
    out.push_str(&format!(
        "\nTámadás {}   Védelem {}\nSzellem {}   Gyorsaság {}\n",
        stats[0], stats[1], stats[2], stats[3]
    ));
    out.push_str("\n[Esc] vissza\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    #[test]
    fn equipment_view_lists_slots_with_names_and_resulting_stats() {
        let mut d = testkit::data();
        d.actors[0].weapon = 10;
        d.actors[0].armor = 11;
        d.items.push(testkit::weapon(10, "Rövidkard", 4));
        let mut vest = testkit::blank_item(11, 3);
        vest.name = "Bőrvért".into();
        vest.def = 3;
        d.items.push(vest);

        let text = compose(0, &d, &Party::default(), &Progression::default());

        assert!(text.contains("Fegyver: Rövidkard"), "weapon slot: {text}");
        assert!(text.contains("Vért: Bőrvért"), "armor slot: {text}");
        assert!(text.contains("Pajzs: —"), "empty shield slot: {text}");
        // Level-2 linear fallback attack 16 + 2*6 = 28, plus the +4 weapon = 32.
        assert!(text.contains("Támadás 32"), "resulting attack: {text}");
        assert!(text.contains("Gyorsaság"), "agility label: {text}");
    }

    #[test]
    fn unknown_member_falls_back_to_a_placeholder() {
        let text = compose(
            9,
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
        );
        assert!(text.contains("nincs karakter"), "placeholder: {text}");
    }
}

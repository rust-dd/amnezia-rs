//! Equipment candidates, inventory swaps, and the legacy composed-text view.
//! Selection lives in [`super::MenuScreen::Equip`]; actual changes use [`Equipment::equip`].

use crate::equipment::{self, Equipment};
use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::{Inventory, Party};

use super::derive;

pub(crate) mod smoke;

#[cfg(test)]
mod selection_tests;

/// The five equipment slot labels, in `ActorDef` slot order.
const SLOT_LABELS: [&str; 5] = ["Fegyver", "Pajzs", "Vért", "Sisak", "Kiegészítő"];

/// The four battle-stat labels the equipment screen previews, in derive order.
const STAT_LABELS: [&str; 4] = ["Támadás", "Védelem", "Szellem", "Gyorsaság"];

/// Whether the `member`-th actor may change gear at all (RM2000 `fix_equipment`
/// actors can't). The input layer gates opening the item picker on this.
pub(super) fn can_change(member: usize, data: &GameData, party: &Party) -> bool {
    party
        .snapshot()
        .get(member)
        .and_then(|&id| data.actor(id))
        .map(|def| !def.fix_equipment)
        .unwrap_or(false)
}

/// The item ids the `member`-th actor may put in 0-based `slot`, in list order:
/// held items in database id order, followed by `0` (unequip). Fixed equipment
/// has no candidates. A dual-wielder's shield slot lists weapons instead of shields.
pub(super) fn candidates(
    member: usize,
    slot: usize,
    data: &GameData,
    party: &Party,
    inventory: &Inventory,
) -> Vec<u32> {
    let Some(def) = party.snapshot().get(member).and_then(|&id| data.actor(id)) else {
        return Vec::new();
    };
    if def.fix_equipment {
        return Vec::new();
    }
    let mut ids = Vec::new();
    for item in &data.items {
        if equipment::can_equip(def, slot, item) && inventory.count(item.id) > 0 {
            ids.push(item.id);
        }
    }
    ids.sort_unstable();
    ids.push(0);
    ids
}

/// Apply the candidate under `cursor` to the `member`-th actor's 0-based `slot`:
/// swap the gear through the runtime store, moving items between it and the
/// inventory (see [`Equipment::equip`]). Returns whether anything changed.
pub(super) fn apply(
    member: usize,
    slot: usize,
    cursor: usize,
    data: &GameData,
    party: &Party,
    inventory: &mut Inventory,
    equipment: &mut Equipment,
) -> bool {
    let roster = party.snapshot();
    let Some(&id) = roster.get(member) else {
        return false;
    };
    let Some(def) = data.actor(id) else {
        return false;
    };
    let cands = candidates(member, slot, data, party, inventory);
    let Some(&new_id) = cands.get(cursor) else {
        return false;
    };
    equipment.equip(def, slot, new_id, &data.items, inventory)
}

/// The display name of item `id` (`—` for the empty id `0`, `#id` for an unknown
/// one), routed through `i18n::tr` for English.
fn item_name(id: u32, data: &GameData) -> String {
    if id == 0 {
        "—".to_string()
    } else {
        data.item(id)
            .map(|item| i18n::tr(&item.name))
            .unwrap_or_else(|| format!("#{id}"))
    }
}

/// Compose the equipment screen for the `member`-th roster entry and the
/// composed-text line its windowskin cursor sits on. When `picking` is `None` the
/// five slots are listed with the current stats and the cursor sits on `slot`;
/// when it is `Some(cursor)` the candidate list for `slot` follows, the cursor
/// sits on the hovered candidate, and the stats read `current → new`.
#[allow(clippy::too_many_arguments)]
pub(super) fn compose(
    hero_name: &crate::text::HeroName,
    member: usize,
    slot: usize,
    picking: Option<usize>,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    inventory: &Inventory,
    equipment: &Equipment,
) -> (String, Option<usize>) {
    let roster = party.snapshot();
    let Some(&id) = roster.get(member) else {
        return ("(nincs karakter)\n".to_string(), None);
    };
    let Some(def) = data.actor(id) else {
        return (format!("#{id} (ismeretlen)\n"), None);
    };

    let level = progression.level(def);
    let slots = equipment.slots(def);
    let current = derive::stats_with_slots(def, level, &data.items, slots);

    let mut lines = vec![
        format!("- Felszerelés -  {}", i18n::tr(hero_name.actor(def))),
        String::new(),
    ];
    let mut cursor_line = None;

    match picking {
        None => {
            for (i, (label, &sid)) in SLOT_LABELS.iter().zip(slots.iter()).enumerate() {
                if i == slot {
                    cursor_line = Some(lines.len());
                }
                lines.push(format!("{label}: {}", item_name(sid, data)));
            }
            lines.push(String::new());
            stat_rows(&mut lines, current, None);
            if def.fix_equipment {
                lines.push("(a felszerelés rögzített)".to_string());
            }
        }
        Some(cursor) => {
            let cands = candidates(member, slot, data, party, inventory);
            lines.push(format!("{} cseréje:", SLOT_LABELS[slot.min(4)]));
            let start = super::items::viewport_start(cursor, cands.len(), 6);
            for (i, &cid) in cands.iter().enumerate().skip(start).take(6) {
                if i == cursor {
                    cursor_line = Some(lines.len());
                }
                let row = if cid == 0 {
                    "— (levesz)".to_string()
                } else {
                    format!("{} ×{}", item_name(cid, data), inventory.count(cid))
                };
                lines.push(row);
            }
            lines.push(String::new());
            let new_id = cands.get(cursor).copied().unwrap_or(0);
            let after = equipment::preview_slots(slots, slot, new_id, &data.items);
            let preview = derive::stats_with_slots(def, level, &data.items, after);
            stat_rows(&mut lines, current, Some(preview));
        }
    }

    lines.push("[Esc] vissza".to_string());
    (lines.join("\n"), cursor_line)
}

fn stat_rows(lines: &mut Vec<String>, current: [u32; 4], preview: Option<[u32; 4]>) {
    for pair in [0..2, 2..4] {
        let row = pair
            .map(|i| match preview {
                Some(after) => format!("{} {} → {}", STAT_LABELS[i], current[i], after[i]),
                None => format!("{} {}", STAT_LABELS[i], current[i]),
            })
            .collect::<Vec<_>>();
        lines.push(row.join("   "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    /// A database with the hero, a starting short-sword (id 10) in the weapon
    /// slot, and a stronger long-sword (id 12) available to swap in.
    fn armed_data() -> GameData {
        let mut d = testkit::data();
        d.actors[0].weapon = 10;
        d.items.push(testkit::weapon(10, "Rövidkard", 4));
        d.items.push(testkit::weapon(12, "Hosszúkard", 12));
        d
    }

    #[test]
    fn slot_view_lists_gear_and_current_stats() {
        let d = armed_data();
        let (text, cursor_line) = compose(
            &crate::text::HeroName("Ron".into()),
            0,
            0,
            None,
            &d,
            &Party::default(),
            &Progression::default(),
            &Inventory::default(),
            &Equipment::default(),
        );
        assert!(text.contains("Fegyver: Rövidkard"), "weapon slot: {text}");
        assert!(text.contains("Pajzs: —"), "empty shield slot: {text}");
        // Level-2 linear fallback attack 16 + 2*6 = 28, plus the +4 weapon = 32.
        assert!(text.contains("Támadás 32"), "current attack: {text}");
        assert_eq!(cursor_line, Some(2), "cursor on the weapon row: {text}");
    }

    #[test]
    fn item_picker_lists_matching_gear_and_previews_the_stat_change() {
        let d = armed_data();
        let mut inv = Inventory::default();
        inv.add_item(12, 1);
        let (text, cursor_line) = compose(
            &crate::text::HeroName("Ron".into()),
            0,
            0,
            Some(0),
            &d,
            &Party::default(),
            &Progression::default(),
            &inv,
            &Equipment::default(),
        );
        assert!(text.contains("Fegyver cseréje:"), "picker header: {text}");
        assert!(text.contains("— (levesz)"), "unequip option: {text}");
        assert!(
            text.contains("Hosszúkard ×1"),
            "candidate with count: {text}"
        );
        // Current 32 (short-sword +4) previews to 40 (long-sword +12).
        assert!(text.contains("Támadás 32 → 40"), "stat preview: {text}");
        assert!(
            cursor_line.is_some(),
            "the candidate cursor is placed: {text}"
        );
    }

    #[test]
    fn candidates_end_with_unequip_and_only_hold_the_slot_type() {
        let mut d = testkit::data();
        d.items.push(testkit::weapon(10, "Kard", 4));
        let mut shield = testkit::blank_item(20, 2);
        shield.name = "Pajzs".into();
        d.items.push(shield);
        let mut inv = Inventory::default();
        inv.add_item(10, 1);
        inv.add_item(20, 1);
        let weapon_slot = candidates(0, 0, &d, &Party::default(), &inv);
        assert_eq!(weapon_slot, vec![10, 0]);
        let shield_slot = candidates(0, 1, &d, &Party::default(), &inv);
        assert_eq!(shield_slot, vec![20, 0]);
        d.items
            .iter_mut()
            .find(|item| item.id == 10)
            .unwrap()
            .actor_set = vec![false];
        assert_eq!(candidates(0, 0, &d, &Party::default(), &inv), [0]);
    }

    #[test]
    fn apply_equips_a_weapon_moves_it_from_the_bag_and_changes_the_derived_atk() {
        let d = armed_data();
        let party = Party::default();
        let mut inv = Inventory::default();
        inv.add_item(12, 1);
        let mut eq = Equipment::default();

        let def = d.actor(1).unwrap();
        let before = derive::stats_with_slots(def, 2, &d.items, eq.slots(def));

        let changed = apply(0, 0, 0, &d, &party, &mut inv, &mut eq);
        assert!(changed, "equipping an available weapon applies");
        assert_eq!(eq.slots(def)[0], 12, "the long-sword is now worn");
        assert_eq!(inv.count(12), 0, "it left the inventory");
        assert_eq!(
            inv.count(10),
            1,
            "the short-sword returned to the inventory"
        );

        let after = derive::stats_with_slots(def, 2, &d.items, eq.slots(def));
        assert_eq!(after[0], before[0] + 8, "atk rose by the +12 vs +4 swap");
    }

    #[test]
    fn a_fixed_equipment_member_cannot_open_the_picker() {
        let mut d = armed_data();
        d.actors[0].fix_equipment = true;
        assert!(!can_change(0, &d, &Party::default()));
        let (text, _) = compose(
            &crate::text::HeroName("Ron".into()),
            0,
            0,
            None,
            &d,
            &Party::default(),
            &Progression::default(),
            &Inventory::default(),
            &Equipment::default(),
        );
        assert!(text.contains("rögzített"), "fixed-equipment note: {text}");
    }
}

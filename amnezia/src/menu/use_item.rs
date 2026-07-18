//! The field item-use sub-screen: pick a party member and apply a held,
//! field-usable recovery item to them. This module owns the field-usable
//! predicate, the held-item ordering that maps a browse row back to an item id,
//! the pure heal arithmetic (mirroring the battle item formula), and the target
//! list rendering. The state transitions and the inventory/vitals writes live in
//! the parent `menu` module.

use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::vitals::Vitals;
use amnezia_data::ItemDef;

use super::derive;

/// Whether `item` can be used on a party member from the field menu: never pure
/// equipment (types 1–5), and only when it restores HP/SP (flat or percent) or
/// cures states. `only_field` items — flagged usable only from the map — count as
/// field-usable by definition.
pub(super) fn field_usable(item: &ItemDef) -> bool {
    if (1..=5).contains(&item.item_type) {
        return false;
    }
    item.recover_hp > 0
        || item.recover_hp_rate > 0
        || item.recover_sp > 0
        || item.recover_sp_rate > 0
        || !item.cure_states.is_empty()
        || item.only_field
}

/// The ids of the held items in the order the Items tab lists them, so a
/// browse-cursor row index maps back to the item id under it. Rows past the end
/// of this list are the blank spacer and the gold line, which select nothing.
pub(super) fn held_item_ids(data: &GameData, inventory: &Inventory) -> Vec<u32> {
    data.items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
        .map(|i| i.id)
        .collect()
}

/// The `(hp, sp)` a member reaches after `item` is applied: each pool gains the
/// item's flat amount plus its percent-of-maximum, clamped to the maximum and
/// only when that pool's gain is positive. Mirrors the battle item formula.
/// `cure_states` is intentionally a no-op here: the field tracks no persistent
/// states to lift.
pub(super) fn heal(hp: i32, sp: i32, max_hp: i32, max_sp: i32, item: &ItemDef) -> (i32, i32) {
    let hp_gain = item.recover_hp as i32 + max_hp * item.recover_hp_rate as i32 / 100;
    let sp_gain = item.recover_sp as i32 + max_sp * item.recover_sp_rate as i32 / 100;
    let hp = if hp_gain > 0 {
        (hp + hp_gain).min(max_hp)
    } else {
        hp
    };
    let sp = if sp_gain > 0 {
        (sp + sp_gain).min(max_sp)
    } else {
        sp
    };
    (hp, sp)
}

/// Apply the field item to the `member`-th party member: heal via [`Vitals`]
/// (max HP/SP derived at their level, exactly as battle does) and consume one from
/// the inventory. Returns `true` when the stack is now empty, signalling the
/// caller to return to Browse.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_field_item(
    item_id: u32,
    member: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    inventory: &mut Inventory,
    vitals: &mut Vitals,
) -> bool {
    let roster = party.snapshot();
    let Some(&actor_id) = roster.get(member) else {
        return false;
    };
    let (Some(def), Some(item)) = (data.actor(actor_id), data.item(item_id)) else {
        return false;
    };
    let level = progression.level(def);
    let (max_hp, max_sp) = derive::max_hp_sp(def, level);
    let (hp, sp) = vitals.get_stored(actor_id).unwrap_or((max_hp, max_sp));
    let (hp, sp) = heal(hp, sp, max_hp, max_sp, item);
    vitals.set(actor_id, hp, sp);
    inventory.remove_item(item_id, 1);
    inventory.count(item_id) == 0
}

/// The item-target sub-screen: the item being applied plus the party roster with
/// the selection cursor and each member's current/maximum HP and SP at their
/// level.
pub(super) fn compose_target(
    item_id: u32,
    cursor: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
) -> String {
    let name = data
        .item(item_id)
        .map(|item| i18n::tr(&item.name))
        .unwrap_or_default();
    let mut out = format!("Használ: {name}          [Esc] vissza\n\n");
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
    use crate::menu::testkit::{self, ITEM_HERB};

    #[test]
    fn field_usable_accepts_recovery_and_rejects_equipment() {
        assert!(field_usable(&testkit::herb()), "a healing item is usable");
        assert!(
            !field_usable(&testkit::weapon(2, "Kard", 5)),
            "equipment is never a field consumable"
        );
        assert!(
            !field_usable(&testkit::blank_item(9, 0)),
            "a normal item with no effect is not usable"
        );

        let mut only_field = testkit::blank_item(9, 0);
        only_field.only_field = true;
        assert!(field_usable(&only_field), "only_field is honored as usable");

        let mut curer = testkit::blank_item(9, 0);
        curer.cure_states = vec![1];
        assert!(field_usable(&curer), "a state cure is usable");
    }

    #[test]
    fn heal_adds_flat_plus_percent_and_clamps_to_max() {
        let herb = testkit::herb(); // +20 flat, +10% of max
        // 20 + 20 + (63 * 10 / 100 = 6) = 46, below the 63 max.
        assert_eq!(heal(20, 5, 63, 37, &herb), (46, 5));
        // Already at max: the same gain clamps back to 63.
        assert_eq!(heal(63, 37, 63, 37, &herb), (63, 37));
        // Near max: clamps to the cap rather than overshooting.
        assert_eq!(heal(60, 0, 63, 37, &herb).0, 63);
    }

    #[test]
    fn heal_touches_only_the_pools_the_item_restores() {
        let mut potion = testkit::blank_item(9, 6);
        potion.recover_sp = 15;
        // HP untouched (no HP effect), SP gains the flat 15.
        assert_eq!(heal(10, 5, 63, 37, &potion), (10, 20));
    }

    #[test]
    fn held_item_ids_follow_the_items_tab_order() {
        let mut d = testkit::data();
        d.items.push(testkit::weapon(7, "Kard", 3));
        let mut inv = Inventory::default();
        inv.add_item(7, 1);
        inv.add_item(ITEM_HERB, 2);
        // Order follows `data.items` (herb id 5 before weapon id 7), not add order.
        assert_eq!(held_item_ids(&d, &inv), vec![ITEM_HERB, 7]);
    }

    #[test]
    fn target_view_names_the_item_and_lists_members_with_vitals() {
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);
        let text = compose_target(
            ITEM_HERB,
            0,
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &vitals,
        );
        assert!(text.contains("Gyógyfű"), "item name: {text}");
        assert!(
            text.contains("▶ Ron — HP 20/63"),
            "cursor + member row: {text}"
        );
    }

    #[test]
    fn apply_raises_vitals_and_decrements_inventory_then_empties() {
        let d = testkit::data();
        let prog = Progression::default();
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 2);
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);

        let empty = apply_field_item(
            ITEM_HERB,
            0,
            &d,
            &Party::default(),
            &prog,
            &mut inv,
            &mut vitals,
        );
        assert!(!empty, "one herb left, so still in the target screen");
        // 20 + 20 flat + 10% of 63 (=6) = 46; SP untouched.
        assert_eq!(vitals.get_stored(1), Some((46, 5)));
        assert_eq!(inv.count(ITEM_HERB), 1);

        let empty = apply_field_item(
            ITEM_HERB,
            0,
            &d,
            &Party::default(),
            &prog,
            &mut inv,
            &mut vitals,
        );
        assert!(empty, "last herb consumed -> caller returns to Browse");
        assert_eq!(inv.count(ITEM_HERB), 0);
    }

    #[test]
    fn apply_on_a_full_member_stays_clamped_at_max() {
        let d = testkit::data();
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 1);
        let mut vitals = Vitals::default();
        // No stored vitals -> full (63/37); healing keeps it at max.
        apply_field_item(
            ITEM_HERB,
            0,
            &d,
            &Party::default(),
            &Progression::default(),
            &mut inv,
            &mut vitals,
        );
        assert_eq!(vitals.get_stored(1), Some((63, 37)));
    }
}

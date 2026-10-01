//! Equipment candidates, inventory swaps, and the original scene navigation.
//! Selection lives in [`super::MenuScreen::Equip`]; actual changes use [`Equipment::equip`].

use crate::equipment::{self, Equipment};
use crate::gamedata::GameData;
use crate::state::{Inventory, Party};

pub(crate) mod layout_smoke;
mod scene;
pub(crate) mod smoke;
mod switching;

pub(super) use scene::{Scene, refresh_actor, stats, update};
pub(crate) use switching::Switch;
pub(super) use switching::register as register_switching;

#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod stat_tests;

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

/// Apply a candidate through [`Equipment::equip`]; return whether anything changed.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{derive, testkit};

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
    }
}

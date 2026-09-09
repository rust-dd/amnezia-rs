//! Runtime equipment: the five gear slots (weapon / shield / armor / helmet /
//! accessory) each party actor currently wears. This is the single source of
//! truth for equipped gear once the game is running — the menu changes it, the
//! battle reads it, and the save persists it. Each actor's `ActorDef` slots stay
//! the *starting* defaults: an actor with no stored entry here falls back to
//! them, so a new game (and any member that joins later) begins with its RM2000
//! starting loadout without an explicit initialisation pass, exactly as
//! [`crate::vitals::Vitals`] and [`crate::progression::Progression`] treat their
//! `ActorDef` values as the default.
//!
//! The mutation ([`Equipment::equip`]) mirrors EasyRPG `Game_Actor::ChangeEquipment`:
//! the newly worn item leaves the inventory, the displaced item returns to it, a
//! `fix_equipment` actor refuses every change, and a two-handed weapon clears the
//! other hand (returning that item too).

use crate::state::Inventory;
use amnezia_data::{ActorDef, ItemDef};
use bevy::prelude::*;
use std::collections::HashMap;

/// The five gear slot indices, in `ActorDef` slot order.
const SLOTS: usize = 5;

/// The RM2000 item categories the equip slots accept, by 0-based slot index:
/// weapon slot → type 1, shield → 2, armor → 3, helmet → 4, accessory → 5. A
/// dual-wielding actor (`two_weapons`) fills the shield slot with a second
/// weapon, so that slot accepts weapons (type 1) instead (EasyRPG
/// `Window_EquipItem` maps `shield` to `weapon` when `HasTwoWeapons`).
pub fn slot_item_type(slot: usize, two_weapons: bool) -> u32 {
    match slot {
        0 => 1,
        1 if two_weapons => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        _ => 5,
    }
}

/// The per-actor equipped item ids, keyed by actor id. An actor absent from the
/// map wears its `ActorDef` starting gear (see [`Equipment::slots`]).
#[derive(Resource, Default)]
pub struct Equipment(HashMap<u32, [u32; SLOTS]>);

impl Equipment {
    /// The actor's five equipped item ids (0 = empty slot), reading its stored
    /// loadout or falling back to the `ActorDef` starting gear when it has none
    /// changed yet.
    pub fn slots(&self, def: &ActorDef) -> [u32; SLOTS] {
        self.0.get(&def.id).copied().unwrap_or([
            def.weapon,
            def.shield,
            def.armor,
            def.helmet,
            def.accessory,
        ])
    }

    /// The item id in one 0-based slot for the actor (an out-of-range slot reads
    /// the accessory slot). The get half of the requested per-slot accessors;
    /// consumers so far read the whole loadout via [`Equipment::slots`].
    #[allow(dead_code)]
    pub fn slot(&self, def: &ActorDef, slot: usize) -> u32 {
        self.slots(def)[slot.min(SLOTS - 1)]
    }

    /// Set one 0-based slot for the actor directly, seeding the rest from its
    /// current loadout so the other slots are preserved. Bypasses the inventory
    /// and the equip rules; the menu uses [`Equipment::equip`] for a real swap.
    #[allow(dead_code)]
    pub fn set_slot(&mut self, def: &ActorDef, slot: usize, item_id: u32) {
        if slot >= SLOTS {
            return;
        }
        let mut slots = self.slots(def);
        slots[slot] = item_id;
        self.0.insert(def.id, slots);
    }

    /// Equip `new_id` (0 = unequip) into the actor's 0-based `slot`, moving items
    /// between the slot and the party inventory the RM2000 way (EasyRPG
    /// `Game_Actor::ChangeEquipment`): the displaced item returns to the
    /// inventory, the newly worn item leaves it, and equipping a two-handed
    /// weapon clears the other hand (that item returns too). A `fix_equipment`
    /// actor refuses every change. Returns whether anything changed.
    pub fn equip(
        &mut self,
        def: &ActorDef,
        slot: usize,
        new_id: u32,
        items: &[ItemDef],
        inventory: &mut Inventory,
    ) -> bool {
        if def.fix_equipment || slot >= SLOTS {
            return false;
        }
        self.equip_from_event(def, slot, new_id, items, inventory)
    }

    /// Scripted equipment changes also apply to actors whose menu loadout is fixed.
    pub fn equip_from_event(
        &mut self,
        def: &ActorDef,
        slot: usize,
        new_id: u32,
        items: &[ItemDef],
        inventory: &mut Inventory,
    ) -> bool {
        if slot >= SLOTS {
            return false;
        }
        let before = self.slots(def);
        let after = preview_slots(before, slot, new_id, items);
        if before == after {
            return false;
        }
        // Every slot whose occupant changed moves one item: the old one back to
        // the inventory, the new one out of it. The two-handed clear is folded
        // into `after`, so the displaced shield is handled by the same diff.
        for i in 0..SLOTS {
            if before[i] != after[i] {
                if before[i] != 0 {
                    inventory.add_item(before[i], 1);
                }
                if after[i] != 0 {
                    inventory.remove_item(after[i], 1);
                }
            }
        }
        self.0.insert(def.id, after);
        true
    }

    /// Snapshot `(actor_id, slots)` pairs for the save file, in id order. Only
    /// actors whose gear was changed are stored; an untouched actor is absent, so
    /// a load restores it to its `ActorDef` starting gear.
    pub fn entries(&self) -> Vec<(u32, [u32; SLOTS])> {
        let mut entries: Vec<(u32, [u32; SLOTS])> =
            self.0.iter().map(|(&id, &slots)| (id, slots)).collect();
        entries.sort_by_key(|&(id, _)| id);
        entries
    }

    /// Replace all stored loadouts from a loaded save.
    pub fn load(&mut self, entries: Vec<(u32, [u32; SLOTS])>) {
        self.0 = entries.into_iter().collect();
    }
}

/// The slots that result from placing `new_id` (0 = unequip) into 0-based `slot`,
/// applying the two-handed rule but touching no inventory. Shared by the real
/// swap ([`Equipment::equip`]) and the menu's stat-change preview so both agree.
pub fn preview_slots(
    mut slots: [u32; SLOTS],
    slot: usize,
    new_id: u32,
    items: &[ItemDef],
) -> [u32; SLOTS] {
    if slot >= SLOTS {
        return slots;
    }
    slots[slot] = new_id;
    clear_other_hand(&mut slots, slot, items);
    slots
}

/// When a two-handed weapon occupies a hand while the other hand is also filled,
/// clear the other hand (EasyRPG `Game_Actor::ChangeEquipment`). Only a change to
/// a hand slot (weapon 0 or shield 1) can trigger it; a change to armour/helmet/
/// accessory leaves the hands alone.
fn clear_other_hand(slots: &mut [u32; SLOTS], changed: usize, items: &[ItemDef]) {
    if changed > 1 {
        return;
    }
    let two_handed = |id: u32| {
        items
            .iter()
            .find(|i| i.id == id)
            .is_some_and(|i| i.item_type == 1 && i.two_handed)
    };
    if slots[0] != 0 && slots[1] != 0 && (two_handed(slots[0]) || two_handed(slots[1])) {
        let other = if changed == 0 { 1 } else { 0 };
        slots[other] = 0;
    }
}

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Equipment>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(id: u32) -> ActorDef {
        ActorDef {
            critical_hit: false,
            critical_hit_chance: 30,
            state_ranks: Vec::new(),
            attribute_ranks: Vec::new(),
            id,
            name: format!("A{id}"),
            title: String::new(),
            level: 1,
            max_level: 50,
            hp: 60,
            sp: 30,
            curves: Default::default(),
            learnings: Vec::new(),
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
            face_name: String::new(),
            face_index: 0,
        }
    }

    fn item(id: u32, item_type: u32) -> ItemDef {
        ItemDef {
            state_chance: 0,
            id,
            name: format!("I{id}"),
            description: String::new(),
            item_type,
            price: 0,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            cure_states: vec![],
            scope: 0,
            only_field: false,
            ko_only: false,
            uses: 0,
            atk: 0,
            def: 0,
            spi: 0,
            agi: 0,
            attribute_defense: vec![],
            state_defense: vec![],
            two_handed: false,
            hit: 0,
            crit: 0,
            weapon_animation: 0,
        }
    }

    #[test]
    fn slots_default_to_the_actor_starting_gear_until_changed() {
        let mut a = actor(1);
        a.weapon = 4;
        a.armor = 7;
        let mut eq = Equipment::default();
        assert_eq!(eq.slots(&a), [4, 0, 7, 0, 0]);
        assert_eq!(eq.slot(&a, 2), 7);
        eq.set_slot(&a, 0, 9);
        assert_eq!(eq.slots(&a), [9, 0, 7, 0, 0]);
    }

    #[test]
    fn equip_swaps_the_slot_and_moves_items_through_the_inventory() {
        let mut a = actor(1);
        a.weapon = 5;
        let items = vec![item(5, 1), item(7, 1)];
        let mut eq = Equipment::default();
        let mut inv = Inventory::default();
        inv.add_item(7, 1);

        assert!(eq.equip(&a, 0, 7, &items, &mut inv));
        assert_eq!(eq.slots(&a)[0], 7, "the new weapon is worn");
        assert_eq!(inv.count(7), 0, "it left the inventory");
        assert_eq!(inv.count(5), 1, "the old weapon returned to the inventory");
        assert!(eq.equip(&a, 0, 0, &items, &mut inv));
        assert_eq!(eq.slots(&a)[0], 0);
        assert_eq!(inv.count(7), 1, "the unequipped weapon is back in the bag");
    }

    #[test]
    fn a_fixed_equipment_actor_refuses_every_change() {
        let mut a = actor(1);
        a.weapon = 5;
        a.fix_equipment = true;
        let items = vec![item(5, 1), item(7, 1)];
        let mut eq = Equipment::default();
        let mut inv = Inventory::default();
        inv.add_item(7, 1);
        assert!(
            !eq.equip(&a, 0, 7, &items, &mut inv),
            "no change is applied"
        );
        assert_eq!(eq.slots(&a)[0], 5, "the fixed loadout is untouched");
        assert_eq!(inv.count(7), 1, "the inventory is untouched");
    }

    #[test]
    fn a_two_handed_weapon_clears_the_shield() {
        let mut a = actor(1);
        a.shield = 20;
        let mut great_sword = item(10, 1);
        great_sword.two_handed = true;
        let items = vec![great_sword, item(20, 2)];
        let mut eq = Equipment::default();
        let mut inv = Inventory::default();
        inv.add_item(10, 1);

        assert!(eq.equip(&a, 0, 10, &items, &mut inv));
        let slots = eq.slots(&a);
        assert_eq!(slots[0], 10, "the two-handed weapon is worn");
        assert_eq!(slots[1], 0, "the shield slot is cleared");
        assert_eq!(inv.count(20), 1, "the displaced shield returned to the bag");
        assert_eq!(inv.count(10), 0, "the weapon left the bag");
    }

    #[test]
    fn dual_wielders_take_a_weapon_in_the_shield_slot() {
        assert_eq!(slot_item_type(1, false), 2, "shield slot wants shields");
        assert_eq!(slot_item_type(1, true), 1, "a dual-wielder wants a weapon");
        assert_eq!(slot_item_type(0, false), 1);
        assert_eq!(slot_item_type(4, false), 5);
    }

    #[test]
    fn entries_round_trip_through_load() {
        let mut eq = Equipment::default();
        let a = actor(3);
        eq.set_slot(&a, 0, 11);
        let snap = eq.entries();
        assert_eq!(snap, vec![(3, [11, 0, 0, 0, 0])]);
        let mut restored = Equipment::default();
        restored.load(snap);
        assert_eq!(restored.slots(&a), [11, 0, 0, 0, 0]);
    }
}

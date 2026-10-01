//! Runtime loadouts shared by menus, battles and saves; absent actors use their
//! `ActorDef` starting gear. Swaps follow EasyRPG `Game_Actor::ChangeEquipment`:
//! return displaced gear, consume replacements, enforce fixed/two-handed rules.

use crate::state::Inventory;
use amnezia_data::{ActorDef, ItemDef};
use bevy::prelude::*;
use std::collections::HashMap;

mod effects;
pub(crate) use effects::EquipmentEffects;

/// The five gear slot indices, in `ActorDef` slot order.
const SLOTS: usize = 5;

/// Map weapon/shield/armor/helmet/accessory slots to item categories 1–5.
/// Dual wielding makes the shield slot accept weapons (EasyRPG `Window_EquipItem`).
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

pub fn can_equip(actor: &ActorDef, slot: usize, item: &ItemDef) -> bool {
    slot < SLOTS
        && item.item_type == slot_item_type(slot, actor.two_weapons)
        && item.usable_by_actor(actor.id)
}

/// The per-actor equipped item ids, keyed by actor id. An actor absent from the
/// map wears its `ActorDef` starting gear (see [`Equipment::slots`]).
#[derive(Resource, Default)]
pub struct Equipment(HashMap<u32, [u32; SLOTS]>);

impl Equipment {
    /// Stored loadout or starting gear; item ID 0 means an empty slot.
    pub fn slots(&self, def: &ActorDef) -> [u32; SLOTS] {
        self.0.get(&def.id).copied().unwrap_or([
            def.weapon,
            def.shield,
            def.armor,
            def.helmet,
            def.accessory,
        ])
    }

    /// Read a zero-based slot; out-of-range indices select the accessory slot.
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

    /// Swap gear with inventory, enforcing fixed equipment and two-handed rules.
    /// `new_id = 0` unequips; returns whether the loadout changed.
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
        if new_id != 0 && inventory.count(new_id) == 0 {
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
        if new_id != 0
            && !items
                .iter()
                .find(|item| item.id == new_id)
                .is_some_and(|item| can_equip(def, slot, item))
        {
            return false;
        }
        let before = self.slots(def);
        let after = preview_slots(before, slot, new_id, items);
        if before == after {
            return false;
        }
        // The loadout diff also returns gear displaced by the two-handed rule.
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

    /// Save changed loadouts in actor-ID order; omitted actors retain starting gear.
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

/// Preview the same two-handed rules as a real swap, without changing inventory.
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

/// Two-handed swaps clear the other hand (EasyRPG `Game_Actor::ChangeEquipment`).
/// Changing armor, helmet or accessory must not affect either hand.
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
    mod restrictions;
    use super::*;

    fn actor(id: u32) -> ActorDef {
        ActorDef {
            character_name: String::new(),
            character_index: 0,
            rename_skill: false,
            skill_name: String::new(),
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
            prevent_critical: false,
            raise_evasion: false,
            half_sp_cost: false,
            actor_set: Vec::new(),
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

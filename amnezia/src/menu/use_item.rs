//! Field-item eligibility, inventory ordering and original recovery effects.

use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::vitals::Vitals;
use amnezia_data::ItemDef;

use super::derive;

/// Field use requires recovery or cured states; equipment types 1–5 are excluded.
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

/// Held item IDs in database order, mapping the two-column cursor to its item.
pub(super) fn held_item_ids(data: &GameData, inventory: &Inventory) -> Vec<u32> {
    data.items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
        .map(|i| i.id)
        .collect()
}

/// Apply flat-plus-percent recovery, capped at each maximum and only for positive gains.
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
/// the inventory. Returns whether any target recovered HP, SP or a condition.
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
    if inventory.count(item_id) == 0 || !field_usable(item) {
        return false;
    }
    let targets = if item.scope == 1 {
        roster
    } else {
        vec![def.id]
    };
    let mut changed = false;
    for actor_id in targets {
        let Some(def) = data.actor(actor_id) else {
            continue;
        };
        let full = derive::max_hp_sp(def, progression.level(def));
        let (hp, sp) = vitals.get_stored(actor_id).unwrap_or(full);
        if item.ko_only && hp > 0 {
            continue;
        }
        if hp == 0 && !item.cure_states.contains(&1) {
            continue;
        }
        let before = vitals.states(actor_id);
        let (new_hp, new_sp) = heal(hp, sp, full.0, full.1, item);
        let new_hp = if hp == 0 { new_hp.max(1) } else { new_hp };
        vitals.set(actor_id, new_hp, new_sp);
        for state in &item.cure_states {
            vitals.change_condition(actor_id, *state, false, full);
        }
        changed |= (hp, sp) != (new_hp, new_sp) || before != vitals.states(actor_id);
    }
    if changed {
        inventory.remove_item(item_id, 1);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_original_life_potion_is_kept_when_used_on_a_living_actor() {
        let mut data = testkit::data();
        data.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
        let mut inventory = Inventory::default();
        inventory.add_item(112, 1);
        let mut vitals = Vitals::default();
        vitals.set(1, 7, 5);
        assert!(!apply_field_item(
            112,
            0,
            &data,
            &Party::default(),
            &Progression::default(),
            &mut inventory,
            &mut vitals,
        ));
        assert_eq!(vitals.get_stored(1), Some((7, 5)));
        assert_eq!(inventory.count(112), 1);
        vitals.set(1, 0, 5);
        vitals.set_states(1, vec![1]);
        assert!(apply_field_item(
            112,
            0,
            &data,
            &Party::default(),
            &Progression::default(),
            &mut inventory,
            &mut vitals,
        ));
        assert_eq!(vitals.get_stored(1), Some((31, 5)));
        assert!(vitals.states(1).is_empty());
        assert_eq!(inventory.count(112), 0);
    }

    #[test]
    fn antidote_cures_poison_and_ordinary_herbs_cannot_revive() {
        let mut data = testkit::data();
        let mut antidote = testkit::blank_item(12, 6);
        antidote.cure_states = vec![2];
        data.items.push(antidote);
        let mut inventory = Inventory::default();
        inventory.add_item(12, 1);
        inventory.add_item(ITEM_HERB, 1);
        let mut vitals = Vitals::default();
        vitals.set_states(1, vec![2]);
        assert!(apply_field_item(
            12,
            0,
            &data,
            &Party::default(),
            &Progression::default(),
            &mut inventory,
            &mut vitals
        ));
        assert!(vitals.states(1).is_empty());
        vitals.set(1, 0, 5);
        assert!(!apply_field_item(
            ITEM_HERB,
            0,
            &data,
            &Party::default(),
            &Progression::default(),
            &mut inventory,
            &mut vitals
        ));
        assert_eq!(inventory.count(ITEM_HERB), 1);
        assert_eq!(vitals.get_stored(1), Some((0, 5)));
    }
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
        let herb = testkit::herb();
        assert_eq!(heal(20, 5, 63, 37, &herb), (46, 5));
        assert_eq!(heal(63, 37, 63, 37, &herb), (63, 37));
        assert_eq!(heal(60, 0, 63, 37, &herb).0, 63);
    }

    #[test]
    fn heal_touches_only_the_pools_the_item_restores() {
        let mut potion = testkit::blank_item(9, 6);
        potion.recover_sp = 15;
        assert_eq!(heal(10, 5, 63, 37, &potion), (10, 20));
    }

    #[test]
    fn held_item_ids_follow_the_items_tab_order() {
        let mut d = testkit::data();
        d.items.push(testkit::weapon(7, "Kard", 3));
        let mut inv = Inventory::default();
        inv.add_item(7, 1);
        inv.add_item(ITEM_HERB, 2);
        assert_eq!(held_item_ids(&d, &inv), vec![ITEM_HERB, 7]);
    }

    #[test]
    fn apply_raises_vitals_and_decrements_inventory_then_empties() {
        let d = testkit::data();
        let prog = Progression::default();
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 2);
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);

        let used = apply_field_item(
            ITEM_HERB,
            0,
            &d,
            &Party::default(),
            &prog,
            &mut inv,
            &mut vitals,
        );
        assert!(used);
        assert_eq!(vitals.get_stored(1), Some((46, 5)));
        assert_eq!(inv.count(ITEM_HERB), 1);

        let used = apply_field_item(
            ITEM_HERB,
            0,
            &d,
            &Party::default(),
            &prog,
            &mut inv,
            &mut vitals,
        );
        assert!(used);
        assert_eq!(inv.count(ITEM_HERB), 0);
    }

    #[test]
    fn apply_on_a_full_member_stays_clamped_at_max() {
        let d = testkit::data();
        let mut inv = Inventory::default();
        inv.add_item(ITEM_HERB, 1);
        let mut vitals = Vitals::default();
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

use super::*;
use crate::menu::testkit;

#[test]
fn candidates_are_in_database_id_order_with_the_empty_entry_last() {
    let mut data = testkit::data();
    data.items = [30, 10, 20].map(|id| testkit::weapon(id, "Kard", 4)).into();
    let mut inventory = Inventory::default();
    for id in [10, 20, 30] {
        inventory.add_item(id, 1);
    }
    assert_eq!(
        candidates(0, 0, &data, &Party::default(), &inventory),
        [10, 20, 30, 0]
    );
}

#[test]
fn fixed_equipment_hides_all_candidates_including_the_empty_entry() {
    let mut data = testkit::data();
    data.actors[0].fix_equipment = true;
    data.items.push(testkit::weapon(10, "Kard", 4));
    let mut inventory = Inventory::default();
    inventory.add_item(10, 1);
    for slot in 0..5 {
        assert!(candidates(0, slot, &data, &Party::default(), &inventory).is_empty());
    }
}

#[test]
fn all_original_actor_slots_keep_only_wearable_items_and_put_empty_last() {
    let mut data = testkit::data();
    let root = crate::assets::asset_root();
    data.actors = crate::assets::load_ron(&format!("{root}/actors.ron"));
    data.items = crate::assets::load_ron(&format!("{root}/items.ron"));
    assert_eq!((data.actors.len(), data.items.len()), (10, 202));
    assert!(data.actor(9).unwrap().fix_equipment);
    data.items.reverse();
    let mut inventory = Inventory::default();
    for item in &data.items {
        inventory.add_item(item.id, 1);
    }
    let mut party = Party::default();
    for actor in &data.actors {
        party.restore(vec![actor.id]);
        for slot in 0..5 {
            let mut expected = Vec::new();
            if !actor.fix_equipment {
                let category = if slot == 1 && actor.two_weapons {
                    1
                } else {
                    slot as u32 + 1
                };
                expected = data
                    .items
                    .iter()
                    .filter(|item| {
                        item.item_type == category
                            && item
                                .actor_set
                                .get(actor.id as usize - 1)
                                .copied()
                                .unwrap_or(true)
                    })
                    .map(|item| item.id)
                    .collect();
                expected.sort_unstable();
                expected.push(0);
            }
            assert_eq!(
                candidates(0, slot, &data, &party, &inventory),
                expected,
                "actor {}, slot {slot}",
                actor.id
            );
        }
    }
}

#[test]
fn an_empty_bag_still_allows_removal_but_a_missing_actor_has_no_entries() {
    let data = testkit::data();
    let mut party = Party::default();
    let inventory = Inventory::default();
    for slot in 0..5 {
        assert_eq!(candidates(0, slot, &data, &party, &inventory), [0]);
        assert!(candidates(1, slot, &data, &party, &inventory).is_empty());
    }
    party.restore(vec![999]);
    assert!(candidates(0, 0, &data, &party, &inventory).is_empty());
}

#[test]
fn the_last_entry_removes_equipment_without_consuming_the_other_candidates() {
    let mut data = testkit::data();
    data.actors[0].weapon = 10;
    data.items = vec![
        testkit::weapon(10, "Rövidkard", 4),
        testkit::weapon(12, "Hosszúkard", 12),
    ];
    let party = Party::default();
    let mut inventory = Inventory::default();
    inventory.add_item(12, 1);
    let mut equipment = Equipment::default();
    let ids = candidates(0, 0, &data, &party, &inventory);
    assert_eq!(ids, [12, 0]);
    assert!(apply(
        0,
        0,
        ids.len() - 1,
        &data,
        &party,
        &mut inventory,
        &mut equipment
    ));
    assert_eq!(equipment.slots(&data.actors[0])[0], 0);
    assert_eq!((inventory.count(10), inventory.count(12)), (1, 1));
    assert_eq!(candidates(0, 0, &data, &party, &inventory), [10, 12, 0]);
}

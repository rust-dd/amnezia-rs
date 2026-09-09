use super::*;

#[test]
fn original_gear_restrictions_cover_all_equipment_and_starting_loadouts() {
    let root = crate::assets::asset_root();
    let items = crate::assets::load_ron::<Vec<ItemDef>>(&format!("{root}/items.ron"));
    let actors = crate::assets::load_ron::<Vec<ActorDef>>(&format!("{root}/actors.ron"));
    let gear = items
        .iter()
        .filter(|item| matches!(item.item_type, 1..=5))
        .collect::<Vec<_>>();
    assert_eq!(gear.len(), 126);
    assert!(gear.iter().all(|item| !item.actor_set.is_empty()));
    let sword = items.iter().find(|item| item.id == 1).unwrap();
    let bow = items.iter().find(|item| item.id == 6).unwrap();
    let claw = items.iter().find(|item| item.id == 13).unwrap();
    for id in 1..=10 {
        assert_eq!(sword.usable_by_actor(id), id == 1);
        assert_eq!(bow.usable_by_actor(id), id == 2);
        assert_eq!(claw.usable_by_actor(id), id == 3);
    }
    for actor in &actors {
        for (slot, id) in Equipment::default().slots(actor).into_iter().enumerate() {
            if id != 0 {
                assert!(
                    can_equip(
                        actor,
                        slot,
                        items.iter().find(|item| item.id == id).unwrap()
                    ),
                    "actor {} slot {slot} item {id}",
                    actor.id
                );
            }
        }
    }
}

#[test]
fn menu_equipment_api_rejects_forbidden_wrong_type_unknown_and_unowned_items() {
    let actor = actor(1);
    let mut denied = item(7, 1);
    denied.actor_set = vec![false, true];
    let items = vec![denied, item(8, 3), item(9, 1)];
    let mut inventory = Inventory::default();
    inventory.add_item(7, 1);
    inventory.add_item(8, 1);
    let mut equipment = Equipment::default();
    for id in [7, 8, 9, 999] {
        assert!(!equipment.equip(&actor, 0, id, &items, &mut inventory));
        assert_eq!(equipment.slots(&actor), [0; 5]);
        assert_eq!(inventory.count(7), 1);
        assert_eq!(inventory.count(8), 1);
        assert_eq!(inventory.count(9), 0);
    }
    assert!(!items[0].usable_by_actor(1));
    assert!(items[0].usable_by_actor(2));
    assert!(items[0].usable_by_actor(3));
}

#[test]
fn scripted_equipping_respects_actor_flags_but_can_supply_unowned_gear_to_locked_actors() {
    let mut actor = actor(1);
    actor.fix_equipment = true;
    let mut weapon = item(7, 1);
    weapon.actor_set = vec![false, true];
    let items = [weapon];
    let mut inventory = Inventory::default();
    let mut equipment = Equipment::default();
    assert!(!equipment.equip_from_event(&actor, 0, 7, &items, &mut inventory));
    actor.id = 2;
    assert!(equipment.equip_from_event(&actor, 0, 7, &items, &mut inventory));
    assert_eq!(equipment.slots(&actor)[0], 7);
    assert_eq!(inventory.count(7), 0);
    assert!(!equipment.equip_from_event(&actor, 0, 7, &items, &mut inventory));
    assert!(equipment.equip_from_event(&actor, 0, 0, &items, &mut inventory));
    assert_eq!(inventory.count(7), 1);
}

#[test]
fn unequipping_an_existing_now_forbidden_item_returns_it_once() {
    let mut actor = actor(1);
    actor.weapon = 7;
    let mut weapon = item(7, 1);
    weapon.actor_set = vec![false];
    let mut inventory = Inventory::default();
    let mut equipment = Equipment::default();
    assert!(equipment.equip(&actor, 0, 0, &[weapon.clone()], &mut inventory));
    assert_eq!(inventory.count(7), 1);
    assert!(!equipment.equip(&actor, 0, 0, &[weapon], &mut inventory));
    assert_eq!(inventory.count(7), 1);
}

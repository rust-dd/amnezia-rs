use super::*;
use crate::state::Inventory;

fn use_item(battle: &mut Battle, inventory: &mut Inventory) -> bool {
    battle.resolve_next_with_items(|id| {
        if !inventory.has(id) {
            return false;
        }
        inventory.remove_item(id, 1);
        true
    })
}

fn item_action(user: usize, target: usize) -> Action {
    Action {
        source: Source::Party(user),
        kind: Command::Item {
            item_id: 50,
            target,
        },
        agility: 100,
    }
}

#[test]
fn a_used_item_is_consumed_once_when_its_effect_runs() {
    let mut battle = build_party2();
    battle.items = vec![medicine(50, 20, 0, vec![])];
    battle.members[1].hp = 1;
    battle.queue = vec![item_action(0, 1)];
    let mut inventory = Inventory::default();
    inventory.add_item(50, 2);
    assert!(use_item(&mut battle, &mut inventory));
    assert_eq!(inventory.count(50), 1);
    assert_eq!(battle.members[1].hp, 21);
    assert!(!use_item(&mut battle, &mut inventory));
    assert_eq!(inventory.count(50), 1);
}

#[test]
fn a_fallen_user_does_not_consume_the_queued_item() {
    let mut battle = build_party2();
    battle.items = vec![medicine(50, 20, 0, vec![])];
    battle.members[0].hp = 0;
    battle.members[1].hp = 1;
    battle.queue = vec![item_action(0, 1)];
    let mut inventory = Inventory::default();
    inventory.add_item(50, 1);
    use_item(&mut battle, &mut inventory);
    assert_eq!(inventory.count(50), 1);
    assert_eq!(battle.members[1].hp, 1);
}

#[test]
fn poison_leaves_the_user_alive_to_consume_the_queued_item() {
    let mut battle = build_party2();
    battle.items = vec![medicine(50, 20, 0, vec![])];
    battle.states = vec![hp_change_state(2, 0, 100, 0)];
    battle.members[0].states = vec![(2, 0)];
    battle.members[1].hp = 1;
    battle.queue = vec![item_action(0, 1)];
    let mut inventory = Inventory::default();
    inventory.add_item(50, 1);
    use_item(&mut battle, &mut inventory);
    assert_eq!(battle.members[0].hp, 1);
    assert_eq!(inventory.count(50), 0);
    assert_eq!(battle.members[1].hp, 21);
}

#[test]
fn two_orders_cannot_use_the_same_last_item_twice() {
    let mut battle = build_party2();
    battle.items = vec![medicine(50, 20, 0, vec![])];
    battle.members[1].hp = 1;
    battle.queue = vec![item_action(0, 1), item_action(1, 1)];
    let mut inventory = Inventory::default();
    inventory.add_item(50, 1);
    while use_item(&mut battle, &mut inventory) {}
    assert_eq!(inventory.count(50), 0);
    assert_eq!(battle.members[1].hp, 21);
}

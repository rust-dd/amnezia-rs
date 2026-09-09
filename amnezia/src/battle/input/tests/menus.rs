use super::*;

#[test]
fn cancelling_a_committed_item_order_leaves_inventory_unchanged() {
    let mut battle = build_party2();
    battle.begin_actor_commands();
    let mut inventory = Inventory::default();
    inventory.add_item(50, 1);
    open_ally_target(&mut battle, None, Some(50));
    ally_target_menu(&press_enter(), &inventory, &mut battle);
    assert_eq!(battle.turn, 1);
    let mut cancel = ButtonInput::default();
    cancel.press(KeyCode::Escape);
    command_menu(&cancel, &mut battle);
    assert_eq!(battle.turn, 0);
    assert!(battle.members[0].command.is_none());
    assert_eq!(inventory.count(50), 1);
}

#[test]
fn revival_items_can_target_a_fallen_party_member() {
    let mut battle = build_party2();
    battle.begin_actor_commands();
    battle.members[1].hp = 0;
    let mut inventory = Inventory::default();
    inventory.add_item(112, 1);
    open_ally_target(&mut battle, None, Some(112));
    let mut down = ButtonInput::<KeyCode>::default();
    down.press(KeyCode::ArrowDown);
    ally_target_menu(&down, &inventory, &mut battle);
    assert_eq!(battle.cursor, 1);
    ally_target_menu(&press_enter(), &inventory, &mut battle);
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Item {
            item_id: 112,
            target: 1
        })
    ));
}

#[test]
fn cancelling_a_target_restores_the_selected_skill_and_command() {
    let mut battle = build_party2();
    battle.begin_actor_commands();
    battle.cursor = 1;
    command_menu(&press_enter(), &mut battle);
    battle.cursor = 11;
    open_target(&mut battle, Some(2));
    let mut cancel = ButtonInput::<KeyCode>::default();
    cancel.press(KeyCode::Escape);
    target_menu(&cancel, &mut battle);
    assert!(battle.menu == MenuLevel::Skill);
    assert_eq!(battle.cursor, 11);
    enter(&mut battle, MenuLevel::Command);
    assert_eq!(battle.cursor, 1);
}

#[test]
fn grid_navigation_matches_two_columns_and_stops_at_an_incomplete_last_row() {
    let mut cursor = 0;
    for (key, expected) in [
        (KeyCode::ArrowRight, 1),
        (KeyCode::ArrowDown, 3),
        (KeyCode::ArrowDown, 3),
        (KeyCode::ArrowLeft, 2),
        (KeyCode::ArrowDown, 4),
        (KeyCode::ArrowUp, 2),
    ] {
        let mut keys = ButtonInput::default();
        keys.press(key);
        move_grid_cursor(&keys, &mut cursor, 5);
        assert_eq!(cursor, expected);
    }
}

#[test]
fn unaffordable_and_zero_power_skills_stay_visible_without_allowing_invalid_orders() {
    let mut battle = build_party2();
    let mut expensive = skill_def(1, 20, 0);
    expensive.sp_cost = 999;
    let zero_power = skill_def(2, 0, 0);
    battle.members[battle.turn].known_skills = vec![1, 2];
    battle.skills = vec![expensive.clone(), zero_power.clone()];
    let data = GameData {
        actors: vec![],
        items: vec![],
        skills: vec![expensive, zero_power],
    };
    assert_eq!(skill_choices(&data, &[1, 2], 10).len(), 2);
    battle.menu = MenuLevel::Skill;
    battle.cursor = 0;
    skill_menu(&press_enter(), &data, &mut battle);
    assert!(battle.menu == MenuLevel::Skill);
    assert!(matches!(battle.pending_se.last(), Some(BattleSe::Buzzer)));
    battle.cursor = 1;
    skill_menu(&press_enter(), &data, &mut battle);
    assert!(battle.menu == MenuLevel::Target);
}

#[test]
fn field_only_medicine_stays_visible_but_cannot_be_selected_in_battle() {
    let mut item = medicine(1);
    item.only_field = true;
    let data = GameData {
        actors: vec![],
        items: vec![item],
        skills: vec![],
    };
    let mut inventory = Inventory::default();
    inventory.add_item(1, 2);
    assert_eq!(item_choices(&data, &inventory).len(), 1);
    let mut battle = build_party2();
    battle.menu = MenuLevel::Item;
    item_menu(&press_enter(), &data, &inventory, &mut battle);
    assert!(battle.menu == MenuLevel::Item);
    assert_eq!(inventory.count(1), 2);
    assert!(matches!(battle.pending_se.last(), Some(BattleSe::Buzzer)));
}

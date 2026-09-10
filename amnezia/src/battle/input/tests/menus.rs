use super::*;

#[test]
fn an_attack_clears_an_earlier_medicine_target_context() {
    let mut battle = build_party2();
    battle.begin_actor_commands();
    battle.pending_item = Some(50);
    open_target(&mut battle, None);
    assert!(battle.menu == MenuLevel::Target);
    assert_eq!(battle.pending_item, None);
    assert_eq!(battle.pending_skill, None);
}

#[test]
fn group_skills_commit_without_an_individual_target_window_or_early_sp_cost() {
    for scope in [1, 4] {
        let skill = skill_def(1, 20, scope);
        let data = GameData {
            actors: vec![],
            items: vec![],
            skills: vec![skill.clone()],
        };
        let mut battle = build_party2();
        battle.begin_actor_commands();
        battle.skills = vec![skill];
        battle.members[0].known_skills = vec![1];
        battle.menu = MenuLevel::Skill;
        let sp = battle.members[0].sp;
        skill_menu(&press_enter(), &data, &mut battle);
        assert!(
            matches!(
                battle.members[0].command,
                Some(Command::Skill {
                    skill_id: 1,
                    target: 0
                })
            ),
            "scope {scope}"
        );
        assert_eq!(battle.members[0].sp, sp);
        assert_eq!(battle.turn, 1);
        assert!(battle.menu == MenuLevel::Command);
    }
}

#[test]
fn group_medicine_commits_immediately_and_undo_does_not_consume_it() {
    let mut item = medicine(50);
    item.scope = 1;
    let id = item.id;
    let data = GameData {
        actors: vec![],
        items: vec![item],
        skills: vec![],
    };
    let mut inventory = Inventory::default();
    inventory.add_item(id, 2);
    let mut battle = build_party2();
    battle.begin_actor_commands();
    battle.menu = MenuLevel::Item;
    item_menu(&press_enter(), &data, &inventory, &mut battle);
    assert!(
        matches!(battle.members[0].command, Some(Command::Item { item_id, target: 0 }) if item_id == id)
    );
    assert_eq!(battle.turn, 1);
    assert!(battle.menu == MenuLevel::Command);
    battle.undo_choice();
    assert!(battle.members[0].command.is_none());
    assert_eq!(inventory.count(id), 2);
}

#[test]
fn a_weapon_restricted_skill_remains_listed_but_cannot_open_target_selection() {
    let mut battle = build_party2();
    battle.attributes = crate::gamedata::attribute_definitions().to_vec();
    battle.members[0].weapon_attributes.clear();
    battle.members[0].known_skills = vec![1];
    let mut skill = skill_def(1, 20, 0);
    skill.attributes = vec![1];
    battle.skills = vec![skill.clone()];
    let data = GameData {
        actors: vec![],
        items: vec![],
        skills: vec![skill],
    };
    battle.menu = MenuLevel::Skill;
    skill_menu(&press_enter(), &data, &mut battle);
    assert!(battle.menu == MenuLevel::Skill);
    assert!(matches!(battle.pending_se.last(), Some(BattleSe::Buzzer)));
    assert!(battle.pending_skill.is_none());
}

#[test]
fn a_silenced_skill_stays_listed_but_cannot_be_confirmed() {
    let mut battle = build_party2();
    battle.states = crate::assets::load_ron(&format!("{}/states.ron", crate::assets::asset_root()));
    battle.members[0].states = vec![(4, 0)];
    battle.members[0].known_skills = vec![1];
    let skill = skill_def(1, 20, 0);
    let data = GameData {
        actors: vec![],
        items: vec![],
        skills: vec![skill.clone()],
    };
    battle.skills = vec![skill];
    battle.menu = MenuLevel::Skill;
    assert_eq!(
        skill_choices(&data, &[1], battle.members[0].equipment_effects).len(),
        1
    );
    skill_menu(&press_enter(), &data, &mut battle);
    assert!(battle.menu == MenuLevel::Skill);
    assert!(matches!(battle.pending_se.last(), Some(BattleSe::Buzzer)));
    assert!(battle.pending_skill.is_none());
}

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
    assert_eq!(skill_choices(&data, &[1, 2], Default::default()).len(), 2);
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

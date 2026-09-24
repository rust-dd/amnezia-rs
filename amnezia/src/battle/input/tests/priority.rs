use super::*;

#[test]
fn decision_has_priority_over_cancel_in_each_actor_window() {
    for menu in [
        MenuLevel::Command,
        MenuLevel::Skill,
        MenuLevel::Item,
        MenuLevel::Target,
        MenuLevel::AllyTarget,
    ] {
        let mut battle = build_party2();
        battle.begin_actor_commands();
        battle.menu = menu;
        battle.cursor = 0;
        let skill = skill_def(2, 40, 3);
        battle.skills = vec![skill.clone()];
        battle.members[0].known_skills = vec![2];
        let data = GameData {
            actors: vec![],
            items: vec![medicine(112)],
            skills: vec![skill],
        };
        let mut inventory = Inventory::default();
        inventory.add_item(112, 1);
        if menu == MenuLevel::AllyTarget {
            battle.pending_item = Some(112);
        }
        let mut keys = press_enter();
        keys.press(KeyCode::Escape);
        match menu {
            MenuLevel::Command => command_menu(&keys, &mut battle),
            MenuLevel::Skill => skill_menu(&keys, &data, &mut battle),
            MenuLevel::Item => item_menu(&keys, &data, &inventory, &mut battle),
            MenuLevel::Target => target_menu(&keys, &mut battle),
            MenuLevel::AllyTarget => ally_target_menu(&keys, &inventory, &mut battle),
        }
        assert_eq!(
            battle.pending_se,
            [BattleSe::Decision],
            "menu {}",
            menu as usize
        );
        match menu {
            MenuLevel::Command => assert!(battle.menu == MenuLevel::Target),
            MenuLevel::Skill | MenuLevel::Item => assert!(battle.menu == MenuLevel::AllyTarget),
            _ => assert!(battle.members[0].command.is_some()),
        }
    }
}

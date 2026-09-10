use super::super::model::testkit::build_party2;
use super::*;

mod menus;

#[test]
fn a_disallowed_escape_buzzes_without_spending_the_party_turn() {
    let mut battle = build_party2();
    battle.phase = Phase::PartyCommand;
    battle.cursor = 2;
    battle.allow_escape = false;
    party_menu(&press_enter(), &mut battle);
    assert!(battle.phase == Phase::PartyCommand);
    assert_eq!(battle.outcome, None);
    assert_eq!(battle.pending_se, vec![BattleSe::Buzzer]);
}
use amnezia_data::{ItemDef, SkillDef};

fn medicine(id: u32) -> ItemDef {
    ItemDef {
        prevent_critical: false,
        raise_evasion: false,
        half_sp_cost: false,
        actor_set: Vec::new(),
        state_chance: 0,
        id,
        name: "Gyógyfű".into(),
        description: String::new(),
        item_type: MEDICINE,
        price: 0,
        recover_hp: 50,
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

fn skill_def(id: u32, power: u32, scope: u32) -> SkillDef {
    SkillDef {
        using_message1: String::new(),
        using_message2: String::new(),
        affect_stats: [false; 4],
        ignore_defense: false,
        id,
        name: "S".into(),
        description: String::new(),
        sp_cost: 3,
        power,
        hit: 0,
        skill_type: 0,
        failure_message: 0,
        scope,
        animation_id: 0,
        physical_rate: 0,
        magical_rate: 3,
        variance: 4,
        affect_hp: true,
        affect_sp: false,
        absorb: false,
        attributes: vec![],
        affected_states: vec![],
    }
}

#[test]
fn a_single_ally_skill_opens_the_ally_menu_then_commits_on_the_chosen_member() {
    let heal = skill_def(2, 40, 3);
    let data = GameData {
        actors: vec![],
        items: vec![],
        skills: vec![heal.clone()],
    };
    let mut battle = build_party2();
    battle.skills = vec![heal];
    let turn = battle.turn;
    battle.members[turn].known_skills = vec![2];
    battle.menu = MenuLevel::Skill;
    battle.cursor = 0;
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Enter);
    skill_menu(&keys, &data, &mut battle);
    assert!(battle.menu == MenuLevel::AllyTarget);
    assert_eq!(battle.pending_skill, Some(2));
    battle.cursor = 1;
    let mut confirm_keys = ButtonInput::<KeyCode>::default();
    confirm_keys.press(KeyCode::Enter);
    let inventory = Inventory::default();
    ally_target_menu(&confirm_keys, &inventory, &mut battle);
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Skill {
            skill_id: 2,
            target: 1
        })
    ));
}

#[test]
fn a_self_scope_skill_commits_immediately_on_the_caster() {
    let buff = skill_def(5, 30, 2);
    let data = GameData {
        actors: vec![],
        items: vec![],
        skills: vec![buff.clone()],
    };
    let mut battle = build_party2();
    battle.skills = vec![buff];
    let turn = battle.turn;
    battle.members[turn].known_skills = vec![5];
    battle.menu = MenuLevel::Skill;
    battle.cursor = 0;
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Enter);
    skill_menu(&keys, &data, &mut battle);
    assert!(battle.menu == MenuLevel::Command);
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Skill {
            skill_id: 5,
            target: 0
        })
    ));
}

#[test]
fn committing_an_item_selects_the_ally_without_consuming_it() {
    let data = GameData {
        actors: vec![],
        items: vec![medicine(50)],
        skills: vec![],
    };
    let mut inventory = Inventory::default();
    inventory.add_item(50, 2);
    let mut battle = build_party2();
    battle.menu = MenuLevel::Item;
    battle.cursor = 0;
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Enter);
    item_menu(&keys, &data, &inventory, &mut battle);
    assert!(battle.menu == MenuLevel::AllyTarget);
    assert_eq!(battle.pending_item, Some(50));
    assert_eq!(inventory.count(50), 2);
    battle.cursor = 1;
    let mut confirm_keys = ButtonInput::<KeyCode>::default();
    confirm_keys.press(KeyCode::Enter);
    ally_target_menu(&confirm_keys, &inventory, &mut battle);
    assert_eq!(inventory.count(50), 2);
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Item {
            item_id: 50,
            target: 1
        })
    ));
}

fn press_enter() -> ButtonInput<KeyCode> {
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Enter);
    keys
}

#[test]
fn the_actor_command_list_resolves_to_the_parsed_terms_without_flee() {
    let mut terms = Terms::default();
    terms.0.command_attack = "Attack".into();
    terms.0.command_skill = "Skill".into();
    terms.0.command_defend = "Defend".into();
    terms.0.command_item = "Item".into();
    terms.0.battle_fight = "Fight".into();
    terms.0.battle_auto = "Auto".into();
    terms.0.battle_escape = "Escape".into();
    assert_eq!(
        command_labels(&terms, None),
        ["Attack", "Skill", "Defend", "Item"]
    );
    assert!(!command_labels(&terms, None).contains(&"Escape".to_string()));
    assert_eq!(party_labels(&terms), ["Fight", "Auto", "Escape"]);
}

#[test]
fn command_labels_fall_back_to_the_hungarian_placeholders() {
    let terms = Terms::default();
    assert_eq!(
        command_labels(&terms, None),
        ["Támadás", "Képesség", "Védekezés", "Tárgy"]
    );
    assert_eq!(party_labels(&terms), ["Harc", "Auto", "Menekülés"]);
}

#[test]
fn the_command_menu_maps_defend_to_row_two_and_item_to_row_three() {
    let mut battle = build_party2();
    battle.begin_actor_commands();
    battle.cursor = 2;
    command_menu(&press_enter(), &mut battle);
    assert!(matches!(battle.members[0].command, Some(Command::Defend)));
    battle.cursor = 3;
    command_menu(&press_enter(), &mut battle);
    assert!(battle.menu == MenuLevel::Item);
}

#[test]
fn the_party_window_fight_option_enters_per_actor_command_entry() {
    let mut battle = build_party2();
    assert!(battle.phase == Phase::PartyCommand);
    battle.cursor = 0;
    party_menu(&press_enter(), &mut battle);
    assert!(battle.phase == Phase::Command);
    assert!(battle.menu == MenuLevel::Command);
}

#[test]
fn the_party_window_auto_option_orders_every_member_to_attack() {
    let mut battle = build_party2();
    battle.cursor = 1;
    party_menu(&press_enter(), &mut battle);
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Attack { .. })
    ));
    assert!(matches!(
        battle.members[1].command,
        Some(Command::Attack { .. })
    ));
    assert!(battle.phase == Phase::Resolve);
}

#[test]
fn the_party_window_escape_forfeits_the_turn_on_failure() {
    let mut battle = build_party2();
    battle.escape_chance = 0;
    battle.cursor = 2;
    party_menu(&press_enter(), &mut battle);
    assert!(battle.members.iter().all(|m| m.command.is_none()));
    assert!(battle.phase == Phase::Resolve);
    assert!(battle.log.iter().any(|l| l.contains("sikertelen")));
}

#[test]
fn navigating_the_battle_menu_plays_the_system_se() {
    let mut battle = build_party2();
    let mut down = ButtonInput::<KeyCode>::default();
    down.press(KeyCode::ArrowDown);
    party_menu(&down, &mut battle);
    assert!(battle.pending_se.contains(&BattleSe::Cursor));
    battle.pending_se.clear();
    battle.cursor = 0;
    party_menu(&press_enter(), &mut battle);
    assert!(battle.pending_se.contains(&BattleSe::Decision));
    battle.pending_se.clear();
    let mut esc = ButtonInput::<KeyCode>::default();
    esc.press(KeyCode::Escape);
    command_menu(&esc, &mut battle);
    assert!(battle.pending_se.contains(&BattleSe::Cancel));
}

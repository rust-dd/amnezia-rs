//! Command-phase input: navigate the command / skill / item / target menus,
//! commit each member's order, and trigger a flee. A thin keyboard driver over
//! the [`Battle`] model. The menu-row builders are `pub` so the UI renders the
//! exact same lists the selection indexes into.

use super::BattleOutcome;
use super::logic::usable_skills;
use super::model::{Battle, BattleSe, Command, MenuLevel, Phase};
use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;
use crate::terms::Terms;
use bevy::prelude::*;

/// The medicine category in the converted `ItemDef::item_type`.
const MEDICINE: u32 = 6;

/// The number of per-actor command options (RM2000 Attack / Skill / Defend /
/// Item, in cursor order) and party-level options (Fight / Auto / Escape). The
/// labels themselves come from the real terms via [`command_labels`] /
/// [`party_labels`]; the counts drive the cursor bounds.
pub const COMMAND_COUNT: usize = 4;
pub const PARTY_COUNT: usize = 3;

/// The per-actor command labels in cursor order, from the real RM2000 terms
/// (`command_attack` / `command_skill` / `command_defend` / `command_item` —
/// EasyRPG `Scene_Battle_Rpg2k`), each falling back to its Hungarian placeholder.
/// Defend precedes Item and there is no per-actor flee (escape is a party option).
pub fn command_labels(terms: &Terms) -> [String; COMMAND_COUNT] {
    let t = &terms.0;
    [
        terms.label(&t.command_attack, "Támadás"),
        terms.label(&t.command_skill, "Képesség"),
        terms.label(&t.command_defend, "Védekezés"),
        terms.label(&t.command_item, "Tárgy"),
    ]
}

/// The party-level option labels in cursor order (RM2000 `battle_fight` /
/// `battle_auto` / `battle_escape`), each falling back to its Hungarian placeholder.
pub fn party_labels(terms: &Terms) -> [String; PARTY_COUNT] {
    let t = &terms.0;
    [
        terms.label(&t.battle_fight, "Harc"),
        terms.label(&t.battle_auto, "Auto"),
        terms.label(&t.battle_escape, "Menekülés"),
    ]
}

/// Drive the command phases: the party-option window at round start, then the
/// per-actor menu levels once the player picks Fight.
pub fn command_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    mut inventory: ResMut<Inventory>,
    mut battle: ResMut<Battle>,
) {
    if !any_key(&keys) {
        return;
    }
    match battle.phase {
        Phase::PartyCommand => party_menu(&keys, &mut battle),
        Phase::Command => match battle.menu {
            MenuLevel::Command => command_menu(&keys, &mut battle),
            MenuLevel::Skill => skill_menu(&keys, &data, &mut battle),
            MenuLevel::Item => item_menu(&keys, &data, &inventory, &mut battle),
            MenuLevel::Target => target_menu(&keys, &mut battle),
            MenuLevel::AllyTarget => ally_target_menu(&keys, &mut inventory, &mut battle),
        },
        _ => {}
    }
}

/// The party-option window (RM2000 Fight / Auto / Escape): Fight drops into
/// per-actor command entry, Auto orders the whole party a basic attack and
/// resolves, Escape attempts to flee now.
fn party_menu(keys: &ButtonInput<KeyCode>, battle: &mut Battle) {
    move_cursor(keys, &mut battle.cursor, PARTY_COUNT);
    if !confirm(keys) {
        return;
    }
    match battle.cursor {
        0 => battle.begin_actor_commands(),
        1 => battle.auto_battle(),
        _ => escape(battle),
    }
}

/// The affordable offensive skills as `(skill_id, sp_cost, label)` for a caster
/// that `knows` the given skill ids and has `sp` points — the single source both
/// the menu and the UI read. Restricting to `known` keeps the list to the actor's
/// learned skills rather than the whole database.
pub fn skill_choices(data: &GameData, known: &[u32], sp: i32) -> Vec<(u32, u32, String)> {
    usable_skills(&data.skills, sp)
        .into_iter()
        .filter(|s| known.contains(&s.id))
        .map(|s| {
            (
                s.id,
                s.sp_cost,
                format!("{} (SP {})", i18n::tr(&s.name), s.sp_cost),
            )
        })
        .collect()
}

/// The usable medicine items as `(item_id, label)` held in `inventory`.
pub fn item_choices(data: &GameData, inventory: &Inventory) -> Vec<(u32, String)> {
    data.items
        .iter()
        .filter(|i| i.item_type == MEDICINE && inventory.count(i.id) > 0)
        .map(|i| {
            (
                i.id,
                format!("{} ×{}", i18n::tr(&i.name), inventory.count(i.id)),
            )
        })
        .collect()
}

fn command_menu(keys: &ButtonInput<KeyCode>, battle: &mut Battle) {
    if keys.just_pressed(KeyCode::Escape) {
        battle.undo_choice();
        return;
    }
    move_cursor(keys, &mut battle.cursor, COMMAND_COUNT);
    if !confirm(keys) {
        return;
    }
    match battle.cursor {
        0 => open_target(battle, None),
        1 => enter(battle, MenuLevel::Skill),
        2 => battle.commit(Command::Defend),
        _ => enter(battle, MenuLevel::Item),
    }
}

fn skill_menu(keys: &ButtonInput<KeyCode>, data: &GameData, battle: &mut Battle) {
    if keys.just_pressed(KeyCode::Escape) {
        enter(battle, MenuLevel::Command);
        return;
    }
    let known = battle.members[battle.turn].known_skills.clone();
    let sp = battle.members[battle.turn].sp;
    let choices = skill_choices(data, &known, sp);
    move_cursor(keys, &mut battle.cursor, choices.len());
    if confirm(keys)
        && let Some(&(skill_id, _, _)) = choices.get(battle.cursor)
    {
        let scope = battle
            .skills
            .iter()
            .find(|s| s.id == skill_id)
            .map(|s| s.scope);
        match scope {
            // Self (2) and all-allies (4) need no target pick: resolve applies to
            // the caster / loops the party, so commit at once. Single-ally (3)
            // opens the ally target menu; enemy scopes (0/1) the enemy one.
            Some(2 | 4) => {
                let target = battle.turn;
                battle.commit(Command::Skill { skill_id, target });
            }
            Some(3) => open_ally_target(battle, Some(skill_id), None),
            _ => open_target(battle, Some(skill_id)),
        }
    }
}

fn item_menu(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &Inventory,
    battle: &mut Battle,
) {
    if keys.just_pressed(KeyCode::Escape) {
        enter(battle, MenuLevel::Command);
        return;
    }
    let choices = item_choices(data, inventory);
    move_cursor(keys, &mut battle.cursor, choices.len());
    if confirm(keys)
        && let Some(&(id, _)) = choices.get(battle.cursor)
    {
        open_ally_target(battle, None, Some(id));
    }
}

fn target_menu(keys: &ButtonInput<KeyCode>, battle: &mut Battle) {
    let living = battle.living_enemies();
    if living.is_empty() {
        enter(battle, MenuLevel::Command);
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        let back = if battle.pending_skill.is_some() {
            MenuLevel::Skill
        } else {
            MenuLevel::Command
        };
        enter(battle, back);
        return;
    }
    move_cursor(keys, &mut battle.cursor, living.len());
    if confirm(keys) {
        let target = living[battle.cursor.min(living.len() - 1)];
        let command = match battle.pending_skill {
            Some(skill_id) => Command::Skill { skill_id, target },
            None => Command::Attack { target },
        };
        battle.commit(command);
    }
}

/// The ally target menu: move the cursor over living party members and, on
/// confirm, commit the pending ally-scope skill or item against the chosen one. A
/// cancel returns to the Skill or Item menu the selection came from.
fn ally_target_menu(keys: &ButtonInput<KeyCode>, inventory: &mut Inventory, battle: &mut Battle) {
    let living = battle.living_members();
    if living.is_empty() {
        enter(battle, MenuLevel::Command);
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        let back = if battle.pending_skill.is_some() {
            MenuLevel::Skill
        } else {
            MenuLevel::Item
        };
        enter(battle, back);
        return;
    }
    move_cursor(keys, &mut battle.cursor, living.len());
    if confirm(keys) {
        let target = living[battle.cursor.min(living.len() - 1)];
        if let Some(skill_id) = battle.pending_skill {
            battle.commit(Command::Skill { skill_id, target });
        } else if let Some(item_id) = battle.pending_item {
            inventory.remove_item(item_id, 1);
            battle.commit(Command::Item { item_id, target });
        }
    }
}

/// Enter the target menu for an attack (`None`) or a chosen skill (its id).
fn open_target(battle: &mut Battle, skill: Option<u32>) {
    if battle.living_enemies().is_empty() {
        return;
    }
    battle.pending_skill = skill;
    battle.menu = MenuLevel::Target;
    battle.cursor = 0;
}

/// Enter the ally target menu for a chosen ally-scope skill or item; exactly one
/// of `skill` / `item` is `Some`. The cursor then ranges over living members.
fn open_ally_target(battle: &mut Battle, skill: Option<u32>, item: Option<u32>) {
    if battle.living_members().is_empty() {
        return;
    }
    battle.pending_skill = skill;
    battle.pending_item = item;
    battle.menu = MenuLevel::AllyTarget;
    battle.cursor = 0;
}

/// Switch to `level`, resetting the cursor.
fn enter(battle: &mut Battle, level: MenuLevel) {
    battle.menu = level;
    battle.cursor = 0;
}

/// Attempt a party escape from the party-option window: play the escape SE, then
/// on success end the fight, or on failure forfeit the whole party's turn (the
/// enemies act) and resolve — RM2000 `ProcessSceneActionEscape`'s failure path.
fn escape(battle: &mut Battle) {
    // The escape SE plays on the attempt (RM2000 `SFX_Escape`), drained like the
    // per-hit effects.
    battle.pending_se.push(BattleSe::Escape);
    if battle.attempt_escape() {
        battle.finish(BattleOutcome::Escape);
    } else {
        let line = i18n::tr(&battle.text.escape_failure);
        battle.log.push(line);
        battle.begin_resolve();
    }
}

/// Move a wrapping cursor over `len` rows, clamped when the list shrank.
fn move_cursor(keys: &ButtonInput<KeyCode>, cursor: &mut usize, len: usize) {
    if len == 0 {
        *cursor = 0;
        return;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        *cursor = (*cursor + 1) % len;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        *cursor = (*cursor + len - 1) % len;
    }
    *cursor = (*cursor).min(len - 1);
}

/// Whether any key the battle menus react to was just pressed.
fn any_key(keys: &ButtonInput<KeyCode>) -> bool {
    const RELEVANT: [KeyCode; 7] = [
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::Space,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ];
    RELEVANT.iter().any(|k| keys.just_pressed(*k))
}

/// The action key: Space or Enter, as the dialogue and shop boxes use.
fn confirm(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
}

#[cfg(test)]
mod tests {
    use super::super::model::testkit::build_party2;
    use super::*;
    use amnezia_data::{ItemDef, SkillDef};

    fn medicine(id: u32) -> ItemDef {
        ItemDef {
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
            id,
            name: "S".into(),
            description: String::new(),
            sp_cost: 3,
            power,
            hit: 0,
            skill_type: 0,
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
        let heal = skill_def(2, 40, 3); // scope 3: single ally
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
        // Point the cursor at the second member and confirm.
        battle.cursor = 1;
        let mut confirm_keys = ButtonInput::<KeyCode>::default();
        confirm_keys.press(KeyCode::Enter);
        let mut inventory = Inventory::default();
        ally_target_menu(&confirm_keys, &mut inventory, &mut battle);
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
        let buff = skill_def(5, 30, 2); // scope 2: self
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
        // No target menu: committed at once against the caster (member 0).
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
    fn committing_an_item_targets_the_chosen_ally_and_consumes_one() {
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
        // Selecting the item opens the ally menu; nothing is consumed yet.
        assert!(battle.menu == MenuLevel::AllyTarget);
        assert_eq!(battle.pending_item, Some(50));
        assert_eq!(inventory.count(50), 2);
        // Confirm the item on the second member.
        battle.cursor = 1;
        let mut confirm_keys = ButtonInput::<KeyCode>::default();
        confirm_keys.press(KeyCode::Enter);
        ally_target_menu(&confirm_keys, &mut inventory, &mut battle);
        assert_eq!(inventory.count(50), 1); // one consumed on confirm
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
        // With parsed terms the labels resolve to the real RM2000 command terms,
        // in RM2000 order — Defend before Item — and no per-actor flee row.
        let mut terms = Terms::default();
        terms.0.command_attack = "Attack".into();
        terms.0.command_skill = "Skill".into();
        terms.0.command_defend = "Defend".into();
        terms.0.command_item = "Item".into();
        terms.0.battle_fight = "Fight".into();
        terms.0.battle_auto = "Auto".into();
        terms.0.battle_escape = "Escape".into();
        assert_eq!(
            command_labels(&terms),
            ["Attack", "Skill", "Defend", "Item"]
        );
        assert!(!command_labels(&terms).contains(&"Escape".to_string()));
        // Escape lives on the party-level window instead.
        assert_eq!(party_labels(&terms), ["Fight", "Auto", "Escape"]);
    }

    #[test]
    fn command_labels_fall_back_to_the_hungarian_placeholders() {
        // No parsed terms: the chrome still reads naturally in Hungarian.
        let terms = Terms::default();
        assert_eq!(
            command_labels(&terms),
            ["Támadás", "Képesség", "Védekezés", "Tárgy"]
        );
        assert_eq!(party_labels(&terms), ["Harc", "Auto", "Menekülés"]);
    }

    #[test]
    fn the_command_menu_maps_defend_to_row_two_and_item_to_row_three() {
        let mut battle = build_party2();
        battle.begin_actor_commands(); // drop into per-actor entry
        // Row 2 = Védekezés -> commit Defend for the first member.
        battle.cursor = 2;
        command_menu(&press_enter(), &mut battle);
        assert!(matches!(battle.members[0].command, Some(Command::Defend)));
        // Row 3 = Tárgy -> the second member opens the item menu (no flee row).
        battle.cursor = 3;
        command_menu(&press_enter(), &mut battle);
        assert!(battle.menu == MenuLevel::Item);
    }

    #[test]
    fn the_party_window_fight_option_enters_per_actor_command_entry() {
        let mut battle = build_party2();
        assert!(battle.phase == Phase::PartyCommand);
        battle.cursor = 0; // Harc
        party_menu(&press_enter(), &mut battle);
        assert!(battle.phase == Phase::Command);
        assert!(battle.menu == MenuLevel::Command);
    }

    #[test]
    fn the_party_window_auto_option_orders_every_member_to_attack() {
        let mut battle = build_party2(); // two members, one foe
        battle.cursor = 1; // Auto
        party_menu(&press_enter(), &mut battle);
        assert!(matches!(
            battle.members[0].command,
            Some(Command::Attack { .. })
        ));
        assert!(matches!(
            battle.members[1].command,
            Some(Command::Attack { .. })
        ));
        assert!(battle.phase == Phase::Resolve); // the round is resolving
    }

    #[test]
    fn the_party_window_escape_forfeits_the_turn_on_failure() {
        let mut battle = build_party2();
        battle.escape_chance = 0; // a 0% chance always fails
        battle.cursor = 2; // Menekülés
        party_menu(&press_enter(), &mut battle);
        // No member acted, yet the round resolves so the enemies act.
        assert!(battle.members.iter().all(|m| m.command.is_none()));
        assert!(battle.phase == Phase::Resolve);
        assert!(battle.log.iter().any(|l| l.contains("sikertelen")));
    }
}

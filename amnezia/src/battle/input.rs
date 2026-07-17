//! Command-phase input: navigate the command / skill / item / target menus,
//! commit each member's order, and trigger a flee. A thin keyboard driver over
//! the [`Battle`] model. The menu-row builders are `pub` so the UI renders the
//! exact same lists the selection indexes into.

use super::BattleOutcome;
use super::logic::usable_skills;
use super::model::{Battle, Command, MenuLevel, Phase};
use crate::gamedata::GameData;
use crate::i18n;
use crate::state::Inventory;
use bevy::prelude::*;

/// The medicine category in the converted `ItemDef::item_type`.
const MEDICINE: u32 = 6;

/// The top-level command options, in cursor order.
pub const COMMAND_LABELS: [&str; 5] = ["Támadás", "Képesség", "Tárgy", "Védekezés", "Menekülés"];

/// Drive the command phase: dispatch the keyboard to the active menu level.
pub fn command_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    mut inventory: ResMut<Inventory>,
    mut battle: ResMut<Battle>,
) {
    if battle.phase != Phase::Command || !any_key(&keys) {
        return;
    }
    match battle.menu {
        MenuLevel::Command => command_menu(&keys, &mut battle),
        MenuLevel::Skill => skill_menu(&keys, &data, &mut battle),
        MenuLevel::Item => item_menu(&keys, &data, &inventory, &mut battle),
        MenuLevel::Target => target_menu(&keys, &mut battle),
        MenuLevel::AllyTarget => ally_target_menu(&keys, &mut inventory, &mut battle),
    }
}

/// The affordable offensive skills as `(skill_id, sp_cost, label)` for the caster
/// with `sp` points — the single source both the menu and the UI read.
pub fn skill_choices(data: &GameData, sp: i32) -> Vec<(u32, u32, String)> {
    usable_skills(&data.skills, sp)
        .into_iter()
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
    move_cursor(keys, &mut battle.cursor, COMMAND_LABELS.len());
    if !confirm(keys) {
        return;
    }
    match battle.cursor {
        0 => open_target(battle, None),
        1 => enter(battle, MenuLevel::Skill),
        2 => enter(battle, MenuLevel::Item),
        3 => battle.commit(Command::Defend),
        _ => flee(battle),
    }
}

fn skill_menu(keys: &ButtonInput<KeyCode>, data: &GameData, battle: &mut Battle) {
    if keys.just_pressed(KeyCode::Escape) {
        enter(battle, MenuLevel::Command);
        return;
    }
    let sp = battle.members[battle.turn].sp;
    let choices = skill_choices(data, sp);
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

fn flee(battle: &mut Battle) {
    if battle.attempt_flee() {
        battle.finish(BattleOutcome::Escape);
    } else {
        battle.log.push("Menekülés sikertelen!".to_string());
        battle.commit(Command::Nothing);
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
            physical_rate: 0,
            magical_rate: 3,
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
}

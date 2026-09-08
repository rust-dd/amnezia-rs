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
    if move_cursor(keys, &mut battle.cursor, PARTY_COUNT) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if !confirm(keys) {
        return;
    }
    if battle.cursor == 2 && !battle.allow_escape {
        battle.pending_se.push(BattleSe::Buzzer);
        return;
    }
    battle.pending_se.push(BattleSe::Decision);
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
        battle.pending_se.push(BattleSe::Cancel);
        battle.undo_choice();
        return;
    }
    if move_cursor(keys, &mut battle.cursor, COMMAND_COUNT) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if !confirm(keys) {
        return;
    }
    battle.pending_se.push(BattleSe::Decision);
    match battle.cursor {
        0 => open_target(battle, None),
        1 => enter(battle, MenuLevel::Skill),
        2 => battle.commit(Command::Defend),
        _ => enter(battle, MenuLevel::Item),
    }
}

fn skill_menu(keys: &ButtonInput<KeyCode>, data: &GameData, battle: &mut Battle) {
    if keys.just_pressed(KeyCode::Escape) {
        battle.pending_se.push(BattleSe::Cancel);
        enter(battle, MenuLevel::Command);
        return;
    }
    let known = battle.members[battle.turn].known_skills.clone();
    let sp = battle.members[battle.turn].sp;
    let choices = skill_choices(data, &known, sp);
    if move_cursor(keys, &mut battle.cursor, choices.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        let Some(&(skill_id, _, _)) = choices.get(battle.cursor) else {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        };
        battle.pending_se.push(BattleSe::Decision);
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
        battle.pending_se.push(BattleSe::Cancel);
        enter(battle, MenuLevel::Command);
        return;
    }
    let choices = item_choices(data, inventory);
    if move_cursor(keys, &mut battle.cursor, choices.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        let Some(&(id, _)) = choices.get(battle.cursor) else {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        };
        battle.pending_se.push(BattleSe::Decision);
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
        battle.pending_se.push(BattleSe::Cancel);
        let back = if battle.pending_skill.is_some() {
            MenuLevel::Skill
        } else {
            MenuLevel::Command
        };
        enter(battle, back);
        return;
    }
    if move_cursor(keys, &mut battle.cursor, living.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        battle.pending_se.push(BattleSe::Decision);
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
        battle.pending_se.push(BattleSe::Cancel);
        let back = if battle.pending_skill.is_some() {
            MenuLevel::Skill
        } else {
            MenuLevel::Item
        };
        enter(battle, back);
        return;
    }
    if move_cursor(keys, &mut battle.cursor, living.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        battle.pending_se.push(BattleSe::Decision);
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

/// Move a wrapping cursor over `len` rows, clamped when the list shrank. Returns
/// whether the cursor actually moved, so the caller can play the cursor SE.
fn move_cursor(keys: &ButtonInput<KeyCode>, cursor: &mut usize, len: usize) -> bool {
    if len == 0 {
        *cursor = 0;
        return false;
    }
    let before = *cursor;
    if keys.just_pressed(KeyCode::ArrowDown) {
        *cursor = (*cursor + 1) % len;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        *cursor = (*cursor + len - 1) % len;
    }
    *cursor = (*cursor).min(len - 1);
    *cursor != before
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
mod tests;

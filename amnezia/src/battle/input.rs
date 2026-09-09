//! Command-phase input: navigate the command / skill / item / target menus,
//! commit each member's order, and trigger a flee. A thin keyboard driver over
//! the [`Battle`] model. The menu-row builders are `pub` so the UI renders the
//! exact same lists the selection indexes into.

use super::BattleOutcome;
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
    inventory: Res<Inventory>,
    mut battle: ResMut<Battle>,
) {
    if !any_key(&keys) || battle.events.blocks_action() {
        return;
    }
    match battle.phase {
        Phase::PartyCommand => party_menu(&keys, &mut battle),
        Phase::Command => match battle.menu {
            MenuLevel::Command => command_menu(&keys, &mut battle),
            MenuLevel::Skill => skill_menu(&keys, &data, &mut battle),
            MenuLevel::Item => item_menu(&keys, &data, &inventory, &mut battle),
            MenuLevel::Target => target_menu(&keys, &mut battle),
            MenuLevel::AllyTarget => ally_target_menu(&keys, &inventory, &mut battle),
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

/// Learned skills in database order, including disabled entries.
pub fn skill_choices(data: &GameData, known: &[u32], _sp: i32) -> Vec<(u32, u32, String)> {
    data.skills
        .iter()
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

pub(super) fn item_enabled(item: &amnezia_data::ItemDef) -> bool {
    item.item_type == MEDICINE && !item.only_field
}

/// Inventory entries in database order; unusable items stay visible but disabled.
pub fn item_choices(data: &GameData, inventory: &Inventory) -> Vec<(u32, String)> {
    data.items
        .iter()
        .filter(|i| inventory.count(i.id) > 0)
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
    if move_grid_cursor(keys, &mut battle.cursor, choices.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        let Some(&(skill_id, _, _)) = choices.get(battle.cursor) else {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        };
        if !data.skills.iter().any(|skill| {
            skill.id == skill_id
                && battle.skill_usable_by(super::model::Source::Party(battle.turn), skill)
        }) {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        }
        battle.pending_se.push(BattleSe::Decision);
        let scope = battle
            .skills
            .iter()
            .find(|s| s.id == skill_id)
            .map(|s| s.scope);
        match scope {
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
    if move_grid_cursor(keys, &mut battle.cursor, choices.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        let Some(&(id, _)) = choices.get(battle.cursor) else {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        };
        if !data
            .items
            .iter()
            .any(|item| item.id == id && item_enabled(item))
        {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        }
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

/// The ally target menu: move the cursor over all party members and, on
/// confirm, commit the pending ally-scope skill or item against the chosen one. A
/// cancel returns to the Skill or Item menu the selection came from.
fn ally_target_menu(keys: &ButtonInput<KeyCode>, inventory: &Inventory, battle: &mut Battle) {
    if battle.members.is_empty() {
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
    if move_cursor(keys, &mut battle.cursor, battle.members.len()) {
        battle.pending_se.push(BattleSe::Cursor);
    }
    if confirm(keys) {
        if battle.pending_item.is_some_and(|id| !inventory.has(id)) {
            battle.pending_se.push(BattleSe::Buzzer);
            return;
        }
        battle.pending_se.push(BattleSe::Decision);
        let target = battle.cursor.min(battle.members.len() - 1);
        if let Some(skill_id) = battle.pending_skill {
            battle.commit(Command::Skill { skill_id, target });
        } else if let Some(item_id) = battle.pending_item {
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
    enter(battle, MenuLevel::Target);
    battle.cursor = 0;
}

/// Enter the ally target menu for a chosen ally-scope skill or item; exactly one
/// of `skill` / `item` is `Some`. Fallen members remain selectable for revival.
fn open_ally_target(battle: &mut Battle, skill: Option<u32>, item: Option<u32>) {
    if battle.members.is_empty() {
        return;
    }
    battle.pending_skill = skill;
    battle.pending_item = item;
    enter(battle, MenuLevel::AllyTarget);
    battle.cursor = 0;
}

/// Preserve each window's selection when opening or cancelling a submenu.
fn enter(battle: &mut Battle, level: MenuLevel) {
    battle.menu_cursors[battle.menu as usize] = battle.cursor;
    battle.menu = level;
    battle.cursor = battle.menu_cursors[level as usize];
}

/// Attempt a party escape from the party-option window: play the escape SE, then
/// on success end the fight, or on failure forfeit the whole party's turn (the
/// enemies act) and resolve — RM2000 `ProcessSceneActionEscape`'s failure path.
fn escape(battle: &mut Battle) {
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

fn move_grid_cursor(keys: &ButtonInput<KeyCode>, cursor: &mut usize, len: usize) -> bool {
    let before = *cursor;
    *cursor = (*cursor).min(len.saturating_sub(1));
    if keys.just_pressed(KeyCode::ArrowDown) && *cursor + 2 < len {
        *cursor += 2;
    }
    if keys.just_pressed(KeyCode::ArrowUp) && *cursor >= 2 {
        *cursor -= 2;
    }
    if keys.just_pressed(KeyCode::ArrowRight) && *cursor + 1 < len {
        *cursor += 1;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) && *cursor > 0 {
        *cursor -= 1;
    }
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

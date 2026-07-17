//! Command-phase input: navigate the command / skill / item / target menus,
//! commit each member's order, and trigger a flee. A thin keyboard driver over
//! the [`Battle`] model. The menu-row builders are `pub` so the UI renders the
//! exact same lists the selection indexes into.

use super::logic::usable_skills;
use super::model::{Battle, Command, MenuLevel, Phase};
use super::BattleOutcome;
use crate::gamedata::GameData;
use crate::state::Inventory;
use bevy::prelude::*;

/// The medicine category in the converted `ItemDef::item_type`.
const MEDICINE: u32 = 6;

/// The top-level command options, in cursor order.
pub const COMMAND_LABELS: [&str; 5] =
    ["Támadás", "Képesség", "Tárgy", "Védekezés", "Menekülés"];

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
        MenuLevel::Item => item_menu(&keys, &data, &mut inventory, &mut battle),
        MenuLevel::Target => target_menu(&keys, &mut battle),
    }
}

/// The affordable offensive skills as `(power, sp_cost, label)` for the caster
/// with `sp` points — the single source both the menu and the UI read.
pub fn skill_choices(data: &GameData, sp: i32) -> Vec<(u32, u32, String)> {
    usable_skills(&data.skills, sp)
        .into_iter()
        .map(|s| (s.power, s.sp_cost, format!("{} (SP {})", s.name, s.sp_cost)))
        .collect()
}

/// The usable medicine items as `(item_id, label)` held in `inventory`.
pub fn item_choices(data: &GameData, inventory: &Inventory) -> Vec<(u32, String)> {
    data.items
        .iter()
        .filter(|i| i.item_type == MEDICINE && inventory.count(i.id) > 0)
        .map(|i| (i.id, format!("{} ×{}", i.name, inventory.count(i.id))))
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
        && let Some(&(power, cost, _)) = choices.get(battle.cursor)
    {
        open_target(battle, Some((power, cost)));
    }
}

fn item_menu(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &mut Inventory,
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
        inventory.remove_item(id, 1);
        battle.commit(Command::Item);
    }
}

fn target_menu(keys: &ButtonInput<KeyCode>, battle: &mut Battle) {
    let living = battle.living_enemies();
    if living.is_empty() {
        enter(battle, MenuLevel::Command);
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        let back = if battle.pending_skill.is_some() { MenuLevel::Skill } else { MenuLevel::Command };
        enter(battle, back);
        return;
    }
    move_cursor(keys, &mut battle.cursor, living.len());
    if confirm(keys) {
        let target = living[battle.cursor.min(living.len() - 1)];
        let command = match battle.pending_skill {
            Some((power, cost)) => Command::Skill { power, cost, target },
            None => Command::Attack { target },
        };
        battle.commit(command);
    }
}

/// Enter the target menu for an attack (`None`) or a chosen `(power, cost)` skill.
fn open_target(battle: &mut Battle, skill: Option<(u32, u32)>) {
    if battle.living_enemies().is_empty() {
        return;
    }
    battle.pending_skill = skill;
    battle.menu = MenuLevel::Target;
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

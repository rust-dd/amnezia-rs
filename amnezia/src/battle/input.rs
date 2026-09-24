//! Command-phase input: navigate the command / skill / item / target menus,
//! commit each member's order, and trigger a flee. A thin keyboard driver over
//! the [`Battle`] model. The menu-row builders are `pub` so the UI renders the
//! exact same lists the selection indexes into.

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

/// An enabled actor override keeps even an empty skill label, as Tray requires.
pub fn command_labels(
    terms: &Terms,
    actor: Option<&amnezia_data::ActorDef>,
) -> [String; COMMAND_COUNT] {
    let t = &terms.0;
    [
        terms.label(&t.command_attack, "Támadás"),
        actor.filter(|actor| actor.rename_skill).map_or_else(
            || terms.label(&t.command_skill, "Képesség"),
            |actor| i18n::tr(&actor.skill_name),
        ),
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
    if (!confirm(&keys) && !keys.just_pressed(KeyCode::Escape)) || battle.events.blocks_action() {
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
        _ => battle.begin_escape(super::message::Controls::from_keys(keys)),
    }
}

/// Learned skills in database order, including disabled entries.
pub fn skill_choices(
    data: &GameData,
    known: &[u32],
    effects: crate::equipment::EquipmentEffects,
) -> Vec<(u32, u32, String)> {
    data.skills
        .iter()
        .filter(|s| known.contains(&s.id))
        .map(|s| {
            let cost = effects.skill_cost(s.sp_cost);
            (s.id, cost, format!("{} (SP {})", i18n::tr(&s.name), cost))
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
    if !confirm(keys) && keys.just_pressed(KeyCode::Escape) {
        battle.pending_se.push(BattleSe::Cancel);
        battle.undo_choice();
        return;
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
    if !confirm(keys) && keys.just_pressed(KeyCode::Escape) {
        battle.pending_se.push(BattleSe::Cancel);
        enter(battle, MenuLevel::Command);
        return;
    }
    let known = battle.members[battle.turn].known_skills.clone();
    let choices = skill_choices(data, &known, battle.members[battle.turn].equipment_effects);
    if confirm(keys) {
        remember_skill_cursor(battle);
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
            Some(1) => {
                let target = battle.living_enemies().first().copied().unwrap_or(0);
                battle.commit(Command::Skill { skill_id, target });
            }
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
    if !confirm(keys) && keys.just_pressed(KeyCode::Escape) {
        battle.pending_se.push(BattleSe::Cancel);
        enter(battle, MenuLevel::Command);
        return;
    }
    let choices = item_choices(data, inventory);
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
        if data.item(id).is_some_and(|item| item.scope == 1) {
            battle.commit(Command::Item {
                item_id: id,
                target: battle.turn,
            });
        } else {
            open_ally_target(battle, None, Some(id));
        }
    }
}

fn target_menu(keys: &ButtonInput<KeyCode>, battle: &mut Battle) {
    let living = battle.living_enemies();
    if living.is_empty() {
        enter(battle, MenuLevel::Command);
        return;
    }
    if !confirm(keys) && keys.just_pressed(KeyCode::Escape) {
        battle.pending_se.push(BattleSe::Cancel);
        let back = if battle.pending_skill.is_some() {
            MenuLevel::Skill
        } else {
            MenuLevel::Command
        };
        enter(battle, back);
        return;
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
    if !confirm(keys) && keys.just_pressed(KeyCode::Escape) {
        battle.pending_se.push(BattleSe::Cancel);
        let back = if battle.pending_skill.is_some() {
            MenuLevel::Skill
        } else {
            MenuLevel::Item
        };
        enter(battle, back);
        return;
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
    battle.pending_item = None;
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
    if battle.menu == MenuLevel::Skill {
        remember_skill_cursor(battle);
    }
    battle.menu_cursors[battle.menu as usize] = battle.cursor;
    if level == MenuLevel::Skill && battle.menu == MenuLevel::Command {
        let remembered = battle.skill_cursors.get(battle.turn).copied().unwrap_or(0);
        let count = battle.members.get(battle.turn).map_or(0, |actor| {
            battle
                .skills
                .iter()
                .filter(|skill| actor.known_skills.contains(&skill.id))
                .count()
        });
        battle.menu_cursors[level as usize] = remembered.min(count.saturating_sub(1));
    }
    battle.menu = level;
    battle.cursor = battle.menu_cursors[level as usize];
}

fn remember_skill_cursor(battle: &mut Battle) {
    battle.menu_cursors[MenuLevel::Skill as usize] = battle.cursor;
    if let Some(cursor) = battle.skill_cursors.get_mut(battle.turn) {
        *cursor = battle.cursor;
    }
}

/// The action key: Space or Enter, as the dialogue and shop boxes use.
fn confirm(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
}

#[cfg(test)]
mod tests;

//! The in-game menu: the RM2000 default main menu, a windowskin overlay toggled
//! with Escape. The top level is a command window — Tárgy (Item), Képesség
//! (Skill), Felszerelés (Equipment), Mentés (Save), Kilépés (End Game) — beside a
//! party status window listing each member's name, level, and HP/SP. Confirming a
//! command drills into its flow: the held-item list and field-use for Item; a
//! party-member prompt then a skill list (and a field heal) for Skill; a
//! read-only equipment view for Equipment; a save request for Save; and a
//! return-to-title confirmation for End Game. Selecting a member from the party
//! window (→ from the command list) opens that member's status detail.
//!
//! The Skill list shows only the chosen caster's known skills — the actor
//! `learnings` at or below its current level (their SP is what a cast spends).
//! Field skill-use is limited to HP-recovery ally skills healing a flat `power`,
//! and equipment is view-only — both noted where they live ([`skills`], [`equip`]).
//!
//! State and input live in [`input`]; the movement/interpreter pause guard that
//! freezes the world while the menu is open (keyed on [`MenuOpen`]) is wired by
//! the main session. This module owns the resources, the panel plugin, and the
//! shared screen types.

mod command;
mod derive;
mod equip;
mod input;
mod items;
mod nav;
mod render;
mod skills;
mod status;
mod use_item;
mod view;

#[cfg(test)]
mod testkit;

use bevy::prelude::*;

/// Whether the menu overlay is showing. The pause guard reads this; this module
/// owns the toggle (Escape).
#[derive(Resource, Default)]
pub struct MenuOpen(pub bool);

/// Whether the player may open the main menu (RM2000 `ChangeMainMenuAccess`,
/// opcode 11960). Defaults enabled; a cutscene disables it to lock the menu shut
/// and re-enables it afterwards. Only opening is gated — a menu already up stays
/// usable, matching RPG_RT's `SetAllowMenu`.
#[derive(Resource)]
pub struct MenuAccess(pub bool);

impl Default for MenuAccess {
    fn default() -> Self {
        Self(true)
    }
}

/// What a member-select prompt is choosing a party member *for*: to cast a skill,
/// to inspect equipment, or to view status detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberAction {
    Skill,
    Equip,
    Status,
}

/// The menu's active screen. The top-level `Command` list uses the cursor stored
/// in [`MenuState`]; every drilled-in screen carries its own cursor (and the
/// member/item/skill it is bound to), so backing out restores the command cursor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum MenuScreen {
    #[default]
    Command,
    ItemList {
        cursor: usize,
    },
    ItemTarget {
        item_id: u32,
        cursor: usize,
    },
    MemberSelect {
        action: MemberAction,
        cursor: usize,
    },
    SkillList {
        member: usize,
        cursor: usize,
    },
    SkillTarget {
        member: usize,
        skill_id: u32,
        cursor: usize,
    },
    Equip {
        member: usize,
    },
    Status {
        member: usize,
    },
    Saved,
    EndGame {
        cursor: usize,
    },
}

/// The command-list cursor (only meaningful on [`MenuScreen::Command`]) and the
/// active screen.
#[derive(Resource, Default)]
struct MenuState {
    cursor: usize,
    screen: MenuScreen,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuOpen>()
            .init_resource::<MenuAccess>()
            .init_resource::<MenuState>()
            .add_systems(Startup, view::spawn_ui)
            .add_systems(Update, (input::menu_input, view::update_ui));
    }
}

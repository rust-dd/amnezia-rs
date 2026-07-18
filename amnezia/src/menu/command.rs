//! The top-level RM2000 command window and the party status window beside it.
//! This module owns the five default commands (Item / Skill / Equipment / Save /
//! End Game), what each one opens, and the two text columns of the command
//! screen: the command list on the left and the always-visible party roster (each
//! member's name, level, and current/maximum HP and SP) on the right. The
//! per-member figures are derived exactly as the battle system derives them (see
//! [`super::derive`]) so the panel matches a real fight.

use crate::gamedata::GameData;
use crate::i18n;
use crate::progression::Progression;
use crate::state::Party;
use crate::vitals::Vitals;

use super::derive;
use super::{MemberAction, MenuScreen};

/// The RM2000 default main-menu commands, in cursor order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Item,
    Skill,
    Equipment,
    Save,
    EndGame,
}

/// The command list in RM2000 order: Item, Skill, Equipment, Save, End Game.
pub(super) const COMMANDS: [Command; 5] = [
    Command::Item,
    Command::Skill,
    Command::Equipment,
    Command::Save,
    Command::EndGame,
];

/// The Hungarian label shown for a command.
pub(super) fn label(command: Command) -> &'static str {
    match command {
        Command::Item => "Tárgy",
        Command::Skill => "Képesség",
        Command::Equipment => "Felszerelés",
        Command::Save => "Mentés",
        Command::EndGame => "Kilépés",
    }
}

/// What confirming a command does: open a sub-screen, or trigger the single-slot
/// save (which the caller wires to [`crate::save::SaveRequest`]).
pub(super) enum CommandAction {
    Open(MenuScreen),
    Save,
}

/// The action a confirm on `command` performs. Item drills into the held-item
/// list; Skill and Equipment prompt for a party member first; Save requests a
/// save; End Game opens the return-to-title confirmation.
pub(super) fn dispatch(command: Command) -> CommandAction {
    match command {
        Command::Item => CommandAction::Open(MenuScreen::ItemList { cursor: 0 }),
        Command::Skill => CommandAction::Open(MenuScreen::MemberSelect {
            action: MemberAction::Skill,
            cursor: 0,
        }),
        Command::Equipment => CommandAction::Open(MenuScreen::MemberSelect {
            action: MemberAction::Equip,
            cursor: 0,
        }),
        Command::Save => CommandAction::Save,
        Command::EndGame => CommandAction::Open(MenuScreen::EndGame { cursor: 0 }),
    }
}

/// The screen reached by confirming `member` in a member-select prompt: the
/// member's skill list, their equipment view, or their status detail.
pub(super) fn member_screen(action: MemberAction, member: usize) -> MenuScreen {
    match action {
        MemberAction::Skill => MenuScreen::SkillList { member, cursor: 0 },
        MemberAction::Equip => MenuScreen::Equip { member },
        MemberAction::Status => MenuScreen::Status { member },
    }
}

/// Compose the command column, marking the row under `cursor`.
pub(super) fn compose_commands(cursor: usize) -> String {
    let mut out = String::from("- Menü -\n\n");
    for (i, &command) in COMMANDS.iter().enumerate() {
        let marker = if i == cursor { "▶ " } else { "  " };
        out.push_str(&format!("{marker}{}\n", label(command)));
    }
    out.push_str("\n[→] csapat\n[Esc] bezár");
    out
}

/// Compose the party status column: each member's name, level, and current/
/// maximum HP and SP. When `cursor` is `Some`, the selected member is marked (the
/// member-select prompts share this panel as their selection surface).
pub(super) fn compose_party(
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
    cursor: Option<usize>,
) -> String {
    let mut out = String::from("- Csapat -\n\n");
    for (row, &id) in party.snapshot().iter().enumerate() {
        let marker = match cursor {
            Some(c) if c == row => "▶ ",
            _ => "  ",
        };
        match data.actor(id) {
            Some(def) => {
                let level = progression.level(def);
                let (max_hp, max_sp) = derive::max_hp_sp(def, level);
                let (hp, sp) = vitals.get_stored(id).unwrap_or((max_hp, max_sp));
                out.push_str(&format!(
                    "{marker}{}  Lv{level}\n     HP {hp}/{max_hp}  SP {sp}/{max_sp}\n",
                    i18n::tr(&def.name)
                ));
            }
            None => out.push_str(&format!("{marker}#{id}\n")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    #[test]
    fn commands_are_the_five_rm2000_entries_in_order_including_kilepes() {
        let labels: Vec<&str> = COMMANDS.iter().map(|&c| label(c)).collect();
        assert_eq!(
            labels,
            vec!["Tárgy", "Képesség", "Felszerelés", "Mentés", "Kilépés"]
        );
        // The last command is the missing "End Game" the rework restores.
        assert_eq!(COMMANDS.last(), Some(&Command::EndGame));
    }

    #[test]
    fn dispatch_routes_each_command_to_its_screen_or_save() {
        assert!(matches!(
            dispatch(Command::Item),
            CommandAction::Open(MenuScreen::ItemList { .. })
        ));
        assert!(matches!(
            dispatch(Command::Skill),
            CommandAction::Open(MenuScreen::MemberSelect {
                action: MemberAction::Skill,
                ..
            })
        ));
        assert!(matches!(
            dispatch(Command::Equipment),
            CommandAction::Open(MenuScreen::MemberSelect {
                action: MemberAction::Equip,
                ..
            })
        ));
        assert!(matches!(dispatch(Command::Save), CommandAction::Save));
        assert!(matches!(
            dispatch(Command::EndGame),
            CommandAction::Open(MenuScreen::EndGame { .. })
        ));
    }

    #[test]
    fn member_screen_maps_the_action_to_its_detail() {
        assert_eq!(
            member_screen(MemberAction::Skill, 2),
            MenuScreen::SkillList {
                member: 2,
                cursor: 0
            }
        );
        assert_eq!(
            member_screen(MemberAction::Equip, 1),
            MenuScreen::Equip { member: 1 }
        );
        assert_eq!(
            member_screen(MemberAction::Status, 0),
            MenuScreen::Status { member: 0 }
        );
    }

    #[test]
    fn command_column_marks_the_cursor_row() {
        let text = compose_commands(1);
        assert!(text.contains("▶ Képesség"), "cursor on Skill: {text}");
        assert!(text.contains("  Tárgy"), "Item not marked: {text}");
        assert!(text.contains("Kilépés"), "End Game listed: {text}");
    }

    #[test]
    fn party_panel_shows_each_members_name_level_and_vitals() {
        let mut vitals = Vitals::default();
        vitals.set(1, 20, 5);
        let text = compose_party(
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &vitals,
            Some(0),
        );
        assert!(text.contains("▶ Ron  Lv2"), "name/level + cursor: {text}");
        assert!(text.contains("HP 20/63"), "hp cur/max: {text}");
        assert!(text.contains("SP 5/37"), "sp cur/max: {text}");
    }

    #[test]
    fn party_panel_defaults_to_full_when_no_vitals_stored() {
        let text = compose_party(
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Vitals::default(),
            None,
        );
        assert!(text.contains("Ron  Lv2"), "name/level: {text}");
        assert!(text.contains("HP 63/63"), "full hp: {text}");
    }
}

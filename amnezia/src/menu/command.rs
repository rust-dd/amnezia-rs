//! The top-level RM2000 command window: the five default commands (Item / Skill /
//! Equipment / Save / End Game), their Hungarian labels, and what confirming each
//! one opens. The window itself — five bare rows under a windowskin cursor — is
//! drawn by [`super::view`]; this module owns only the command set and the screen
//! each confirm drills into.

use super::{MemberAction, MenuScreen};
use crate::terms::Terms;

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

/// The localised label for a command, sourced from the real RM2000 Terms (the
/// main-menu commands reuse the battle `command_item` / `command_skill` terms and
/// the `menu_equipment` / `menu_save` / `menu_quit` terms — EasyRPG
/// `Scene_Menu::CreateCommandWindow`). Each falls back to its faithful Hungarian
/// placeholder when the term is blank, and routes through `i18n::tr` for English.
pub(super) fn label(command: Command, terms: &Terms) -> String {
    let t = &terms.0;
    match command {
        Command::Item => terms.label(&t.command_item, "Tárgy"),
        Command::Skill => terms.label(&t.command_skill, "Képesség"),
        Command::Equipment => terms.label(&t.menu_equipment, "Felszerelés"),
        Command::Save => terms.label(&t.menu_save, "Mentés"),
        Command::EndGame => terms.label(&t.menu_quit, "Kilépés"),
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
        // RM2000 `Scene_End::CreateCommandWindow` opens with the cursor on Nem
        // (`SetIndex(1)`), so a reflexive confirm cancels rather than quitting.
        Command::EndGame => CommandAction::Open(MenuScreen::EndGame { cursor: 1 }),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_resolve_to_the_parsed_terms_in_rm2000_order() {
        // The five commands map onto the real RM2000 command / menu terms.
        let mut terms = Terms::default();
        terms.0.command_item = "Item".into();
        terms.0.command_skill = "Skill".into();
        terms.0.menu_equipment = "Equip".into();
        terms.0.menu_save = "Save".into();
        terms.0.menu_quit = "Quit".into();
        let labels: Vec<String> = COMMANDS.iter().map(|&c| label(c, &terms)).collect();
        assert_eq!(labels, vec!["Item", "Skill", "Equip", "Save", "Quit"]);
        // The last command is the "End Game" the rework restores.
        assert_eq!(COMMANDS.last(), Some(&Command::EndGame));
    }

    #[test]
    fn a_blank_term_falls_back_to_the_hungarian_placeholder() {
        // With no parsed terms the chrome still reads naturally in Hungarian.
        let terms = Terms::default();
        let labels: Vec<String> = COMMANDS.iter().map(|&c| label(c, &terms)).collect();
        assert_eq!(
            labels,
            vec!["Tárgy", "Képesség", "Felszerelés", "Mentés", "Kilépés"]
        );
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
        // End Game opens with the cursor defaulted to Nem (cursor 1), so a
        // reflexive confirm cancels instead of quitting to the title.
        assert!(matches!(
            dispatch(Command::EndGame),
            CommandAction::Open(MenuScreen::EndGame { cursor: 1 })
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
}

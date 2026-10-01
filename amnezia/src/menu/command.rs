//! Top-level command order, labels and destination screens; rendering lives in [`super::view`].

use super::{MemberAction, MenuScreen};
use crate::state::Party;
use crate::terms::Terms;
use crate::vitals::Vitals;

/// The RM2000 default main-menu commands, in cursor order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Item,
    Skill,
    Equipment,
    Save,
    EndGame,
}

pub(super) const COMMANDS: [Command; 5] = [
    Command::Item,
    Command::Skill,
    Command::Equipment,
    Command::Save,
    Command::EndGame,
];

pub(super) fn enabled(command: Command, members: usize, save: bool) -> bool {
    match command {
        Command::Item | Command::Skill | Command::Equipment => members > 0,
        Command::Save => save,
        Command::EndGame => true,
    }
}

pub(super) fn member_enabled(
    action: MemberAction,
    member: usize,
    party: &Party,
    vitals: &Vitals,
) -> bool {
    let roster = party.snapshot();
    let Some(&actor) = roster.get(member) else {
        return false;
    };
    if action != MemberAction::Skill {
        return true;
    }
    let active = vitals.states(actor);
    !crate::conditions::definitions()
        .iter()
        .any(|state| state.restriction == 1 && active.contains(&state.id))
}

/// Localized RM2000 command/menu terms, matching `Scene_Menu::CreateCommandWindow`;
/// blank terms use Hungarian fallbacks.
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

pub(super) enum CommandAction {
    Open(MenuScreen),
    Save,
}

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
        MemberAction::Equip => MenuScreen::Equip {
            member,
            slot: 0,
            picking: None,
        },
        MemberAction::Status => MenuScreen::Status { member },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_resolve_to_the_parsed_terms_in_rm2000_order() {
        let mut terms = Terms::default();
        terms.0.command_item = "Item".into();
        terms.0.command_skill = "Skill".into();
        terms.0.menu_equipment = "Equip".into();
        terms.0.menu_save = "Save".into();
        terms.0.menu_quit = "Quit".into();
        let labels: Vec<String> = COMMANDS.iter().map(|&c| label(c, &terms)).collect();
        assert_eq!(labels, vec!["Item", "Skill", "Equip", "Save", "Quit"]);
        assert_eq!(COMMANDS.last(), Some(&Command::EndGame));
    }

    #[test]
    fn a_blank_term_falls_back_to_the_hungarian_placeholder() {
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
            MenuScreen::Equip {
                member: 1,
                slot: 0,
                picking: None
            }
        );
        assert_eq!(
            member_screen(MemberAction::Status, 0),
            MenuScreen::Status { member: 0 }
        );
    }
}

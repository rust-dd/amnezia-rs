//! The top-level RM2000 command window: the five default commands (Item / Skill /
//! Equipment / Save / End Game), their Hungarian labels, and what confirming each
//! one opens. The window itself — five bare rows under a windowskin cursor — is
//! drawn by [`super::view`]; this module owns only the command set and the screen
//! each confirm drills into.

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
    fn commands_are_the_five_rm2000_entries_in_order_including_kilepes() {
        let labels: Vec<&str> = COMMANDS.iter().map(|&c| label(c)).collect();
        assert_eq!(
            labels,
            vec!["Tárgy", "Képesség", "Felszerelés", "Mentés", "Kilépés"]
        );
        // The last command is the "End Game" the rework restores.
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

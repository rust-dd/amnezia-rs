//! The menu's render dispatch: turn the active [`MenuScreen`] into the two text
//! columns the panel shows — the left/main column and the right party column. The
//! party column is only filled on the top-level Command screen (the RM2000 status
//! window beside the command list); every sub-screen fills the left column alone
//! and leaves the right empty (the caller hides the empty right window). The
//! member-select prompts reuse the party roster from [`super::command`] as their
//! left-hand selection surface.

use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::vitals::Vitals;

use super::{MemberAction, MenuScreen, command, equip, items, skills, status, use_item};

/// The End Game confirmation rows, in cursor order (Igen = yes returns to title).
pub(super) const END_GAME_ROWS: [&str; 2] = ["Igen", "Nem"];

/// Compose the `(left, right)` text for `screen`. `command_cursor` is the Command
/// screen's row (kept in `MenuState`); every other screen carries its own cursor.
#[allow(clippy::too_many_arguments)]
pub(super) fn compose(
    screen: MenuScreen,
    command_cursor: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    inventory: &Inventory,
    vitals: &Vitals,
) -> (String, String) {
    match screen {
        MenuScreen::Command => (
            command::compose_commands(command_cursor),
            command::compose_party(data, party, progression, vitals, None),
        ),
        MenuScreen::ItemList { cursor } => {
            (items::compose_list(cursor, data, inventory), String::new())
        }
        MenuScreen::ItemTarget { item_id, cursor } => (
            use_item::compose_target(item_id, cursor, data, party, progression, vitals),
            String::new(),
        ),
        MenuScreen::MemberSelect { action, cursor } => (
            member_select(action, cursor, data, party, progression, vitals),
            String::new(),
        ),
        MenuScreen::SkillList { member, cursor } => (
            skills::compose_list(member, cursor, data, party),
            String::new(),
        ),
        MenuScreen::SkillTarget {
            member,
            skill_id,
            cursor,
        } => (
            skills::compose_target(member, skill_id, cursor, data, party, progression, vitals),
            String::new(),
        ),
        MenuScreen::Equip { member } => (
            equip::compose(member, data, party, progression),
            String::new(),
        ),
        MenuScreen::Status { member } => (
            status::compose_status(member, data, party, progression, vitals),
            String::new(),
        ),
        MenuScreen::Saved => (compose_saved(), String::new()),
        MenuScreen::EndGame { cursor } => (compose_end_game(cursor), String::new()),
    }
}

/// The left column of a member-select prompt: a title for the pending action plus
/// the party roster with the selection cursor.
fn member_select(
    action: MemberAction,
    cursor: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
) -> String {
    let title = match action {
        MemberAction::Skill => "Ki használjon képességet?",
        MemberAction::Equip => "Kinek a felszerelése?",
        MemberAction::Status => "Kit nézel meg?",
    };
    format!(
        "{title}\n\n{}\n[Esc] vissza",
        command::compose_party(data, party, progression, vitals, Some(cursor))
    )
}

/// The Save confirmation shown after the single-slot save is requested.
fn compose_saved() -> String {
    String::from("Mentés kész.\n\n[Enter] vissza")
}

/// The End Game (return-to-title) confirmation, marking the cursor row.
fn compose_end_game(cursor: usize) -> String {
    let mut out = String::from("Visszatérsz a címképernyőre?\n\n");
    for (i, label) in END_GAME_ROWS.iter().enumerate() {
        let marker = if i == cursor { "▶ " } else { "  " };
        out.push_str(&format!("{marker}{label}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    #[test]
    fn command_screen_fills_both_columns() {
        let (left, right) = compose(
            MenuScreen::Command,
            0,
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Inventory::default(),
            &Vitals::default(),
        );
        assert!(left.contains("Kilépés"), "command list on the left: {left}");
        assert!(right.contains("Ron"), "party panel on the right: {right}");
    }

    #[test]
    fn sub_screens_leave_the_right_column_empty() {
        let (_, right) = compose(
            MenuScreen::Status { member: 0 },
            0,
            &testkit::data(),
            &Party::default(),
            &Progression::default(),
            &Inventory::default(),
            &Vitals::default(),
        );
        assert!(
            right.is_empty(),
            "sub-screen hides the party window: {right:?}"
        );
    }

    #[test]
    fn end_game_prompt_lists_igen_then_nem_and_marks_the_cursor() {
        let text = compose_end_game(0);
        assert!(text.contains("▶ Igen"), "cursor on Igen: {text}");
        assert!(text.contains("  Nem"), "Nem present: {text}");
        assert!(text.contains("címképernyőre"), "prompt text: {text}");
    }
}

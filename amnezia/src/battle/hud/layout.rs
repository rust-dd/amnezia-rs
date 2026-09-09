use super::*;

pub(super) fn rectangle(panel: Panel, battle: &Battle) -> Option<(f32, f32, f32, f32)> {
    if battle.events.presenting() {
        return None;
    }
    match panel {
        Panel::Option if battle.phase == Phase::PartyCommand => Some((0.0, 160.0, 76.0, 80.0)),
        Panel::Status if battle.phase == Phase::PartyCommand => Some((76.0, 160.0, 244.0, 80.0)),
        Panel::Status
            if battle.phase == Phase::Command
                && matches!(battle.menu, MenuLevel::Command | MenuLevel::AllyTarget) =>
        {
            Some((0.0, 160.0, 244.0, 80.0))
        }
        Panel::Command if battle.phase == Phase::Command => match battle.menu {
            MenuLevel::Command => Some((244.0, 160.0, 76.0, 80.0)),
            MenuLevel::Skill | MenuLevel::Item => Some((0.0, 160.0, 320.0, 80.0)),
            MenuLevel::Target => Some((0.0, 160.0, 136.0, 80.0)),
            MenuLevel::AllyTarget => None,
        },
        Panel::Message if matches!(battle.phase, Phase::Resolve | Phase::Outcome) => {
            Some((0.0, 160.0, 320.0, 80.0))
        }
        Panel::Help
            if battle.phase == Phase::Command
                && matches!(battle.menu, MenuLevel::Skill | MenuLevel::Item) =>
        {
            Some((0.0, 0.0, 320.0, 32.0))
        }
        _ => None,
    }
}

pub(super) fn selection(panel: Panel, battle: &Battle) -> Option<(usize, usize)> {
    rectangle(panel, battle)?;
    match panel {
        Panel::Option => Some((battle.cursor, 1)),
        Panel::Command => Some((battle.cursor, list_columns(battle))),
        Panel::Status if battle.menu == MenuLevel::AllyTarget && battle.phase == Phase::Command => {
            (battle.cursor < battle.members.len()).then_some((battle.cursor, 1))
        }
        Panel::Status if battle.phase == Phase::Command => Some((battle.turn, 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::model::testkit::build_party2;

    #[test]
    fn targets_and_ally_selection_use_the_original_narrow_windows() {
        let mut battle = build_party2();
        battle.phase = Phase::Command;
        battle.menu = MenuLevel::Target;
        assert_eq!(
            rectangle(Panel::Command, &battle),
            Some((0.0, 160.0, 136.0, 80.0))
        );
        assert!(rectangle(Panel::Status, &battle).is_none());
        battle.menu = MenuLevel::AllyTarget;
        assert!(rectangle(Panel::Command, &battle).is_none());
        assert_eq!(
            rectangle(Panel::Status, &battle),
            Some((0.0, 160.0, 244.0, 80.0))
        );
    }

    #[test]
    fn two_column_lists_scroll_by_whole_rows_and_keep_the_cursor_visible() {
        let mut start = 0;
        for cursor in 0..40 {
            start = first_visible(start, cursor, 2);
            assert!(start.is_multiple_of(2));
            assert!((start..start + 8).contains(&cursor));
        }
    }

    #[test]
    fn moving_up_inside_the_visible_page_does_not_scroll_the_list() {
        let mut first = first_visible(0, 11, 2);
        assert_eq!(first, 4);
        for cursor in [9, 7, 5] {
            first = first_visible(first, cursor, 2);
            assert_eq!(first, 4);
        }
        assert_eq!(first_visible(first, 3, 2), 2);
    }
}

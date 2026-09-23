use super::*;

pub(super) fn base_menu(battle: &Battle) -> MenuLevel {
    if !matches!(battle.menu, MenuLevel::Target | MenuLevel::AllyTarget) {
        return battle.menu;
    }
    if battle.pending_skill.is_some() {
        MenuLevel::Skill
    } else if battle.pending_item.is_some() {
        MenuLevel::Item
    } else {
        MenuLevel::Command
    }
}

pub(super) fn rectangle(panel: Panel, battle: &Battle) -> Option<(f32, f32, f32, f32)> {
    if battle.events.presenting() {
        return None;
    }
    if battle.phase == Phase::PartyCommand {
        return match panel {
            Panel::Option => Some((0.0, 160.0, 76.0, 80.0)),
            Panel::Status => Some((76.0, 160.0, 244.0, 80.0)),
            Panel::Command => Some((320.0, 160.0, 76.0, 80.0)),
            _ => None,
        };
    }
    if matches!(
        battle.phase,
        Phase::Encounter | Phase::Escape | Phase::Resolve | Phase::Outcome
    ) {
        return (panel == Panel::Message).then_some((0.0, 160.0, 320.0, 80.0));
    }
    if battle.phase != Phase::Command {
        return None;
    }
    let base = base_menu(battle);
    match panel {
        Panel::Option if base == MenuLevel::Command => Some((-76.0, 160.0, 76.0, 80.0)),
        Panel::Command if base == MenuLevel::Command => Some((244.0, 160.0, 76.0, 80.0)),
        Panel::Status if base == MenuLevel::Command || battle.menu == MenuLevel::AllyTarget => {
            Some((0.0, 160.0, 244.0, 80.0))
        }
        Panel::Skill if base == MenuLevel::Skill => Some((0.0, 160.0, 320.0, 80.0)),
        Panel::Item if base == MenuLevel::Item => Some((0.0, 160.0, 320.0, 80.0)),
        Panel::Target if battle.menu == MenuLevel::Target => Some((0.0, 160.0, 136.0, 80.0)),
        Panel::Help if matches!(base, MenuLevel::Skill | MenuLevel::Item) => {
            Some((0.0, 0.0, 320.0, 32.0))
        }
        _ => None,
    }
}

pub(super) fn selection(panel: Panel, battle: &Battle) -> Option<(usize, usize)> {
    rectangle(panel, battle)?;
    if battle.phase == Phase::PartyCommand && panel != Panel::Option {
        return None;
    }
    if matches!(panel, Panel::Help | Panel::Message) {
        return None;
    }
    Some((panel.cursor(battle), panel.columns()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::model::testkit::build_party2;

    #[test]
    fn attack_targets_overlay_the_actor_windows_without_erasing_them() {
        let mut battle = build_party2();
        battle.phase = Phase::Command;
        battle.menu = MenuLevel::Target;
        assert_eq!(
            rectangle(Panel::Target, &battle),
            Some((0.0, 160.0, 136.0, 80.0))
        );
        assert_eq!(
            rectangle(Panel::Command, &battle),
            Some((244.0, 160.0, 76.0, 80.0))
        );
        assert_eq!(
            rectangle(Panel::Status, &battle),
            Some((0.0, 160.0, 244.0, 80.0))
        );
        assert!(rectangle(Panel::Skill, &battle).is_none());
        assert!(rectangle(Panel::Help, &battle).is_none());
        assert!(Panel::Target as usize > Panel::Command as usize);
    }

    #[test]
    fn skill_and_item_target_overlays_keep_the_help_list_and_inactive_cursor() {
        for item in [false, true] {
            let mut battle = build_party2();
            battle.phase = Phase::Command;
            let list = if item { Panel::Item } else { Panel::Skill };
            if item {
                battle.pending_item = Some(1);
                battle.menu_cursors[MenuLevel::Item as usize] = 9;
            } else {
                battle.pending_skill = Some(1);
                battle.menu_cursors[MenuLevel::Skill as usize] = 9;
            }
            for level in [MenuLevel::Target, MenuLevel::AllyTarget] {
                battle.menu = level;
                battle.cursor = 1;
                assert_eq!(rectangle(list, &battle), Some((0.0, 160.0, 320.0, 80.0)));
                assert!(rectangle(Panel::Help, &battle).is_some());
                assert_eq!(selection(list, &battle), Some((9, 2)));
                assert_eq!(
                    Panel::active(&battle),
                    Some(if level == MenuLevel::Target {
                        Panel::Target
                    } else {
                        Panel::Status
                    })
                );
                assert_eq!(
                    rectangle(Panel::Status, &battle).is_some(),
                    level == MenuLevel::AllyTarget
                );
                assert_eq!(
                    rectangle(Panel::Target, &battle).is_some(),
                    level == MenuLevel::Target
                );
                assert!(rectangle(Panel::Command, &battle).is_none());
            }
            assert!(Panel::Status as usize > list as usize);
        }
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

    #[test]
    fn each_list_keeps_its_scroll_under_target_overlays_and_resets_for_a_new_battle() {
        let mut battle = build_party2();
        battle.phase = Phase::Command;
        battle.menu = MenuLevel::Skill;
        battle.cursor = 11;
        let mut app = App::new();
        app.insert_resource(battle)
            .init_resource::<ListScroll>()
            .add_systems(Update, scroll);
        app.update();
        assert_eq!(
            app.world().resource::<ListScroll>().first[Panel::Skill as usize],
            4
        );
        {
            let mut battle = app.world_mut().resource_mut::<Battle>();
            battle.menu_cursors[MenuLevel::Skill as usize] = 11;
            battle.menu = MenuLevel::Target;
            battle.pending_skill = Some(1);
            battle.cursor = 1;
        }
        app.update();
        assert_eq!(
            app.world().resource::<ListScroll>().first[Panel::Skill as usize],
            4
        );
        assert_eq!(Panel::Skill.cursor(app.world().resource::<Battle>()), 11);
        {
            let mut battle = app.world_mut().resource_mut::<Battle>();
            battle.menu = MenuLevel::Item;
            battle.cursor = 9;
        }
        app.update();
        assert_eq!(
            app.world().resource::<ListScroll>().first[Panel::Item as usize],
            2
        );
        assert_eq!(
            app.world().resource::<ListScroll>().first[Panel::Skill as usize],
            4
        );
        {
            let mut battle = app.world_mut().resource_mut::<Battle>();
            battle.generation += 1;
            battle.cursor = 0;
        }
        app.update();
        assert_eq!(app.world().resource::<ListScroll>().first, [0; 8]);
    }
}

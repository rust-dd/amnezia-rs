use super::*;

fn filled(count: usize) -> App {
    let mut app = app_on(0, MenuScreen::ItemList { cursor: 0 });
    for index in 0..count {
        let mut item = app.world().resource::<GameData>().items[0].clone();
        item.id = 1000 + index as u32;
        item.name = format!("Tárgy {index}");
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_item(item.id, 1);
        app.world_mut().resource_mut::<GameData>().items.push(item);
    }
    app
}

#[test]
fn item_list_moves_vertically_by_two_columns_and_horizontally_by_one() {
    let mut app = filled(5);
    for (key, cursor) in [
        (KeyCode::ArrowDown, 2),
        (KeyCode::ArrowRight, 3),
        (KeyCode::ArrowDown, 3),
        (KeyCode::ArrowUp, 1),
        (KeyCode::ArrowLeft, 0),
    ] {
        press_frame(&mut app, key);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::ItemList { cursor }
        );
    }
}

#[test]
fn cancelling_an_item_target_returns_to_the_selected_inventory_entry() {
    let mut app = filled(5);
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::ItemList { cursor: 1 };
    press_frame(&mut app, KeyCode::Enter);
    assert!(matches!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemTarget { .. }
    ));
    press_frame(&mut app, KeyCode::Escape);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemList { cursor: 1 }
    );
}

#[test]
fn an_empty_or_disabled_inventory_choice_plays_only_the_buzzer() {
    for disabled in [false, true] {
        let mut app = filled(0);
        if disabled {
            app.world_mut()
                .resource_mut::<GameData>()
                .items
                .push(testkit::weapon(2000, "Kard", 5));
            app.world_mut()
                .resource_mut::<Inventory>()
                .add_item(2000, 1);
        }
        let mut sounds = SystemSounds::default();
        sounds.buzzer.name = "BUZZER".into();
        sounds.decision.name = "DECISION".into();
        app.insert_resource(sounds);
        press_frame(&mut app, KeyCode::Enter);
        let heard = app
            .world_mut()
            .resource_mut::<Messages<AudioRequest>>()
            .drain()
            .filter_map(|audio| match audio {
                AudioRequest::Sound { name, .. } => Some(name),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(heard, ["BUZZER"]);
        assert!(matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::ItemList { .. }
        ));
    }
}

#[test]
fn item_window_navigation_precedes_confirmation_in_the_same_frame() {
    let mut app = filled(5);
    let expected = use_item::held_item_ids(
        app.world().resource::<GameData>(),
        app.world().resource::<Inventory>(),
    )[1];
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    confirm(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemTarget {
            item_id: expected,
            cursor: 0,
        }
    );
}

#[test]
fn confirmation_during_scrolling_uses_the_new_index_with_the_old_help() {
    let mut app = filled(25);
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::ItemList { cursor: 22 };
    app.update();
    press_frame(&mut app, KeyCode::ArrowDown);
    let nav = &app.world().resource::<items::List>().navigation;
    assert_eq!((nav.index, nav.help_index, nav.offset), (24, 22, 0));
    let expected = use_item::held_item_ids(
        app.world().resource::<GameData>(),
        app.world().resource::<Inventory>(),
    )[24];
    press_frame(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        MenuScreen::ItemTarget {
            item_id: expected,
            cursor: 0,
        }
    );
}

#[test]
fn scene_pause_preserves_the_scroll_and_does_not_accumulate_catch_up_ticks() {
    let mut app = filled(25);
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::ItemList { cursor: 22 };
    app.update();
    press_frame(&mut app, KeyCode::ArrowDown);
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    for frame in [1, 2, 600, 601] {
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame = frame;
        press_frame(&mut app, KeyCode::Escape);
        let nav = &app.world().resource::<items::List>().navigation;
        assert_eq!((nav.index, nav.offset, nav.cursor_frame), (24, 0, 0));
        assert!(app.world().resource::<MenuOpen>().0);
    }
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    assert_eq!(app.world().resource::<items::List>().navigation.offset, 0);
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 602;
    app.update();
    assert_eq!(app.world().resource::<items::List>().navigation.offset, 4);
}

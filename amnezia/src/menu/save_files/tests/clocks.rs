use super::*;
use crate::save::preview::{Contents, PartyPreview};

fn open_load(app: &mut App, valid: bool) {
    app.world_mut()
        .resource_scope(|world, mut files: Mut<SaveFiles>| {
            files.open_load(
                world.resource::<crate::save::SaveLocation>(),
                world.resource::<crate::gamedata::GameData>(),
                world.resource::<crate::timing::GameFrames>().frame,
            );
            if valid {
                files.entries.as_mut().unwrap()[0].contents = Contents::Party(PartyPreview {
                    name: "Ron".into(),
                    level: 2,
                    hp: 63,
                    faces: Vec::new(),
                });
            }
        });
}

#[test]
fn opening_updates_only_the_selected_window_once_before_its_first_input_frame() {
    for selected in 0..15 {
        let mut nav = super::super::navigation::Navigation::new(selected);
        assert_eq!(nav.arrow, 0);
        for (index, cursor) in nav.cursors.iter().enumerate() {
            assert_eq!(*cursor, u32::from(index == selected));
        }
        for tick in 1..=42 {
            nav.tick(&ButtonInput::default(), false, true);
            assert_eq!(nav.cursors[selected], (1 + tick) % 21);
            assert_eq!(nav.arrow, tick % 40);
        }
    }
}

#[test]
fn cancel_confirm_and_disabled_confirm_all_update_file_window_clocks() {
    for (tag, valid, key, decision) in [
        ("load_clock_cancel", true, KeyCode::Escape, Some(false)),
        ("load_clock_confirm", true, KeyCode::Enter, Some(true)),
        ("load_clock_disabled", false, KeyCode::Enter, None),
    ] {
        let (mut app, directory) = app(tag);
        open_load(&mut app, valid);
        let before = app.world().resource::<SaveFiles>().navigation.cursors[0];
        step(&mut app, Some(key));
        let files = app.world().resource::<SaveFiles>();
        assert_eq!(files.navigation.arrow, 1, "{tag}");
        assert_eq!(files.navigation.cursors[0], (before + 1) % 21, "{tag}");
        assert_eq!(files.finished, decision);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        std::fs::remove_dir(directory).unwrap();
    }
}

#[test]
fn repeated_disabled_decisions_do_not_stop_the_cursor_or_arrow_blink() {
    let (mut app, directory) = app("load_clock_repeated");
    open_load(&mut app, false);
    let before = app.world().resource::<SaveFiles>().navigation.cursors[0];
    for tick in 1..=42 {
        step(&mut app, Some(KeyCode::Enter));
        let files = app.world().resource::<SaveFiles>();
        assert_eq!(files.navigation.cursors[0], (before + tick) % 21);
        assert_eq!(files.navigation.arrow, tick % 40);
        assert!(files.finished.is_none());
        assert!(!app.world().resource::<crate::save::LoadRequest>().0);
    }
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn a_simultaneous_arrow_updates_the_window_after_confirming_the_previous_slot() {
    let (mut app, directory) = app("load_clock_simultaneous");
    open_load(&mut app, true);
    let before = app.world().resource::<SaveFiles>().navigation.cursors[0];
    let world = app.world_mut();
    world.resource_mut::<crate::timing::GameFrames>().frame += 1;
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.press(KeyCode::Enter);
    keys.press(KeyCode::ArrowDown);
    app.update();
    let files = app.world().resource::<SaveFiles>();
    assert_eq!(files.finished, Some(true));
    assert_eq!(
        *app.world().resource::<ActiveSlot>(),
        ActiveSlot::new(1).unwrap()
    );
    assert_eq!(files.navigation.index, 1);
    assert_eq!(files.navigation.cursors[0], before);
    assert_eq!(files.navigation.cursors[1], 1);
    assert_eq!(files.navigation.arrow, 1);
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn crystal_confirmation_updates_the_last_visible_frame_before_any_file_write() {
    let (mut app, directory) = app("crystal_clock_confirm");
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    app.world_mut()
        .resource_mut::<crate::save::EventSaveRequest>()
        .0 = true;
    step(&mut app, None);
    advance(&mut app, 12);
    let before = app.world().resource::<SaveFiles>().navigation.cursors[0];
    step(&mut app, Some(KeyCode::Enter));
    let files = app.world().resource::<SaveFiles>();
    assert_eq!(files.finished, Some(true));
    assert_eq!(files.navigation.cursors[0], (before + 1) % 21);
    assert_eq!(files.navigation.arrow, 1);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn a_transition_holds_both_file_clocks_without_catching_up_on_return() {
    let (mut app, directory) = app("load_clock_pause");
    open_load(&mut app, true);
    let before = app.world().resource::<SaveFiles>().navigation.cursors[0];
    let mut transition = crate::transitions::Transition::default();
    transition.start_for(crate::transitions::Kind::Fade, false, 100, IVec2::ZERO, 6);
    app.insert_resource(transition);
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 100;
    step(&mut app, None);
    assert_eq!(
        app.world().resource::<SaveFiles>().navigation.cursors[0],
        before
    );
    assert_eq!(app.world().resource::<SaveFiles>().navigation.arrow, 0);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    step(&mut app, None);
    assert_eq!(
        app.world().resource::<SaveFiles>().navigation.cursors[0],
        (before + 1) % 21
    );
    assert_eq!(app.world().resource::<SaveFiles>().navigation.arrow, 1);
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn a_failed_load_marks_the_confirmed_file_instead_of_the_moved_cursor() {
    let (mut app, directory) = app("load_clock_failed_selection");
    open_load(&mut app, true);
    let world = app.world_mut();
    world.resource_mut::<crate::timing::GameFrames>().frame += 1;
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.press(KeyCode::Enter);
    keys.press(KeyCode::ArrowDown);
    app.update();
    let selected = *app.world().resource::<ActiveSlot>();
    let mut files = app.world_mut().resource_mut::<SaveFiles>();
    assert_eq!(files.navigation.index, 1);
    files.suspend();
    files.reject_load(2, selected);
    let entries = files.entries.as_ref().unwrap();
    assert_eq!(entries[0].contents, Contents::Corrupt);
    assert_eq!(entries[1].contents, Contents::Empty);
    std::fs::remove_dir(directory).unwrap();
}

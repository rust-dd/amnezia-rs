use super::*;
use crate::menu::equip::Scene;
use crate::timing::GameFrames;

fn screen(slot: usize, picking: Option<usize>) -> MenuScreen {
    MenuScreen::Equip {
        member: 0,
        slot,
        picking,
    }
}

fn prepared(picking: Option<usize>) -> App {
    let mut app = app_on(2, screen(0, picking));
    app.world_mut().resource_mut::<GameData>().items =
        (10..40).map(|id| testkit::weapon(id, "Kard", id)).collect();
    for id in 10..40 {
        app.world_mut().resource_mut::<Inventory>().add_item(id, 1);
    }
    app.update();
    app
}

fn tick(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

#[test]
fn slots_wrap_and_page_keys_jump_to_the_ends_without_moving_the_item_cursor() {
    let mut app = prepared(None);
    for (key, slot) in [
        (KeyCode::ArrowUp, 4),
        (KeyCode::ArrowDown, 0),
        (KeyCode::PageDown, 4),
        (KeyCode::PageUp, 0),
    ] {
        press_frame(&mut app, key);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            screen(slot, None)
        );
    }
    press_frame(&mut app, KeyCode::Enter);
    press_frame(&mut app, KeyCode::PageDown);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, Some(0))
    );
}

#[test]
fn scroll_keeps_the_old_cursor_and_help_during_four_intermediate_ticks() {
    let mut app = prepared(Some(10));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    for (frame, offset, cursor, help) in [
        (1, 0, 10, 20),
        (2, 4, 10, 20),
        (3, 8, 10, 20),
        (4, 12, 10, 20),
        (5, 16, 12, 22),
    ] {
        tick(&mut app, frame);
        let scene = app.world().resource::<Scene>();
        let nav = &scene.lists[0];
        assert_eq!(
            (
                nav.index,
                nav.offset,
                nav.cursor_index,
                nav.cursor_y,
                scene.help_id
            ),
            (12, offset, cursor, 80, help),
            "frame {frame}"
        );
    }
}

#[test]
fn comparison_precedes_item_navigation_and_never_changes_inventory() {
    let mut app = prepared(Some(0));
    assert_eq!(app.world().resource::<Scene>().preview.unwrap()[0], 38);
    press_frame(&mut app, KeyCode::ArrowRight);
    assert_eq!(app.world().resource::<Scene>().preview.unwrap()[0], 38);
    assert_eq!(app.world().resource::<Scene>().help_id, 11);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    tick(&mut app, 1);
    assert_eq!(app.world().resource::<Scene>().preview.unwrap()[0], 39);
    for id in 10..40 {
        assert_eq!(app.world().resource::<Inventory>().count(id), 1);
    }
    assert!(app.world().resource::<Equipment>().entries().is_empty());
}

#[test]
fn hidden_item_windows_keep_offsets_and_arrow_clocks_but_pause_the_cursor() {
    let mut app = prepared(Some(12));
    tick(&mut app, 1);
    let cursor = app.world().resource::<Scene>().lists[0].cursor_frame;
    let arrow = app.world().resource::<Scene>().lists[0].arrow_frame;
    app.world_mut().resource_mut::<MenuState>().screen = screen(1, None);
    tick(&mut app, 11);
    let nav = &app.world().resource::<Scene>().lists[0];
    assert_eq!(
        (nav.offset, nav.cursor_frame, nav.arrow_frame),
        (16, cursor, (arrow + 10) % 40)
    );
    app.world_mut().resource_mut::<MenuState>().screen = screen(0, Some(0));
    tick(&mut app, 12);
    let nav = &app.world().resource::<Scene>().lists[0];
    assert_eq!((nav.offset, nav.cursor_frame), (0, (cursor + 1) % 21));
}

#[test]
fn inactive_slot_cursor_freezes_while_the_picker_is_active() {
    let mut app = prepared(None);
    tick(&mut app, 11);
    let frozen = app.world().resource::<Scene>().slot_frame;
    press_frame(&mut app, KeyCode::Enter);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    tick(&mut app, 35);
    assert_eq!(app.world().resource::<Scene>().slot_frame, frozen);
    assert_eq!(
        app.world().resource::<Scene>().lists[0].cursor_frame,
        24 % 21
    );
}

#[test]
fn transition_pauses_scene_clocks_and_actions_but_not_global_hold_time() {
    let mut app = prepared(Some(0));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    tick(&mut app, 1);
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    tick(&mut app, 30);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, Some(2))
    );
    assert_eq!(app.world().resource::<Scene>().lists[0].cursor_frame, 1);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    tick(&mut app, 31);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, Some(2))
    );
    tick(&mut app, 32);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        screen(0, Some(4))
    );
}

#[test]
fn held_picker_navigation_is_render_rate_independent() {
    for fps in [15, 30, 60, 120, 144] {
        let mut app = prepared(Some(0));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        for rendered in 1..=fps {
            tick(&mut app, rendered * 60 / fps);
        }
        let nav = &app.world().resource::<Scene>().lists[0];
        assert_eq!(
            (nav.index, nav.offset, nav.cursor_index),
            (22, 80, 20),
            "{fps} FPS"
        );
    }
}

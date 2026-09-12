use super::*;

fn original(map_id: u32, wait: bool) -> EventCommand {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    let commands = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .filter(|command| command.code == 11610 && command.params[1] == i32::from(wait))
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 1);
    commands[0].clone()
}

fn start(app: &mut App, command: EventCommand, parallel: bool) {
    let commands = vec![command, switch_cmd(9998, 0, 0)];
    if parallel {
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 4, commands)],
        });
    } else {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, commands);
    }
}

fn press(app: &mut App, key: KeyCode) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *keys = ButtonInput::default();
    keys.press(key);
    app.update();
}

#[test]
fn the_original_draco_poll_still_reads_held_cancel_after_another_wait_clears_triggers() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![original(221, true)]),
            map_event(2, 4, vec![original(13, false)]),
        ],
    });
    press(&mut app, KeyCode::Escape);
    assert_eq!(app.world().resource::<Variables>().get(62), 0);
    assert_eq!(app.world().resource::<Variables>().get(52), 6);
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Escape)
    );
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(62), 0);
    assert_eq!(app.world().resource::<Variables>().get(52), 6);
}

#[test]
fn original_waits_only_complete_on_a_later_accepted_trigger() {
    for (key, code) in [
        (KeyCode::ArrowDown, 1),
        (KeyCode::ArrowLeft, 2),
        (KeyCode::ArrowRight, 3),
        (KeyCode::ArrowUp, 4),
        (KeyCode::Enter, 5),
        (KeyCode::Space, 5),
        (KeyCode::Escape, 6),
    ] {
        for map in [13, 118, 221] {
            for parallel in [false, true] {
                let mut app = interp_app();
                let command = original(map, true);
                let variable = command.params[0] as u32;
                start(&mut app, command, parallel);
                press(&mut app, key);
                assert!(!switch_on(&app, 9998));
                app.update();
                assert!(!switch_on(&app, 9998));
                press(&mut app, KeyCode::ShiftLeft);
                assert!(!switch_on(&app, 9998));
                press(&mut app, key);
                let expected = if map == 118 && key == KeyCode::Escape {
                    0
                } else {
                    code
                };
                assert_eq!(app.world().resource::<Variables>().get(variable), expected);
                assert_eq!(switch_on(&app, 9998), expected != 0);
            }
        }
    }
}

#[test]
fn an_armed_wait_does_not_poll_or_reset_its_variable_under_a_message_or_menu() {
    for parallel in [false, true] {
        for menu in [false, true] {
            let mut app = interp_app();
            start(&mut app, original(118, true), parallel);
            app.update();
            app.world_mut().resource_mut::<Variables>().set(89, 789);
            app.world_mut().resource_mut::<MenuOpen>().0 = menu;
            app.world_mut().resource_mut::<Dialogue>().active = !menu;
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.world().resource::<Variables>().get(89), 789);
            assert!(!switch_on(&app, 9998));
            app.world_mut().resource_mut::<MenuOpen>().0 = false;
            app.world_mut().resource_mut::<Dialogue>().close();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
            app.update();
            assert_eq!(app.world().resource::<Variables>().get(89), 0);
            assert!(!switch_on(&app, 9998));
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.world().resource::<Variables>().get(89), 5);
            assert!(switch_on(&app, 9998));
        }
    }
}

#[test]
fn original_waits_reset_triggers_without_releasing_held_keys() {
    for map in [13, 118, 221] {
        for parallel in [false, true] {
            let command = original(map, true);
            let variable = command.params[0] as u32;
            let mut app = interp_app();
            start(&mut app, command, parallel);
            let held = [
                KeyCode::Enter,
                KeyCode::Escape,
                KeyCode::ArrowUp,
                KeyCode::KeyA,
            ];
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                for key in held {
                    keys.press(key);
                }
                keys.press(KeyCode::KeyB);
                keys.clear_just_pressed(KeyCode::KeyB);
                keys.release(KeyCode::KeyB);
            }
            app.update();
            assert_eq!(app.world().resource::<Variables>().get(variable), 0);
            assert!(!switch_on(&app, 9998));
            let keys = app.world().resource::<ButtonInput<KeyCode>>();
            assert_eq!(
                keys.get_just_pressed().len(),
                0,
                "Map{map}, parallel {parallel}"
            );
            assert!(held.into_iter().all(|key| keys.pressed(key)));
            assert!(keys.just_released(KeyCode::KeyB));
            app.update();
            assert!(!switch_on(&app, 9998));
        }
    }
}

#[test]
fn every_unanswered_wait_poll_overwrites_another_variable_writer_with_zero() {
    for parallel in [false, true] {
        let mut app = interp_app();
        start(&mut app, original(118, true), parallel);
        app.update();
        app.world_mut().resource_mut::<Variables>().set(89, 123);
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(89), 0);
        assert!(!switch_on(&app, 9998));
    }
}

#[test]
fn a_parallel_wait_resets_its_target_before_waiting_for_another_message() {
    let mut app = interp_app();
    start(&mut app, original(221, true), true);
    app.world_mut().resource_mut::<Dialogue>().active = true;
    app.world_mut().resource_mut::<Variables>().set(62, 456);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(62), 0);
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Enter)
    );
    assert!(!switch_on(&app, 9998));
}

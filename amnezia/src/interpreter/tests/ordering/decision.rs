use super::*;

fn branch(id: i32) -> Vec<EventCommand> {
    vec![
        cmd(CONDITIONAL_BRANCH, 0, vec![8]),
        switch_cmd(id, 0, 1),
        cmd(END_BRANCH, 0, vec![]),
    ]
}

fn event(id: u32, trigger: u32, commands: Vec<EventCommand>) -> Event {
    let mut event = map_event(id, trigger, commands);
    event.x = 5;
    event.y = 6;
    event.pages[0].layer = 1;
    event
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *input = default();
    for &key in keys {
        input.press(key);
    }
    app.update();
}

#[test]
fn decision_origin_distinguishes_button_contacts_from_movement_contacts() {
    for trigger in [1, 2] {
        for decision in [false, true] {
            let mut app = app();
            crate::dialogue::testing::register_actions(&mut app);
            app.insert_resource(MapEvents {
                events: vec![event(1, trigger, branch(10))],
            });
            press(
                &mut app,
                if decision {
                    &[KeyCode::Enter]
                } else {
                    &[KeyCode::ArrowDown]
                },
            );
            assert_eq!(switch_on(&app, 10), decision);
            assert!(!app.world().resource::<RunningEvent>().active());
        }
    }
}

#[test]
fn decision_attempt_updates_the_origin_of_an_already_queued_contact() {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    app.insert_resource(MapEvents {
        events: vec![event(1, 1, branch(10))],
    });
    press(&mut app, &[KeyCode::ArrowDown, KeyCode::Enter]);
    assert!(switch_on(&app, 10));
}

#[test]
fn called_events_are_not_decision_triggered_and_return_restores_the_caller_origin() {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    let mut commands = branch(10);
    commands.push(cmd(12330, 0, vec![1, 2, 1]));
    commands.extend(branch(12));
    app.insert_resource(MapEvents {
        events: vec![event(1, 0, commands), map_event(2, 0, branch(11))],
    });
    press(&mut app, &[KeyCode::Enter]);
    assert!(switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
    assert!(switch_on(&app, 12));
}

#[test]
fn common_and_parallel_frames_do_not_inherit_a_queued_actions_origin() {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    let mut commands = branch(11);
    commands.push(switch_cmd(5, 1, 0));
    app.insert_resource(CommonEvents(vec![common(1, 3, 5, commands)]));
    app.world_mut().resource_mut::<Switches>().set(5, true);
    app.insert_resource(MapEvents {
        events: vec![event(1, 0, branch(10)), map_event(2, 4, branch(12))],
    });
    press(&mut app, &[KeyCode::Enter]);
    assert!(switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
    assert!(!switch_on(&app, 12));
}

#[test]
fn saved_active_nested_and_pending_actions_keep_their_own_decision_origin() {
    use crate::interpreter::saved::{State, restore};
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    let mut commands = vec![cmd(12330, 0, vec![1, 3, 1])];
    commands.extend(branch(10));
    let mut called = vec![cmd(11410, 0, vec![100])];
    called.extend(branch(12));
    app.insert_resource(MapEvents {
        events: vec![
            event(1, 0, commands),
            event(2, 0, branch(11)),
            map_event(3, 0, called),
        ],
    });
    press(&mut app, &[KeyCode::Enter]);
    let saved = app.world().resource::<RunningEvent>().snapshot().unwrap();
    let encoded = ron::to_string(&saved).unwrap();
    restore(
        app.world_mut(),
        Some(ron::from_str::<State>(&encoded).unwrap()),
    );
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    press(&mut app, &[]);
    assert!(switch_on(&app, 10));
    assert!(switch_on(&app, 11));
    assert!(!switch_on(&app, 12));
}

#[test]
fn legacy_foreground_frames_and_queue_entries_default_to_non_decision_origin() {
    use crate::interpreter::saved::{State, restore};
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    let mut first = vec![cmd(11410, 0, vec![100])];
    first.extend(branch(10));
    app.insert_resource(MapEvents {
        events: vec![event(1, 0, first), event(2, 0, branch(11))],
    });
    press(&mut app, &[KeyCode::Enter]);
    let saved = app.world().resource::<RunningEvent>().snapshot().unwrap();
    let encoded = ron::to_string(&saved)
        .unwrap()
        .replace("decision:true,", "")
        .replace(",decision:true", "");
    assert!(!encoded.contains("decision:true"));
    restore(
        app.world_mut(),
        Some(ron::from_str::<State>(&encoded).unwrap()),
    );
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    press(&mut app, &[]);
    assert!(!switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
}

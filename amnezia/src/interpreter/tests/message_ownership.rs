use super::*;

#[test]
fn original_prison_scene_background_opens_the_door_during_another_message() {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0084.ron",
        crate::assets::asset_root()
    ));
    let page = &map.events.iter().find(|e| e.id == 48).unwrap().pages[0];
    assert_eq!(page.trigger, 4);
    assert_eq!(page.commands[4].code, 10210);
    assert_eq!(page.commands[4].params, [0, 202, 202, 0]);
    let mut app = interp_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.insert_resource(MapEvents {
        events: vec![map_event(48, 4, page.commands[..5].to_vec())],
    });
    app.world_mut().resource_mut::<Dialogue>().active = true;
    for _ in 0..50 {
        app.update();
    }
    assert!(switch_on(&app, 202));
    assert!(app.world().resource::<Dialogue>().active);
}

fn choice_commands() -> Vec<EventCommand> {
    vec![
        cmd(10140, 0, vec![0]),
        EventCommand {
            string: "Igen".into(),
            ..cmd(20140, 0, vec![0])
        },
        switch_cmd(42, 0, 1),
        cmd(20141, 0, vec![]),
    ]
}

#[test]
fn choice_result_belongs_only_to_its_interpreter_even_when_foreground_runs_first() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![switch_cmd(41, 2, 0)]),
            map_event(2, 4, choice_commands()),
        ],
    });
    app.update();
    assert!(app.world().resource::<Choice>().active());
    app.update();
    assert!(!switch_on(&app, 42));
    {
        let mut choice = app.world_mut().resource_mut::<Choice>();
        choice.active = false;
        choice.result = Some(0);
    }
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(3, vec![switch_cmd(43, 0, 0)]);
    app.update();
    assert!(switch_on(&app, 42));
    assert!(switch_on(&app, 43));
    assert!(!app.world().resource::<Choice>().active());
    assert_eq!(app.world().resource::<Choice>().result, None);
}

#[test]
fn pending_choice_result_is_not_overwritten_by_an_earlier_parallel_prompt() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![cmd(11410, 0, vec![0]), message("Következő")]),
            map_event(2, 4, choice_commands()),
        ],
    });
    app.update();
    {
        let mut choice = app.world_mut().resource_mut::<Choice>();
        choice.active = false;
        choice.result = Some(0);
    }
    app.update();
    assert!(switch_on(&app, 42));
    assert!(!app.world().resource::<Dialogue>().active);
    app.update();
    assert!(app.world().resource::<Dialogue>().active);
}

#[test]
fn a_removed_parallel_page_cannot_leave_an_orphaned_result_blocking_future_messages() {
    for numeric in [false, true] {
        let mut app = interp_app();
        app.insert_resource(MapEvents {
            events: vec![map_event(
                1,
                4,
                if numeric {
                    vec![cmd(10150, 0, vec![3, 50])]
                } else {
                    choice_commands()
                },
            )],
        });
        app.update();
        app.world_mut().resource_mut::<MapEvents>().events.clear();
        app.update();
        if numeric {
            let mut input = app.world_mut().resource_mut::<InputNumber>();
            input.active = false;
            input.result = Some(123);
        } else {
            let mut choice = app.world_mut().resource_mut::<Choice>();
            choice.active = false;
            choice.result = Some(0);
        }
        app.world_mut().resource_mut::<MapEvents>().events =
            vec![map_event(2, 4, vec![message("Új esemény")])];
        app.update();
        assert!(app.world().resource::<Dialogue>().active);
        assert_eq!(
            app.world().resource::<Variables>().get(50),
            if numeric { 123 } else { 0 }
        );
        assert!(!switch_on(&app, 42));
    }
}

#[test]
fn numeric_prompt_holds_its_owner_without_freezing_other_background_work() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![cmd(10150, 0, vec![3, 50]), switch_cmd(40, 0, 0)]),
            map_event(2, 4, vec![switch_cmd(41, 2, 0)]),
        ],
    });
    app.update();
    assert!(app.world().resource::<InputNumber>().active());
    for _ in 0..3 {
        let before = switch_on(&app, 41);
        app.update();
        assert_ne!(switch_on(&app, 41), before);
        assert!(!switch_on(&app, 40));
    }
    {
        let mut input = app.world_mut().resource_mut::<InputNumber>();
        input.active = false;
        input.result = Some(123);
    }
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(50), 123);
    assert!(switch_on(&app, 40));
}

#[test]
fn waiting_keys_do_not_consume_dialogue_input_but_nonwaiting_keys_can_sample_it() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![cmd(11610, 0, vec![50, 1, 1, 1, 1])]),
            map_event(2, 4, vec![cmd(11610, 0, vec![51, 0, 1, 1, 1])]),
        ],
    });
    app.update();
    app.world_mut().resource_mut::<Dialogue>().active = true;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(50), 0);
    assert_eq!(app.world().resource::<Variables>().get(51), 5);
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(50), 5);
}

#[test]
fn message_sensitive_commands_wait_but_tint_and_other_effects_continue() {
    for code in [
        10110, 10120, 10130, 10140, 10150, 10810, 10830, 10710, 10720, 10730, 11010, 11020, 11110,
        11120, 11130, 11910, 12420, 12510,
    ] {
        let mut app = interp_app();
        app.world_mut().resource_mut::<Dialogue>().active = true;
        app.insert_resource(MapEvents {
            events: vec![map_event(
                1,
                4,
                vec![
                    switch_cmd(40, 0, 0),
                    cmd(code, 0, vec![]),
                    switch_cmd(41, 0, 0),
                ],
            )],
        });
        app.update();
        assert!(switch_on(&app, 40), "command {code}");
        assert!(!switch_on(&app, 41), "command {code}");
    }
    let mut app = interp_app();
    app.world_mut().resource_mut::<Dialogue>().active = true;
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![
                cmd(11030, 0, vec![50, 100, 100, 100, 0, 0]),
                switch_cmd(41, 0, 0),
            ],
        )],
    });
    app.update();
    assert!(switch_on(&app, 41));
    assert_eq!(app.world().resource::<Messages<ScreenEffect>>().len(), 1);
}

fn message(text: &str) -> EventCommand {
    EventCommand {
        string: text.into(),
        ..cmd(10110, 0, vec![])
    }
}

#[test]
fn only_the_parallel_message_owner_waits_while_other_timers_continue() {
    let mut app = interp_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![message("Első"), switch_cmd(40, 0, 0)]),
            map_event(2, 4, vec![cmd(11410, 0, vec![1]), switch_cmd(41, 0, 0)]),
        ],
    });
    for _ in 0..10 {
        app.update();
    }
    assert!(app.world().resource::<Dialogue>().active);
    assert!(!switch_on(&app, 40));
    assert!(switch_on(&app, 41));
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert!(switch_on(&app, 40));
}

#[test]
fn another_parallel_message_cannot_replace_the_open_message() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![message("Első"), switch_cmd(40, 0, 0)]),
            map_event(2, 4, vec![switch_cmd(41, 0, 0), message("Második")]),
        ],
    });
    app.update();
    assert!(switch_on(&app, 41));
    assert_eq!(app.world().resource::<Dialogue>().boxes[0].lines[0], "Első");
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert!(switch_on(&app, 40));
    assert_eq!(
        app.world().resource::<Dialogue>().boxes[0].lines[0],
        "Második"
    );
}

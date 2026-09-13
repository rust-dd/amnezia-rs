use super::*;

mod storage;

#[test]
fn all_original_crystals_stop_at_the_save_menu_until_the_request_is_consumed() {
    let mut crystals = 0;
    for entry in std::fs::read_dir(format!("{}/maps", crate::assets::asset_root())).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let map = crate::assets::load_ron::<amnezia_data::Map>(path.to_str().unwrap());
        for event in &map.events {
            for page in &event.pages {
                let Some(index) = page
                    .commands
                    .iter()
                    .position(|command| command.code == 11910)
                else {
                    continue;
                };
                assert!(page.commands[index].params.is_empty());
                let mut app = interp_app();
                app.world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(event.id, page.commands.clone());
                app.update();
                assert!(app.world().resource::<EventSaveRequest>().0);
                let frame = &app.world().resource::<RunningEvent>().frame;
                assert!(frame.active(), "{} event {}", path.display(), event.id);
                assert_eq!(frame.ip, index + 1);
                for _ in 0..3 {
                    app.update();
                    assert_eq!(app.world().resource::<RunningEvent>().frame.ip, index + 1);
                    assert!(app.world().resource::<RunningEvent>().active());
                }
                app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
                app.update();
                assert!(!app.world().resource::<RunningEvent>().active());
                assert!(!app.world().resource::<EventSaveRequest>().0);
                crystals += 1;
            }
        }
    }
    assert_eq!(crystals, 16);
}

#[test]
fn saving_suspends_the_foreground_tail_and_the_remaining_parallel_work() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(2, 4, vec![switch_cmd(903, 0, 0)])],
    });
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            switch_cmd(901, 0, 0),
            cmd(11910, 0, vec![]),
            switch_cmd(902, 0, 0),
        ],
    );
    for _ in 0..4 {
        app.update();
        assert!(app.world().resource::<Switches>().get(901));
        assert!(!app.world().resource::<Switches>().get(902));
        assert!(!app.world().resource::<Switches>().get(903));
        assert!(app.world().resource::<EventSaveRequest>().0);
    }
    app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
    app.update();
    assert!(app.world().resource::<Switches>().get(902));
    assert!(app.world().resource::<Switches>().get(903));
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn a_parallel_save_holds_its_tail_other_background_events_and_new_autoruns() {
    let mut app = interp_app();
    let mut autorun = map_event(3, 3, vec![switch_cmd(906, 0, 0), switch_cmd(901, 1, 0)]);
    autorun.pages[0].condition.flags = 1;
    autorun.pages[0].condition.switch_a = 901;
    app.insert_resource(MapEvents {
        events: vec![
            map_event(
                1,
                4,
                vec![
                    switch_cmd(901, 0, 0),
                    cmd(11910, 0, vec![]),
                    switch_cmd(904, 0, 0),
                ],
            ),
            map_event(2, 4, vec![switch_cmd(905, 0, 0)]),
            autorun,
        ],
    });
    for _ in 0..4 {
        app.update();
        assert!(app.world().resource::<EventSaveRequest>().0);
        assert!(!app.world().resource::<RunningEvent>().active());
        for id in [904, 905, 906] {
            assert!(!app.world().resource::<Switches>().get(id), "switch {id}");
        }
    }
    app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
    app.update();
    for id in [904, 905, 906] {
        assert!(app.world().resource::<Switches>().get(id), "switch {id}");
    }
}

#[test]
fn saving_from_a_called_page_keeps_the_caller_and_resumes_both_tails_once() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(
            2,
            0,
            vec![
                switch_cmd(901, 2, 0),
                cmd(11910, 0, vec![]),
                switch_cmd(902, 2, 0),
            ],
        )],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(12330, 0, vec![1, 2, 1]), switch_cmd(903, 2, 0)]);
    for _ in 0..4 {
        app.update();
        let frame = &app.world().resource::<RunningEvent>().frame;
        assert_eq!(frame.event_id, 2);
        assert_eq!(frame.ip, 2);
        assert_eq!(frame.call_stack.len(), 1);
        assert!(switch_on(&app, 901));
        assert!(!switch_on(&app, 902));
        assert!(!switch_on(&app, 903));
    }
    app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
    for _ in 0..4 {
        app.update();
        assert!(switch_on(&app, 901));
        assert!(switch_on(&app, 902));
        assert!(switch_on(&app, 903));
        assert!(!app.world().resource::<RunningEvent>().active());
    }
}

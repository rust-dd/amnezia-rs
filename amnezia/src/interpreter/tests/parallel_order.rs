use super::*;

fn append(digit: i32) -> Vec<EventCommand> {
    vec![
        cmd(10220, 0, vec![0, 1, 1, 3, 0, 10]),
        cmd(10220, 0, vec![0, 1, 1, 1, 0, digit]),
    ]
}

fn value(app: &App) -> i32 {
    app.world().resource::<Variables>().get(1)
}

fn gated_map(id: u32, commands: Vec<EventCommand>) -> Event {
    let mut event = map_event(id, 4, commands);
    event.pages[0].condition.flags = 1;
    event.pages[0].condition.switch_a = 5;
    event
        .pages
        .insert(0, map_event(id, 0, vec![]).pages.remove(0));
    event
}

#[test]
fn a_common_event_preserves_its_waiting_stack_while_its_gate_is_off() {
    let mut app = interp_app();
    let commands = append(1)
        .into_iter()
        .chain([cmd(11410, 0, vec![0])])
        .chain(append(2))
        .collect();
    app.insert_resource(CommonEvents(vec![common(1, 4, 5, commands)]));
    set_switch(&mut app, 5, true);
    app.update();
    assert_eq!(value(&app), 1);
    set_switch(&mut app, 5, false);
    for _ in 0..3 {
        app.update();
        assert_eq!(value(&app), 1);
    }
    set_switch(&mut app, 5, true);
    app.update();
    assert_eq!(value(&app), 12);
}

#[test]
fn common_gates_are_checked_at_each_events_own_update() {
    for enabled in [false, true] {
        let mut app = interp_app();
        app.insert_resource(CommonEvents(vec![
            common(1, 4, 0, vec![switch_cmd(5, i32::from(!enabled), 0)]),
            common(2, 4, 5, append(2)),
        ]));
        set_switch(&mut app, 5, !enabled);
        app.update();
        assert_eq!(value(&app), if enabled { 2 } else { 0 });
    }
}

#[test]
fn a_common_event_can_finish_its_burst_after_turning_its_own_gate_off() {
    let mut app = interp_app();
    let commands = [switch_cmd(5, 1, 0)].into_iter().chain(append(1)).collect();
    app.insert_resource(CommonEvents(vec![common(1, 4, 5, commands)]));
    set_switch(&mut app, 5, true);
    app.update();
    assert_eq!(value(&app), 1);
    app.update();
    assert_eq!(value(&app), 1);
}

#[test]
fn later_map_events_see_page_changes_from_earlier_parallel_commands() {
    for enabled in [false, true] {
        let mut app = interp_app();
        app.insert_resource(MapEvents {
            events: vec![
                map_event(1, 4, vec![switch_cmd(5, i32::from(!enabled), 0)]),
                gated_map(2, append(2)),
            ],
        });
        set_switch(&mut app, 5, !enabled);
        app.update();
        assert_eq!(value(&app), if enabled { 2 } else { 0 });
    }
}

#[test]
fn reactivated_earlier_pages_keep_event_id_order() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![gated_map(1, append(1)), map_event(2, 4, append(2))],
    });
    app.update();
    assert_eq!(value(&app), 2);
    app.world_mut().resource_mut::<Variables>().set(1, 0);
    set_switch(&mut app, 5, true);
    app.update();
    assert_eq!(value(&app), 12);
}

#[test]
fn a_page_change_cancels_the_parallel_owner_including_its_called_stack() {
    for nested in [false, true] {
        let mut app = interp_app();
        let commands = if nested {
            vec![cmd(12330, 0, vec![1, 2, 1]), switch_cmd(10, 0, 0)]
        } else {
            vec![switch_cmd(5, 1, 0), switch_cmd(10, 0, 0)]
        };
        app.insert_resource(MapEvents {
            events: vec![
                gated_map(1, commands),
                map_event(2, 0, vec![switch_cmd(5, 1, 0), switch_cmd(11, 0, 0)]),
            ],
        });
        set_switch(&mut app, 5, true);
        app.update();
        assert!(!switch_on(&app, 10), "nested={nested}");
        assert!(!switch_on(&app, 11), "nested={nested}");
    }
}

#[test]
fn a_transient_page_change_discards_another_events_suspended_stack() {
    for foreground in [false, true] {
        let mut app = interp_app();
        let commands = append(1)
            .into_iter()
            .chain([cmd(11410, 0, vec![0]), cmd(11410, 0, vec![0])])
            .chain(append(2))
            .collect();
        app.insert_resource(MapEvents {
            events: vec![gated_map(1, commands)],
        });
        set_switch(&mut app, 5, true);
        app.update();
        assert_eq!(value(&app), 1);
        let pulse = vec![switch_cmd(5, 1, 0), switch_cmd(5, 0, 0)];
        if foreground {
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(0, pulse);
        } else {
            app.insert_resource(CommonEvents(vec![common(1, 4, 0, pulse)]));
        }
        app.update();
        if foreground {
            assert_eq!(value(&app), 1);
            app.update();
            assert_eq!(value(&app), 11);
        } else {
            assert_eq!(value(&app), 11);
        }
    }
}

#[test]
fn a_parallel_owners_replacement_page_waits_for_its_next_update() {
    let mut app = interp_app();
    let mut event = map_event(1, 4, vec![switch_cmd(5, 0, 0), switch_cmd(10, 0, 0)]);
    event
        .pages
        .push(gated_map(1, vec![switch_cmd(11, 0, 0)]).pages.remove(1));
    app.insert_resource(MapEvents {
        events: vec![event],
    });
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(switch_on(&app, 11));
}

#[test]
fn changing_a_called_pages_condition_does_not_cancel_its_different_owner() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(
                1,
                4,
                vec![cmd(12330, 0, vec![1, 2, 2]), switch_cmd(10, 0, 0)],
            ),
            gated_map(2, vec![switch_cmd(5, 1, 0), switch_cmd(11, 0, 0)]),
        ],
    });
    set_switch(&mut app, 5, true);
    app.update();
    assert!(!switch_on(&app, 5));
    assert!(switch_on(&app, 10));
    assert!(switch_on(&app, 11));
}

#[test]
fn losing_every_page_finishes_the_current_burst_but_pauses_future_updates() {
    let mut app = interp_app();
    let commands = append(1)
        .into_iter()
        .chain([
            switch_cmd(5, 1, 0),
            switch_cmd(10, 0, 0),
            cmd(11410, 0, vec![0]),
        ])
        .chain(append(2))
        .collect();
    let mut event = gated_map(1, commands);
    event.pages.remove(0);
    app.insert_resource(MapEvents {
        events: vec![event],
    });
    set_switch(&mut app, 5, true);
    app.update();
    assert!(switch_on(&app, 10));
    assert_eq!(value(&app), 1);
    for _ in 0..3 {
        app.update();
        assert_eq!(value(&app), 1);
    }
    set_switch(&mut app, 5, true);
    app.update();
    assert_eq!(value(&app), 11);
}

#[test]
fn restoring_the_same_page_after_no_page_cancels_the_old_owner() {
    let mut app = interp_app();
    let mut event = gated_map(
        1,
        vec![
            switch_cmd(5, 1, 0),
            switch_cmd(5, 0, 0),
            switch_cmd(10, 0, 0),
        ],
    );
    event.pages.remove(0);
    app.insert_resource(MapEvents {
        events: vec![event],
    });
    set_switch(&mut app, 5, true);
    app.update();
    assert!(switch_on(&app, 5));
    assert!(!switch_on(&app, 10));
}

use super::*;

mod saved_queue;

fn append(digit: i32) -> Vec<EventCommand> {
    vec![
        cmd(10220, 0, vec![0, 1, 1, 3, 0, 10]),
        cmd(10220, 0, vec![0, 1, 1, 1, 0, digit]),
    ]
}

fn value(app: &App) -> i32 {
    app.world().resource::<Variables>().get(1)
}

#[test]
fn queued_map_autoruns_execute_in_id_order_within_the_same_update() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(2, 3, append(2)), map_event(1, 3, append(1))],
    });
    app.update();
    assert_eq!(value(&app), 12);
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn a_finishing_foreground_event_continues_with_already_queued_map_work() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 3, append(1))],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, append(9));
    app.update();
    assert_eq!(value(&app), 91);
}

#[test]
fn a_queued_map_event_runs_above_the_common_autorun_without_skipping_its_first_command() {
    let mut app = interp_app();
    let mut commands = append(1);
    commands.push(switch_cmd(5, 1, 0));
    app.insert_resource(CommonEvents(vec![common(1, 3, 5, commands)]));
    app.world_mut().resource_mut::<Switches>().set(5, true);
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 3, append(2))],
    });
    app.update();
    assert_eq!(value(&app), 21);
    assert!(!switch_on(&app, 5));
}

#[test]
fn repeating_common_autoruns_share_one_update_command_budget() {
    let mut app = interp_app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        3,
        0,
        vec![cmd(10220, 0, vec![0, 1, 1, 1, 0, 1])],
    )]));
    app.update();
    assert_eq!(value(&app), 10_000);
}

#[test]
fn a_transient_page_replacement_discards_the_previously_queued_autorun() {
    let mut app = interp_app();
    let mut event = map_event(1, 3, append(1));
    let mut replacement = map_event(1, 0, vec![]).pages.remove(0);
    replacement.condition.flags = 1;
    replacement.condition.switch_a = 5;
    event.pages.push(replacement);
    app.insert_resource(MapEvents {
        events: vec![
            event,
            map_event(2, 4, vec![switch_cmd(5, 0, 0), switch_cmd(5, 1, 0)]),
        ],
    });
    app.update();
    assert_eq!(value(&app), 0);
}

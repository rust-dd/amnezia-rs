use super::*;

#[test]
fn a_disabled_common_condition_ignores_its_nonzero_switch_id() {
    for trigger in [3, 4] {
        let mut app = interp_app();
        let mut event = common(1, trigger, 5, vec![switch_cmd(10, 0, 0)]);
        event.switch_flag = false;
        app.insert_resource(CommonEvents(vec![event]));
        app.update();
        assert!(switch_on(&app, 10), "trigger={trigger}");
    }
}

#[test]
fn original_call_trigger_is_not_an_autorun_or_parallel_event() {
    let mut app = interp_app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        5,
        0,
        vec![switch_cmd(10, 0, 0)],
    )]));
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(!app.world().resource::<RunningEvent>().active());
    assert_eq!(app.world().resource::<ParallelPool>().count(), 0);
}

#[test]
fn legacy_common_event_assets_default_to_no_switch_condition() {
    let events =
        ron::from_str::<Vec<CommonEvent>>("[(id:1,name:\"\",trigger:5,switch_id:1,commands:[])]")
            .unwrap();
    assert!(!events[0].switch_flag);
}

use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Transition, TransitionPlugin};

mod callbacks;
mod clocks;

fn app() -> App {
    let mut app = interp_app();
    app.add_plugins(TransitionPlugin);
    app.update();
    app
}

fn tick(app: &mut App, frame: u32) {
    let waiting = app.world().resource::<Transition>().busy();
    app.insert_resource(crate::timing::SceneWait(waiting));
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

fn increment(id: i32) -> EventCommand {
    cmd(10220, 0, vec![0, id, id, 1, 0, 1])
}

fn counts(app: &App) -> [i32; 4] {
    let vars = app.world().resource::<Variables>();
    std::array::from_fn(|index| vars.get(index as u32 + 1))
}

#[test]
fn foreground_transition_resumes_after_the_parallel_stage() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, vec![increment(1)])]));
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![increment(2)])],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(11010, 0, vec![0]), increment(3)]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 1, 0, 0]);
    tick(&mut app, 34);
    assert_eq!(counts(&app), [1, 1, 0, 0]);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 1, 1, 0]);
    tick(&mut app, 36);
    assert_eq!(counts(&app), [2, 2, 1, 0]);
}

#[test]
fn common_transition_resumes_its_owner_even_after_disabling_its_gate() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![
        common(1, 4, 0, vec![increment(1)]),
        common(
            2,
            4,
            5,
            vec![switch_cmd(5, 1, 0), cmd(11010, 0, vec![0]), increment(2)],
        ),
        common(3, 4, 0, vec![increment(3)]),
    ]));
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![increment(4)])],
    });
    set_switch(&mut app, 5, true);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 0, 0, 0]);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 1, 1, 1]);
    tick(&mut app, 36);
    assert_eq!(counts(&app), [2, 1, 2, 2]);
}

#[test]
fn map_transition_resumes_at_its_event_without_repeating_common_or_earlier_map_events() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, vec![increment(1)])]));
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![increment(2)]),
            map_event(
                2,
                4,
                vec![
                    cmd(11010, 0, vec![0]),
                    increment(3),
                    cmd(11410, 0, vec![100]),
                ],
            ),
            map_event(3, 4, vec![increment(4)]),
        ],
    });
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 1, 0, 0]);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 1, 1, 1]);
    tick(&mut app, 36);
    assert_eq!(counts(&app), [2, 2, 1, 2]);
}

#[test]
fn resuming_a_transition_retains_the_same_ten_thousand_command_budget() {
    for parallel in [false, true] {
        let mut app = app();
        let mut commands = vec![increment(1); 9_998];
        commands.extend([cmd(11010, 0, vec![0]), increment(2), increment(3)]);
        if parallel {
            app.insert_resource(CommonEvents(vec![common(1, 4, 0, commands)]));
        } else {
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(7, commands);
        }
        tick(&mut app, 0);
        assert_eq!(counts(&app), [9_998, 0, 0, 0]);
        tick(&mut app, 35);
        assert_eq!(counts(&app), [9_998, 1, 0, 0], "parallel={parallel}");
        tick(&mut app, 36);
        assert_eq!(counts(&app), [9_998, 1, 1, 0]);
    }
}

#[test]
fn instantaneous_show_and_redundant_erase_resume_in_the_same_visit() {
    let mut app = app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        7,
        vec![
            cmd(11010, 0, vec![0]),
            increment(1),
            cmd(11010, 0, vec![17]),
            increment(2),
            cmd(11020, 0, vec![20]),
            increment(3),
        ],
    );
    tick(&mut app, 0);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 1, 1, 0]);
    assert!(app.world().resource::<Transition>().erased());
    assert!(!app.world().resource::<Transition>().event_erased);
}

#[test]
fn a_nested_make_way_visit_discards_the_async_request_but_keeps_the_command_position() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![
                cmd(11010, 0, vec![0]),
                increment(1),
                cmd(11410, 0, vec![100]),
            ],
        )],
    });
    assert!(!super::super::update_map_event(app.world_mut(), 1));
    assert!(!app.world().resource::<Transition>().busy());
    assert!(!app.world().resource::<Transition>().event_erased);
    assert_eq!(counts(&app), [0; 4]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 0, 0, 0]);
    assert!(!app.world().resource::<Transition>().busy());
}

#[test]
fn a_resumed_scene_request_commits_before_the_next_map_update() {
    let mut app = app();
    app.insert_resource(CommonEvents(vec![common(1, 4, 0, vec![increment(1)])]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![cmd(11010, 0, vec![19]), cmd(11910, 0, vec![])]);
    tick(&mut app, 0);
    assert!(!app.world().resource::<EventSaveRequest>().0);
    tick(&mut app, 1);
    assert_eq!(counts(&app), [1, 0, 0, 0]);
    assert!(app.world().resource::<EventSaveRequest>().0);
}

#[test]
fn a_transition_on_the_command_limit_does_not_replenish_the_resumed_visit() {
    let mut app = app();
    let mut commands = vec![increment(1); 9_999];
    commands.extend([cmd(11010, 0, vec![19]), increment(2)]);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, commands);
    tick(&mut app, 0);
    tick(&mut app, 1);
    assert_eq!(counts(&app), [9_999, 0, 0, 0]);
    tick(&mut app, 2);
    assert_eq!(counts(&app), [9_999, 1, 0, 0]);
}

#[test]
fn an_async_map_owner_without_an_active_page_finishes_its_suspended_burst() {
    let mut app = app();
    let mut event = map_event(
        1,
        4,
        vec![switch_cmd(5, 1, 0), cmd(11010, 0, vec![19]), increment(1)],
    );
    event.pages[0].condition.flags = 1;
    event.pages[0].condition.switch_a = 5;
    app.insert_resource(MapEvents {
        events: vec![event],
    });
    set_switch(&mut app, 5, true);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [0; 4]);
    tick(&mut app, 1);
    assert_eq!(counts(&app), [1, 0, 0, 0]);
    tick(&mut app, 2);
    assert_eq!(counts(&app), [1, 0, 0, 0]);
}

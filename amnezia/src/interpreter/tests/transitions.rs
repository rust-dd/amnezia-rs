use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Transition, TransitionPlugin};

fn app(commands: Vec<EventCommand>) -> App {
    let mut app = interp_app();
    app.add_plugins(TransitionPlugin);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, commands);
    app
}

fn tick(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

#[test]
fn screen_commands_wait_for_the_actual_transition_without_a_second_timer() {
    for (kind, duration) in [(0, 35), (17, 41), (19, 1), (20, 30)] {
        let mut app = app(vec![cmd(11010, 0, vec![kind]), switch_cmd(70, 0, 0)]);
        tick(&mut app, 0);
        for frame in 0..duration {
            tick(&mut app, frame);
            assert!(!switch_on(&app, 70), "kind {kind} at {frame}");
        }
        tick(&mut app, duration);
        assert!(switch_on(&app, 70), "kind {kind}");
        assert!(app.world().resource::<Transition>().erased());
    }
}

#[test]
fn default_command_uses_changed_setting_and_none_show_keeps_black() {
    let mut app = app(vec![
        cmd(10690, 0, vec![0, 20]),
        cmd(10690, 0, vec![1, 20]),
        cmd(11010, 0, vec![-1]),
        switch_cmd(70, 0, 0),
        cmd(11020, 0, vec![-1]),
        switch_cmd(71, 0, 0),
    ]);
    tick(&mut app, 0);
    tick(&mut app, 29);
    assert!(!switch_on(&app, 70));
    tick(&mut app, 30);
    assert!(switch_on(&app, 70));
    assert!(!switch_on(&app, 71));
    assert!(app.world().resource::<Transition>().erased());
    assert!(!app.world().resource::<Transition>().event_erased);
    tick(&mut app, 31);
    assert!(switch_on(&app, 71));
    assert!(app.world().resource::<Transition>().erased());
}

#[test]
fn repeated_erase_does_not_add_another_full_transition() {
    let mut app = app(vec![
        cmd(11010, 0, vec![0]),
        cmd(11010, 0, vec![17]),
        switch_cmd(70, 0, 0),
        cmd(11020, 0, vec![19]),
        switch_cmd(71, 0, 0),
    ]);
    tick(&mut app, 0);
    tick(&mut app, 35);
    assert!(app.world().resource::<Transition>().erased());
    assert!(!app.world().resource::<Transition>().busy());
    tick(&mut app, 36);
    assert!(switch_on(&app, 70));
    assert!(!switch_on(&app, 71));
    tick(&mut app, 37);
    assert!(switch_on(&app, 71));
    assert!(!app.world().resource::<Transition>().erased());
}

#[test]
fn transition_blocks_other_parallel_interpreters_until_it_finishes() {
    let mut app = app(vec![cmd(11010, 0, vec![17]), switch_cmd(70, 0, 0)]);
    app.insert_resource(MapEvents {
        events: vec![map_event(2, 4, vec![switch_cmd(71, 2, 0)])],
    });
    tick(&mut app, 0);
    assert!(switch_on(&app, 71));
    tick(&mut app, 40);
    assert!(switch_on(&app, 71));
    tick(&mut app, 41);
    assert!(switch_on(&app, 70));
    assert!(!switch_on(&app, 71));
}

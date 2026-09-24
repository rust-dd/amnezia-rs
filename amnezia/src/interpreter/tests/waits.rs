use super::*;
use crate::interpreter::frame::MAX_STEPS_PER_FRAME;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

fn app() -> App {
    let mut app = interp_app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app.update();
    app
}

fn start(app: &mut App, parallel: bool, commands: Vec<EventCommand>) {
    if parallel {
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 4, commands)],
        });
    } else {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(0, commands);
    }
}

fn check_wait(command: EventCommand, ticks: usize) {
    for parallel in [false, true] {
        let mut app = app();
        start(
            &mut app,
            parallel,
            vec![command.clone(), switch_cmd(10, 0, 0)],
        );
        for tick in 0..ticks {
            app.update();
            assert!(!switch_on(&app, 10), "tick {tick}, parallel {parallel}");
        }
        app.update();
        assert!(switch_on(&app, 10), "{ticks} ticks, parallel {parallel}");
    }
}

#[test]
fn timed_waits_include_the_command_update_without_fractional_drift() {
    for tenths in [1, 2, 3, 10, 100] {
        check_wait(cmd(11410, 0, vec![tenths]), tenths as usize * 6);
    }
}

#[test]
fn consecutive_zero_waits_each_yield_one_update_and_ignore_the_rpg2003_mode() {
    for parallel in [false, true] {
        let mut app = app();
        start(
            &mut app,
            parallel,
            vec![
                cmd(11410, 0, vec![0, 1]),
                switch_cmd(10, 0, 0),
                cmd(11410, 0, vec![0]),
                switch_cmd(11, 0, 0),
            ],
        );
        app.update();
        assert!(!switch_on(&app, 10));
        app.update();
        assert!(switch_on(&app, 10));
        assert!(!switch_on(&app, 11));
        app.update();
        assert!(switch_on(&app, 11));
    }
}

#[test]
fn timed_screen_and_picture_commands_share_the_original_wait_clock() {
    for tenths in [0, 1, 3] {
        let ticks = (tenths as usize * 6).max(1);
        for code in [11030, 11040] {
            check_wait(cmd(code, 0, vec![10, 20, 30, 31, tenths, 1]), ticks);
        }
        if tenths > 0 {
            check_wait(cmd(11050, 0, vec![3, 5, tenths, 1]), ticks);
        }
        let mut params = vec![1, 0, 100, 100, 0, 100, 0, 0, 100, 100, 100, 100, 0, 0];
        params.extend([tenths, 1]);
        check_wait(cmd(11120, 0, params), ticks);
    }
}

#[test]
fn stopping_shake_and_empty_pan_or_animation_do_not_create_a_wait() {
    for command in [
        cmd(11050, 0, vec![3, 5, 0, 1]),
        cmd(11060, 0, vec![2, 1, 0, 4, 1]),
        cmd(11210, 0, vec![9999, 10001, 1, 0]),
    ] {
        check_wait(command, 0);
    }
}

#[test]
fn pan_waits_count_logical_frames_from_the_command_update() {
    check_wait(cmd(11060, 0, vec![2, 1, 2, 3, 1]), 32);
}

#[test]
fn animation_waits_use_two_logical_ticks_per_data_frame() {
    for frames in [0, 1, 3, 30] {
        for parallel in [false, true] {
            let mut app = app();
            app.insert_resource(AnimationLibrary(vec![amnezia_data::AnimationDef {
                id: 1,
                name: String::new(),
                animation_name: String::new(),
                scope: 0,
                position: 0,
                frames: vec![amnezia_data::AnimationFrameDef { cells: vec![] }; frames],
                timings: vec![],
            }]));
            start(
                &mut app,
                parallel,
                vec![cmd(11210, 0, vec![1, 10001, 1, 0]), switch_cmd(10, 0, 0)],
            );
            for _ in 0..frames * 2 {
                app.update();
                assert!(!switch_on(&app, 10));
            }
            app.update();
            assert!(switch_on(&app, 10), "{frames} frames, parallel {parallel}");
        }
    }
}

#[test]
fn proceed_with_movement_continues_immediately_when_no_forced_route_is_pending() {
    check_wait(cmd(11340, 0, vec![]), 0);
}

#[test]
fn the_command_budget_can_defer_the_initial_wait_tick() {
    for parallel in [false, true] {
        let mut app = app();
        let mut commands = vec![cmd(12410, 0, vec![]); MAX_STEPS_PER_FRAME - 1];
        commands.push(cmd(11410, 0, vec![1]));
        commands.push(switch_cmd(10, 0, 0));
        start(&mut app, parallel, commands);
        app.update();
        for _ in 0..6 {
            app.update();
            assert!(!switch_on(&app, 10));
        }
        app.update();
        assert!(switch_on(&app, 10));
    }
}

#[test]
fn a_zero_wait_at_the_budget_boundary_keeps_its_unconsumed_tick() {
    for parallel in [false, true] {
        let mut app = app();
        let mut commands = vec![cmd(12410, 0, vec![]); MAX_STEPS_PER_FRAME - 1];
        commands.push(cmd(11410, 0, vec![0]));
        commands.push(switch_cmd(10, 0, 0));
        start(&mut app, parallel, commands);
        for _ in 0..2 {
            app.update();
            assert!(!switch_on(&app, 10));
        }
        app.update();
        assert!(switch_on(&app, 10));
    }
}

#[test]
fn time_and_movement_gates_precede_key_wait_polling() {
    let mut app = app();
    app.world_mut().resource_mut::<RunningEvent>().start(
        0,
        vec![cmd(11610, 0, vec![7, 1, 1, 1, 1]), switch_cmd(10, 0, 0)],
    );
    app.update();
    {
        let mut running = app.world_mut().resource_mut::<RunningEvent>();
        running.frame.wait = 1.0 / 60.0;
        running.frame.wait_movement = true;
    }
    app.world_mut().resource_mut::<Variables>().set(7, 777);
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]);
    for _ in 0..2 {
        app.update();
        assert_eq!(app.world().resource::<Variables>().get(7), 777);
        assert!(app.world().resource::<RunningEvent>().frame.key_pending);
        assert!(!switch_on(&app, 10));
    }
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::default();
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(7), 5);
    assert!(switch_on(&app, 10));
}

#[test]
fn menu_suspension_preserves_the_remaining_wait_without_an_extra_resume_frame() {
    let mut app = app();
    start(
        &mut app,
        false,
        vec![cmd(11410, 0, vec![1]), switch_cmd(10, 0, 0)],
    );
    app.update();
    let remaining = app.world().resource::<RunningEvent>().frame.wait;
    app.world_mut().resource_mut::<MenuOpen>().0 = true;
    for _ in 0..8 {
        app.update();
        assert_eq!(app.world().resource::<RunningEvent>().frame.wait, remaining);
    }
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    for _ in 0..5 {
        app.update();
        assert!(!switch_on(&app, 10));
    }
    app.update();
    assert!(switch_on(&app, 10));
}

#[test]
fn a_one_second_wait_finishes_on_logical_frame_61_at_every_render_rate() {
    use crate::timing::{GameFrames, TimingPlugin, logical::LogicalPlugin};
    #[derive(Resource, Default)]
    struct FinishedAt(Option<u32>);
    for fps in [15, 30, 60, 144] {
        let mut app = interp_app();
        app.add_plugins((TimingPlugin, LogicalPlugin))
            .init_resource::<FinishedAt>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .add_systems(
                Update,
                (|frames: Res<GameFrames>, switches: Res<Switches>, mut at: ResMut<FinishedAt>| {
                    if switches.get(10) && at.0.is_none() {
                        at.0 = Some(frames.frame);
                    }
                })
                .after(crate::interpreter::InterpreterStep),
            );
        app.update();
        start(
            &mut app,
            false,
            vec![cmd(11410, 0, vec![10]), switch_cmd(10, 0, 0)],
        );
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / f64::from(fps),
        )));
        for _ in 0..fps * 2 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<FinishedAt>().0,
            Some(61),
            "{fps} FPS"
        );
    }
}

use super::*;

fn open_owned_prompt(app: &mut App, parallel: bool) {
    let commands = vec![
        command(10730, vec![1, 30, 1]),
        increment(1),
        command(11410, vec![100]),
    ];
    if parallel {
        app.insert_resource(CommonEvents(vec![common(1, commands)]));
    } else {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(7, commands);
    }
    tick(app, 0);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    crate::dialogue::testing::dismiss(app.world_mut());
    app.world_mut().resource_mut::<Choice>().active = false;
}

#[test]
fn declining_a_parallel_inn_waits_for_window_close_but_foreground_continues_immediately() {
    for parallel in [false, true] {
        let mut app = interpreter_app();
        open_owned_prompt(&mut app, parallel);
        let frame = app.world().resource::<GameFrames>().frame;
        app.world_mut().resource_mut::<Choice>().result = Some(1);
        flow::accept(app.world_mut());
        for offset in 0..=8 {
            tick(&mut app, frame + offset);
            assert_eq!(
                counts(&app)[0],
                i32::from(!parallel || offset == 8),
                "parallel={parallel}, tick={offset}"
            );
        }
        assert_eq!(app.world().resource::<Inventory>().gold(), 100);
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
        assert!(heard(&mut app).is_empty());
    }
}

#[test]
fn paid_inn_keeps_only_message_motion_alive_until_the_seventh_raw_tick() {
    let mut app = interpreter_app();
    open_owned_prompt(&mut app, false);
    let frame = app.world().resource::<GameFrames>().frame;
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    flow::accept(app.world_mut());
    let gates = |scene: crate::world::ScenePause, pause: crate::dialogue::MessagePause| {
        (
            scene.paused(),
            scene.screen_effects_paused(),
            pause.paused(),
        )
    };
    for (offset, height) in [28, 22, 17, 11, 5, 0, 0].into_iter().enumerate() {
        tick(&mut app, frame + offset as u32);
        assert!(matches!(
            app.world().resource::<State>().phase,
            Phase::Closing
        ));
        assert_eq!(
            app.world()
                .resource::<Dialogue>()
                .lifecycle
                .message
                .half_height(80),
            height
        );
        assert_eq!(
            app.world_mut().run_system_once(gates).unwrap(),
            (true, true, false)
        );
        assert_eq!(counts(&app), [0; 3]);
    }
    tick(&mut app, frame + 7);
    assert!(matches!(
        app.world().resource::<State>().phase,
        Phase::FadeOut { .. }
    ));
    assert_eq!(
        app.world_mut().run_system_once(gates).unwrap(),
        (true, true, true)
    );
    tick(&mut app, frame + 42);
    tick(&mut app, frame + 77);
    assert_eq!(counts(&app), [1, 0, 0]);
    assert_eq!(
        app.world_mut().run_system_once(gates).unwrap(),
        (true, false, true)
    );
    tick(&mut app, frame + 78);
    assert_eq!(
        app.world_mut().run_system_once(gates).unwrap(),
        (false, false, false)
    );
}

#[test]
fn paid_parallel_inn_returns_to_message_tail_before_resuming_its_owner_next_tick() {
    let mut app = interpreter_app();
    open_owned_prompt(&mut app, true);
    let frame = app.world().resource::<GameFrames>().frame;
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    flow::accept(app.world_mut());
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![increment(2)]);
    for offset in 1..=77 {
        tick(&mut app, frame + offset);
        assert_eq!(
            counts(&app),
            [0, i32::from(offset == 77), 0],
            "tick={offset}"
        );
    }
    tick(&mut app, frame + 78);
    assert_eq!(counts(&app), [1, 1, 0]);
    assert_eq!(app.world().resource::<Inventory>().gold(), 70);
}

#[test]
fn reloading_clears_a_suspended_inn_owner_without_running_its_tail() {
    let mut app = interpreter_app();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![command(10730, vec![0, 0, 1]), increment(1)]);
    tick(&mut app, 0);
    assert!(
        app.world()
            .resource::<crate::interpreter::continuation::Continuation>()
            .waiting()
    );
    crate::session::clear_transient(app.world_mut());
    assert!(
        !app.world()
            .resource::<crate::interpreter::continuation::Continuation>()
            .waiting()
    );
    tick(&mut app, 80);
    assert_eq!(counts(&app), [0; 3]);
}

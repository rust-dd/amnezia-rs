use super::*;
use crate::interpreter::scenes::Requests;
use crate::transitions::Transition;

#[test]
fn an_unpublished_battle_cannot_consume_an_older_encounters_result() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![scene(10710), switch_cmd(10, 0, 0)]),
            map_event(2, 4, vec![cmd(11010, 0, vec![0]), cmd(11410, 0, vec![100])]),
        ],
    });
    app.update();
    assert!(app.world().resource::<Requests>().pending());
    let result = Some(crate::battle::BattleOutcome::Victory);
    app.world_mut().resource_mut::<BattleResult>().0 = result;
    app.world_mut().resource_mut::<Transition>().clear();
    app.update();
    assert!(!switch_on(&app, 10));
    assert_eq!(app.world().resource::<BattleResult>().0, result);
    assert_eq!(battle_requests(&mut app).len(), 1);
}

#[test]
fn an_earlier_scene_request_survives_a_later_screen_transition() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![scene(10720), switch_cmd(10, 0, 0)]),
            map_event(2, 4, vec![cmd(11010, 0, vec![0]), cmd(11410, 0, vec![100])]),
        ],
    });
    app.update();
    assert!(app.world().resource::<Requests>().pending());
    assert!(app.world().resource::<Transition>().busy());
    assert!(shop_requests(&mut app).is_empty());
    for _ in 0..3 {
        app.update();
        assert!(app.world().resource::<Requests>().pending());
        assert!(!switch_on(&app, 10));
    }
    app.world_mut().resource_mut::<Transition>().clear();
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(!app.world().resource::<Requests>().pending());
    assert_eq!(shop_requests(&mut app).len(), 1);
    app.update();
    assert!(switch_on(&app, 10));
}

#[test]
fn session_cleanup_discards_a_suspended_scene_and_its_owner_ticket() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![scene(10710)]),
            map_event(2, 4, vec![cmd(11010, 0, vec![0])]),
        ],
    });
    app.update();
    assert!(app.world().resource::<Requests>().pending());
    crate::session::clear_transient(app.world_mut());
    app.insert_resource(MapEvents { events: vec![] });
    app.update();
    assert!(!app.world().resource::<Requests>().pending());
    assert!(battle_requests(&mut app).is_empty());
    assert!(!app.world().resource::<RunningEvent>().active());
    assert_eq!(app.world().resource::<ParallelPool>().count(), 0);
}

#[test]
fn a_requested_scene_limits_a_called_frame_without_replaying_the_call() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![
            map_event(1, 4, vec![scene(11910)]),
            map_event(
                2,
                4,
                vec![cmd(12330, 0, vec![1, 3, 1]), switch_cmd(12, 0, 0)],
            ),
            map_event(3, 0, vec![switch_cmd(10, 0, 0), switch_cmd(11, 0, 0)]),
        ],
    });
    app.update();
    assert!(!switch_on(&app, 10));
    assert!(!switch_on(&app, 11));
    app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
    app.world_mut().resource_mut::<MapEvents>().events[0].pages[0].trigger = 0;
    app.update();
    assert!(switch_on(&app, 10));
    assert!(switch_on(&app, 11));
    assert!(switch_on(&app, 12));
}

#[test]
fn a_scene_request_keeps_the_last_live_timer_and_screen_tick() {
    use crate::screenfx::TintState;
    let mut app = super::super::super::scene_effects::app();
    {
        let mut clock = app.world_mut().resource_mut::<GameClock>();
        clock.remaining = 10.0;
        clock.start();
    }
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![cmd(11030, 0, vec![40, 100, 100, 100, 10, 0]), scene(11910)],
        )],
    });
    app.update();
    let remaining = app.world().resource::<GameClock>().remaining;
    assert!((remaining - (10.0 - 1.0 / 60.0)).abs() < 0.00001);
    assert_eq!(app.world().resource::<TintState>().tone()[0], 99.0);
    for _ in 0..3 {
        app.update();
        assert_eq!(app.world().resource::<GameClock>().remaining, remaining);
        assert_eq!(app.world().resource::<TintState>().tone()[0], 99.0);
    }
}

#[test]
fn replacement_order_is_identical_at_every_render_rate() {
    use crate::timing::logical::LogicalPlugin;
    for fps in [15, 30, 60, 144] {
        let mut app = app();
        app.add_plugins(LogicalPlugin);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / f64::from(fps),
        )));
        app.insert_resource(MapEvents {
            events: vec![
                map_event(1, 4, vec![scene(10710)]),
                map_event(2, 4, vec![scene(11910)]),
                map_event(3, 4, vec![switch_cmd(10, 0, 0), switch_cmd(11, 0, 0)]),
            ],
        });
        for _ in 0..fps {
            app.update();
            assert!(battle_requests(&mut app).is_empty());
        }
        assert!(app.world().resource::<EventSaveRequest>().0, "{fps} FPS");
        assert!(switch_on(&app, 10), "{fps} FPS");
        assert!(!switch_on(&app, 11), "{fps} FPS");
    }
}

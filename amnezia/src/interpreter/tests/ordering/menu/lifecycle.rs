use super::*;
use crate::audio::{AudioRequest, SystemSounds};
use crate::menu::Calling;
use crate::vehicles::{VehiclePlugin, Vehicles};

#[test]
fn airship_ascent_holds_a_menu_latch_until_a_later_stopped_update() {
    let mut app = fixture();
    app.add_plugins(VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    app.world_mut()
        .resource_scope(|world, mut vehicles: Mut<Vehicles>| {
            vehicles.set_location(2, 0, 5, 5);
            assert!(vehicles.toggle(world.resource::<MapData>(), (5, 5, 2), |_, _| false));
        });
    tick(&mut app, &[KeyCode::Escape]);
    assert!(app.world().resource::<Calling>().pending());
    for _ in 0..31 {
        assert!(!calling(&app));
        tick(&mut app, &[]);
    }
    assert!(!app.world().resource::<Vehicles>().airship_transitioning());
    assert!(!calling(&app));
    tick(&mut app, &[]);
    assert!(calling(&app));
}

#[test]
fn a_menu_request_resets_the_pose_and_plays_one_decision_sound() {
    for mode in [0, 4, 5] {
        let mut app = fixture();
        app.insert_resource(SystemSounds {
            decision: amnezia_data::SoundDef {
                name: "decision".into(),
                volume: 100,
                tempo: 100,
                ..default()
            },
            ..default()
        });
        tick(&mut app, &[KeyCode::Escape]);
        assert!(app.world().resource::<Messages<AudioRequest>>().is_empty());
        let world = app.world_mut();
        let (mut hero, mut route) = world
            .query::<(&mut Player, &mut RouteStepper)>()
            .single_mut(world)
            .unwrap();
        hero.frame = 2;
        route.animation.mode = mode;
        tick(&mut app, &[]);
        let world = app.world_mut();
        assert_eq!(
            world.query::<&Player>().single(world).unwrap().frame,
            if mode == 4 { 2 } else { 1 }
        );
        assert_eq!(
            world
                .resource_mut::<Messages<AudioRequest>>()
                .drain()
                .collect::<Vec<_>>(),
            [AudioRequest::play_sound("decision", &[100, 100])]
        );
        for _ in 0..20 {
            tick(&mut app, &[]);
            assert!(app.world().resource::<Messages<AudioRequest>>().is_empty());
        }
    }
}

#[test]
fn fresh_cancel_on_the_consumption_tick_preserves_a_second_latch() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    tick(&mut app, &[KeyCode::Escape]);
    assert!(calling(&app));
    assert!(app.world().resource::<Calling>().pending());
}

#[test]
fn session_cleanup_discards_both_latched_and_unpublished_menu_requests() {
    for consume in [false, true] {
        let mut app = fixture();
        tick(&mut app, &[KeyCode::Escape]);
        assert!(app.world().resource::<Calling>().pending());
        if consume {
            assert!(app.world_mut().resource_mut::<Calling>().consume());
        }
        crate::session::clear_transient(app.world_mut());
        assert!(!app.world().resource::<Calling>().pending());
        tick(&mut app, &[]);
        assert!(!calling(&app));
        assert!(!app.world().resource::<MenuOpen>().0);
    }
}

#[test]
fn the_request_frame_finishes_map_updates_before_the_erase_begins() {
    #[derive(Resource, Default)]
    struct Observed(Vec<(bool, bool)>);
    let mut app = fixture();
    app.init_resource::<Observed>().add_systems(
        Update,
        (|flow: Res<SceneFlow>, pause: crate::world::ScenePause, mut seen: ResMut<Observed>| {
            seen.0.push((flow.requested(), pause.paused()));
        })
        .after(crate::dialogue::MessageUpdate)
        .before(crate::interpreter::InterpreterStep),
    );
    tick(&mut app, &[KeyCode::Escape]);
    tick(&mut app, &[]);
    tick(&mut app, &[]);
    assert_eq!(
        app.world().resource::<Observed>().0,
        [(false, false), (true, false), (false, true)]
    );
}

#[test]
fn menu_capture_and_consumption_take_two_logical_updates_at_every_render_rate() {
    use crate::timing::{GameFrames, logical::LogicalPlugin};
    #[derive(Resource, Default)]
    struct RequestedAt(Option<u32>);
    for fps in [15, 30, 60, 144] {
        let mut app = fixture();
        app.add_plugins(LogicalPlugin)
            .init_resource::<RequestedAt>()
            .add_systems(
                Update,
                (|flow: Res<SceneFlow>, frames: Res<GameFrames>, mut at: ResMut<RequestedAt>| {
                    if flow.requested() && at.0.is_none() {
                        at.0 = Some(frames.frame);
                    }
                })
                .after(crate::dialogue::MessageUpdate)
                .before(crate::interpreter::InterpreterStep),
            );
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / f64::from(fps),
        )));
        let start = app.world().resource::<GameFrames>().frame;
        for frame in 0..fps {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            if frame == 0 {
                keys.press(KeyCode::Escape);
            }
            app.update();
        }
        assert_eq!(
            app.world().resource::<RequestedAt>().0,
            Some(start + 2),
            "{fps} FPS"
        );
    }
}

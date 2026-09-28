use super::*;

#[test]
fn same_visit_music_commands_are_flushed_before_inn_memory_without_replaying_requests() {
    let mut app = interpreter_app();
    app.add_plugins(AssetPlugin::default())
        .init_asset::<AudioSource>()
        .init_resource::<crate::audio::MemorizedBgm>()
        .add_systems(
            Update,
            crate::audio::flush.after(crate::interpreter::InterpreterStep),
        );
    let previous = crate::audio::BgmTrack {
        name: "Elven".into(),
        volume: 0.35,
        speed: 1.25,
        fade_in: 0.5,
    };
    let current = crate::audio::BgmTrack {
        name: "House".into(),
        volume: 1.0,
        speed: 1.2,
        fade_in: 0.25,
    };
    app.world_mut().write_message(previous.replay());
    app.world_mut().write_message(AudioRequest::MemorizeBgm);
    let mut play = command(11510, vec![250, 100, 120, 50]);
    play.string = "House".into();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![play, command(10730, vec![0, 0, 1]), increment(1)]);
    tick(&mut app, 0);
    assert_eq!(
        app.world().resource::<State>().before.as_ref(),
        Some(&current)
    );
    let saved = |capture: crate::audio::saved::Capture| capture.snapshot().unwrap();
    assert_eq!(
        app.world_mut().run_system_once(saved).unwrap().memorized,
        Some(previous.clone())
    );
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<AudioPlayer>>()
        .single(app.world())
        .unwrap();
    crate::audio::flush(app.world_mut());
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<AudioPlayer>>()
            .single(app.world())
            .unwrap(),
        entity
    );
    tick(&mut app, 36);
    tick(&mut app, 72);
    let music = app.world_mut().run_system_once(saved).unwrap();
    assert_eq!(music.current, Some(current));
    assert_eq!(music.memorized, Some(previous));
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<AudioPlayer>>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(counts(&app), [1, 0, 0]);
}

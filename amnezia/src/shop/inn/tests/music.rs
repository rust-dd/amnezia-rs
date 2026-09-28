use super::*;

#[test]
fn jingle_starts_after_erasure_and_uses_the_current_system_override_once() {
    let mut app = app();
    let mut overrides = crate::system_bgm::SystemBgm::default();
    overrides.change("Inn", &[2, 750, 80, 150, 50]);
    let expected = AudioRequest::music_once(overrides.get(2, &default()));
    app.insert_resource(overrides);
    open(&mut app, 0);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::FadeOutBgm { duration: 0.8 }]
    );
    for _ in 0..4 {
        app.world_mut().run_system_once(flow::advance).unwrap();
        assert!(heard(&mut app).is_empty());
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
    }
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert_eq!(heard(&mut app), [expected]);
    assert!(matches!(
        app.world().resource::<State>().phase,
        Phase::Resting { .. }
    ));
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
}

#[test]
fn absent_playback_finishes_without_an_extra_key_and_restores_silence() {
    let mut app = app();
    app.insert_resource(CurrentBgm::default());
    app.world_mut().resource_mut::<SystemMusic>().inn.name = "MissingAudio".into();
    app.world_mut().resource_mut::<Vitals>().set(1, 0, 0);
    app.world_mut()
        .resource_mut::<Vitals>()
        .set_states(1, vec![2, 3]);
    app.world_mut()
        .resource_mut::<Vitals>()
        .set_states(2, vec![2]);
    open(&mut app, 0);
    heard(&mut app);
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(
        matches!(heard(&mut app).as_slice(), [AudioRequest::BgmOnce(track)] if track.name == "MissingAudio")
    );
    assert!(!app.world().resource::<Vitals>().states(1).is_empty());
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert_eq!(
        heard(&mut app),
        [AudioRequest::StopBgm, AudioRequest::StopBgm]
    );
    assert!(app.world().resource::<Vitals>().states(1).is_empty());
    assert_eq!(app.world().resource::<Vitals>().states(2), [2]);
    assert!(matches!(
        app.world().resource::<State>().phase,
        Phase::FadeIn
    ));
}

#[test]
fn playing_music_waits_up_to_the_original_ten_second_deadline() {
    for elapsed in [Duration::ZERO, Duration::from_millis(9999)] {
        assert_eq!(flow::rest_completion(true, elapsed), None);
        assert_eq!(
            flow::rest_completion(false, elapsed),
            Some(Completion::PlaybackStopped)
        );
    }
    for elapsed in [Duration::from_secs(10), Duration::from_secs(11)] {
        assert_eq!(
            flow::rest_completion(true, elapsed),
            Some(Completion::Deadline)
        );
    }
}

#[test]
fn playback_completion_survives_audio_cleanup_and_the_return_transition() {
    let mut app = app();
    app.world_mut().resource_mut::<State>().phase = Phase::Resting {
        started: Duration::ZERO,
    };
    app.insert_resource(CurrentBgm::default());
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert_eq!(
        app.world().resource::<State>().completed,
        Some(Completion::PlaybackStopped)
    );
    assert_eq!(
        heard(&mut app),
        [AudioRequest::StopBgm, AudioRequest::StopBgm]
    );
    app.world_mut().resource_mut::<Transition>().clear();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(matches!(app.world().resource::<State>().phase, Phase::Idle));
    assert_eq!(
        app.world().resource::<State>().completed,
        Some(Completion::PlaybackStopped)
    );
    open(&mut app, 30);
    assert_eq!(app.world().resource::<State>().completed, None);
}

#[test]
fn intentionally_silent_stays_do_not_report_finished_playback() {
    let mut app = app();
    open(&mut app, 0);
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert_eq!(
        app.world().resource::<State>().completed,
        Some(Completion::Silent)
    );
}

#[test]
fn reload_discards_an_unfinished_inn_and_its_private_music_memory() {
    let mut app = app();
    open(&mut app, 0);
    assert!(app.world().resource::<State>().before.is_some());
    crate::session::clear_transient(app.world_mut());
    assert!(matches!(app.world().resource::<State>().phase, Phase::Idle));
    assert!(app.world().resource::<State>().before.is_none());
    assert!(!app.world().resource::<ShopOpen>().0);
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(heard(&mut app).is_empty());
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
}

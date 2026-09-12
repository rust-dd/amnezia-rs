use super::super::*;
use super::MusicState;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_resource::<CurrentBgm>()
        .init_resource::<MemorizedBgm>()
        .add_message::<AudioRequest>()
        .add_systems(Update, play_requests);
    app
}

fn track(fade_in: f32) -> AudioRequest {
    AudioRequest::Bgm {
        name: "AmneziaSaveMissingMusic".into(),
        volume: 0.25,
        speed: 1.1,
        fade_in,
    }
}

#[test]
fn a_same_name_request_updates_saved_fade_in_without_restarting_the_track() {
    let mut app = app();
    for fade_in in [1.5, 2.0] {
        app.world_mut().write_message(track(fade_in));
        app.update();
        assert_eq!(
            app.world()
                .resource::<CurrentBgm>()
                .track()
                .unwrap()
                .replay(),
            track(fade_in)
        );
    }
}

#[test]
fn memorizing_and_replaying_preserves_the_original_fade_in_setting() {
    let mut app = app();
    app.world_mut().write_message(track(1.5));
    app.world_mut().write_message(AudioRequest::MemorizeBgm);
    app.world_mut().write_message(AudioRequest::StopBgm);
    app.world_mut()
        .write_message(AudioRequest::PlayMemorizedBgm);
    app.update();
    assert_eq!(
        app.world()
            .resource::<CurrentBgm>()
            .track()
            .unwrap()
            .replay(),
        track(1.5)
    );
}

#[test]
fn legacy_track_snapshots_default_to_no_fade_in() {
    let track = ron::from_str::<BgmTrack>("(name:\"House\",volume:0.4,speed:0.8)").unwrap();
    assert_eq!(track.fade_in, 0.0);
}

#[test]
fn invalid_playback_parameters_cannot_enter_a_restored_music_state() {
    for (volume, speed, fade_in) in [
        (f32::NAN, 1.0, 0.0),
        (-0.1, 1.0, 0.0),
        (1.1, 1.0, 0.0),
        (1.0, 0.0, 0.0),
        (1.0, f32::INFINITY, 0.0),
        (1.0, 1.0, -0.1),
        (1.0, 1.0, f32::NAN),
    ] {
        let track = BgmTrack {
            name: "House".into(),
            volume,
            speed,
            fade_in,
        };
        for state in [
            MusicState {
                current: Some(track.clone()),
                memorized: None,
            },
            MusicState {
                current: None,
                memorized: Some(track),
            },
        ] {
            assert!(!state.valid());
        }
    }
    assert!(MusicState::default().valid());
}

#[test]
fn clearing_a_session_discards_any_waiting_music_restore() {
    let mut world = World::new();
    super::prepare(&mut world, 2, Some(MusicState::default()));
    assert!(world.get_resource::<super::Pending>().is_some());
    crate::session::clear_transient(&mut world);
    assert!(world.get_resource::<super::Pending>().is_none());
}

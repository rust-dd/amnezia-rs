use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn callback_audio_delivery_does_not_advance_the_player_fade_clock() {
    let mut world = World::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(250));
    world.insert_resource(time);
    world.insert_resource(CurrentBgm {
        fade: Some(BgmFade::fade_out(1.0, 1.0)),
        ..default()
    });
    world.init_resource::<crate::timing::logical::Step>();
    world
        .resource_mut::<crate::timing::logical::Step>()
        .callback = true;
    world.run_system_once(drive_bgm_fade).unwrap();
    assert_eq!(
        world
            .resource::<CurrentBgm>()
            .fade
            .as_ref()
            .unwrap()
            .volume(),
        1.0
    );
    world
        .resource_mut::<crate::timing::logical::Step>()
        .callback = false;
    world.run_system_once(drive_bgm_fade).unwrap();
    assert_eq!(
        world
            .resource::<CurrentBgm>()
            .fade
            .as_ref()
            .unwrap()
            .volume(),
        0.75
    );
}

#[test]
fn one_pass_music_preserves_the_system_parameters_and_silence_sentinels() {
    let music = MusicDef {
        name: "Inn".into(),
        fadein: 750,
        volume: 80,
        tempo: 150,
        ..default()
    };
    let AudioRequest::BgmOnce(track) = AudioRequest::music_once(&music) else {
        panic!("one pass requested");
    };
    assert_eq!(track.replay(), AudioRequest::from_music(&music));
    assert_eq!(track.speed, 1.5);
    assert_eq!(track.fade_in, 0.75);
    for name in ["", "(OFF)"] {
        assert_eq!(
            AudioRequest::music_once(&MusicDef {
                name: name.into(),
                ..music.clone()
            }),
            AudioRequest::StopBgm
        );
    }
}

#[test]
fn a_pending_audio_entity_is_not_mistaken_for_finished_playback() {
    let mut world = World::new();
    let pending = world.spawn_empty().id();
    world.insert_resource(CurrentBgm {
        entity: Some(pending),
        ..default()
    });
    let playing =
        |current: Res<CurrentBgm>, sinks: Query<&AudioSink>| current.playing_or_pending(&sinks);
    assert!(world.run_system_once(playing).unwrap());
    world.resource_mut::<CurrentBgm>().entity = None;
    assert!(!world.run_system_once(playing).unwrap());
}

use super::*;
use bevy::ecs::system::RunSystemOnce;

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

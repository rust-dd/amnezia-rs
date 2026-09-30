use super::*;
use bevy::ecs::system::RunSystemOnce;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .init_resource::<CurrentBgm>()
        .init_resource::<MemorizedBgm>()
        .add_message::<AudioRequest>()
        .add_systems(Update, play_requests);
    app
}

fn request(volume: f32, speed: f32) -> AudioRequest {
    AudioRequest::Bgm {
        name: "House".into(),
        volume,
        speed,
        fade_in: 0.0,
    }
}

fn assert_pending(world: &World, volume: f32, speed: f32) -> Entity {
    let entity = world.resource::<CurrentBgm>().entity.unwrap();
    assert!(world.get::<AudioSink>(entity).is_none());
    let settings = world.get::<PlaybackSettings>(entity).unwrap();
    assert_eq!(settings.volume, Volume::Linear(volume));
    assert_eq!(settings.speed, speed);
    entity
}

#[test]
fn same_frame_music_requests_retain_the_latest_pending_volume_and_tempo() {
    let mut app = app();
    app.world_mut().write_message(request(0.8, 1.0));
    app.world_mut().write_message(request(0.25, 1.4));
    app.update();
    assert_pending(app.world(), 0.25, 1.4);
}

#[test]
fn updating_an_unloaded_track_does_not_restart_it_or_change_one_shot_playback() {
    let mut app = app();
    app.world_mut()
        .write_message(AudioRequest::BgmOnce(BgmTrack {
            name: "House".into(),
            volume: 0.8,
            speed: 1.0,
            fade_in: 0.0,
        }));
    app.update();
    let original = assert_pending(app.world(), 0.8, 1.0);
    app.world_mut().write_message(request(0.4, 0.7));
    app.update();
    assert_eq!(assert_pending(app.world(), 0.4, 0.7), original);
    assert!(matches!(
        app.world().get::<PlaybackSettings>(original).unwrap().mode,
        bevy::audio::PlaybackMode::Once
    ));
}

#[test]
fn a_fade_updates_pending_playback_even_when_it_finishes_before_decoding() {
    let mut world = World::new();
    let entity = world
        .spawn(PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)))
        .id();
    world.insert_resource(CurrentBgm {
        entity: Some(entity),
        fade: Some(BgmFade::fade_in(0.8, 1.0)),
        ..default()
    });
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(500));
    world.insert_resource(time);
    world.run_system_once(drive_bgm_fade).unwrap();
    assert_pending(&world, 0.4, 1.0);
    world.run_system_once(drive_bgm_fade).unwrap();
    assert_pending(&world, 0.8, 1.0);
    assert!(world.resource::<CurrentBgm>().fade.is_none());
}

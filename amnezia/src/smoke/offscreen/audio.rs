use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackSettings, SpatialAudioSink};
use bevy::prelude::*;

pub(super) fn configure(app: &mut App, offscreen: bool) {
    if offscreen {
        app.add_systems(
            PostUpdate,
            mute.before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(Last, verify);
        info!("offscreen smoke audio is muted; playback and completion checks remain active");
    }
}

fn mute(mut settings: Query<&mut PlaybackSettings>) {
    for mut settings in &mut settings {
        if !settings.muted {
            settings.muted = true;
        }
    }
}

fn verify(sinks: Query<&AudioSink>, spatial: Query<&SpatialAudioSink>) {
    for sink in &sinks {
        assert!(sink.is_muted(), "offscreen audio must remain muted");
    }
    for sink in &spatial {
        assert!(sink.is_muted(), "offscreen spatial audio must remain muted");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::audio::Volume;

    #[test]
    fn offscreen_players_start_muted_without_changing_playback_or_original_gain() {
        let mut app = App::new();
        configure(&mut app, true);
        app.add_systems(
            PostUpdate,
            (|settings: Query<&PlaybackSettings>| {
                assert!(settings.iter().all(|settings| settings.muted));
            })
            .after(bevy::transform::TransformSystems::Propagate),
        );
        for settings in [
            PlaybackSettings::LOOP,
            PlaybackSettings::ONCE,
            PlaybackSettings::DESPAWN,
        ] {
            let settings = settings.with_volume(Volume::Linear(0.67)).with_speed(1.25);
            let entity = app.world_mut().spawn(settings).id();
            app.update();
            let actual = app.world().get::<PlaybackSettings>(entity).unwrap();
            assert!(actual.muted);
            assert!(!actual.paused);
            assert_eq!(
                std::mem::discriminant(&actual.mode),
                std::mem::discriminant(&settings.mode)
            );
            assert_eq!(actual.volume, settings.volume);
            assert_eq!(actual.speed, settings.speed);
            assert_eq!(actual.start_position, settings.start_position);
            assert_eq!(actual.duration, settings.duration);
        }
    }

    #[test]
    fn native_players_keep_their_requested_mute_state() {
        let mut app = App::new();
        configure(&mut app, false);
        let audible = app.world_mut().spawn(PlaybackSettings::LOOP).id();
        let muted = app.world_mut().spawn(PlaybackSettings::ONCE.muted()).id();
        app.update();
        assert!(!app.world().get::<PlaybackSettings>(audible).unwrap().muted);
        assert!(app.world().get::<PlaybackSettings>(muted).unwrap().muted);
    }
}

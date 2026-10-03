//! [`AudioRequest`] messages decouple the interpreter from Bevy audio playback.
//! BGM uses converted OGG or ambient WAV files; MIDI-only tracks remain silent.

use crate::assets::{asset_root, load_ron};
use amnezia_data::{MusicDef, SoundDef, SystemDef};
use bevy::audio::{AudioSink, AudioSinkPlayback, Volume};
use bevy::prelude::*;

#[cfg(test)]
mod asset_tests;
#[cfg(test)]
mod inn_tests;
mod paths;
mod playback;
mod request;
pub(crate) mod saved;

use playback::{BgmAction, drive_bgm_fade, play_requests};
use request::BgmFade;
pub use request::{AudioRequest, BgmTrack};

/// The requested name prevents autorun replays from restarting a track.
/// Volume/speed retain full target values so memorizing during a fade is stable.
#[derive(Resource, Default)]
pub(crate) struct CurrentBgm {
    entity: Option<Entity>,
    name: String,
    volume: f32,
    speed: f32,
    /// The requested envelope survives a completed ramp for memorize/save replay.
    fade_in: f32,
    fade: Option<BgmFade>,
}

impl CurrentBgm {
    pub(crate) fn playing_or_pending(&self, sinks: &Query<&AudioSink>) -> bool {
        self.entity
            .is_some_and(|entity| sinks.get(entity).map_or(true, |sink| !sink.empty()))
    }

    /// Includes unplayable MIDI-only tracks so memorize/restore preserves their silence.
    pub(crate) fn track(&self) -> Option<BgmTrack> {
        if self.name.is_empty() {
            return None;
        }
        Some(BgmTrack {
            name: self.name.clone(),
            volume: self.volume,
            speed: self.speed,
            fade_in: self.fade_in,
        })
    }

    /// RPG_RT's `music_stopping` guard makes same-name replays restart during fade-out.
    fn stopping(&self) -> bool {
        self.fade.as_ref().is_some_and(|fade| fade.stop_at_end)
    }

    /// Same-name requests preserve playback unless the current track is stopping.
    fn action_for(&self, name: &str, volume: f32, speed: f32) -> BgmAction {
        if self.name == name && !self.stopping() {
            if self.volume != volume || self.speed != speed {
                BgmAction::UpdateParams
            } else {
                BgmAction::Ignore
            }
        } else {
            BgmAction::Restart
        }
    }

    /// Fade from current gain; unplayable tracks stop immediately.
    fn start_fade_out(&mut self, duration: f32) {
        if self.entity.is_none() {
            self.name.clear();
            self.fade = None;
            return;
        }
        let start = self.fade.as_ref().map_or(self.volume, BgmFade::volume);
        self.fade = Some(BgmFade::fade_out(start, duration));
    }

    /// Test snapshot without an audio player.
    #[cfg(test)]
    pub(crate) fn with_track(name: &str, volume: f32, speed: f32) -> Self {
        Self {
            entity: None,
            name: name.to_string(),
            volume,
            speed,
            fade_in: 0.0,
            fade: None,
        }
    }
}

/// Event BGM memory (opcodes 11530/11540), separate from battle and inn snapshots.
#[derive(Resource, Default)]
pub struct MemorizedBgm(Option<BgmTrack>);

/// The interface and item-use sound effects loaded from the original system data.
#[derive(Resource, Default)]
pub struct SystemSounds {
    pub cursor: SoundDef,
    pub decision: SoundDef,
    pub cancel: SoundDef,
    pub buzzer: SoundDef,
    pub item: SoundDef,
}

/// Non-map scene music cached from `system.ron`.
#[derive(Resource, Default)]
pub struct SystemMusic {
    pub title: MusicDef,
    pub inn: MusicDef,
    pub gameover: MusicDef,
}

/// Queue a system sound effect (a [`SoundDef`] from [`SystemSounds`]); an
/// `(OFF)`/empty effect plays nothing.
pub fn play_system_se(audio: &mut MessageWriter<AudioRequest>, sound: &SoundDef) {
    if let Some(request) = AudioRequest::se(&sound.name, sound.volume, sound.tempo) {
        audio.write(request);
    }
}

pub struct AudioPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AudioRequests;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let system: SystemDef = load_ron(&format!("{}/system.ron", asset_root()));
        app.add_message::<AudioRequest>()
            .init_resource::<CurrentBgm>()
            .init_resource::<MemorizedBgm>()
            .insert_resource(SystemSounds {
                cursor: system.cursor_se,
                decision: system.decision_se,
                cancel: system.cancel_se,
                buzzer: system.buzzer_se,
                item: system.item_se,
            })
            .insert_resource(SystemMusic {
                title: system.title_music,
                inn: system.inn_music,
                gameover: system.gameover_music,
            })
            .add_systems(
                Update,
                (flush, drive_bgm_fade)
                    .chain()
                    .in_set(AudioRequests)
                    .after(crate::interpreter::InterpreterStep),
            );
    }
}

pub(crate) fn flush(world: &mut World) {
    if world.contains_resource::<AssetServer>()
        && world.contains_resource::<MemorizedBgm>()
        && world.contains_resource::<CurrentBgm>()
        && world.contains_resource::<Messages<AudioRequest>>()
    {
        world.run_system_cached(play_requests).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_request_updates_params_without_restarting() {
        let current = CurrentBgm::with_track("Field", 0.8, 1.0);
        assert_eq!(current.action_for("Field", 0.8, 1.0), BgmAction::Ignore);
        assert_eq!(
            current.action_for("Field", 0.5, 1.0),
            BgmAction::UpdateParams
        );
        assert_eq!(
            current.action_for("Field", 0.8, 1.5),
            BgmAction::UpdateParams
        );
        assert_eq!(current.action_for("House", 0.8, 1.0), BgmAction::Restart);
    }

    #[test]
    fn a_fading_out_track_restarts_on_a_same_name_request() {
        let mut current = CurrentBgm {
            entity: Some(Entity::PLACEHOLDER),
            name: "Elven".into(),
            volume: 0.8,
            speed: 1.0,
            fade_in: 0.0,
            fade: None,
        };
        current.start_fade_out(3.0);
        assert_eq!(current.action_for("Elven", 0.8, 1.0), BgmAction::Restart);
    }

    #[test]
    fn fade_out_without_a_playable_track_goes_silent_at_once() {
        let mut current = CurrentBgm::with_track("MidiOnly", 1.0, 1.0);
        current.start_fade_out(2.0);
        assert!(current.track().is_none());
        assert!(current.fade.is_none());
    }

    #[test]
    fn fade_out_ramps_a_playing_track_then_marks_it_stopping() {
        let mut current = CurrentBgm {
            entity: Some(Entity::PLACEHOLDER),
            name: "Elven".into(),
            volume: 0.8,
            speed: 1.0,
            fade_in: 0.0,
            fade: None,
        };
        current.start_fade_out(3.0);
        assert!(current.stopping(), "a fading-out track reports stopping");
        assert_eq!(
            current.fade.as_ref().map(BgmFade::volume),
            Some(0.8),
            "the fade begins from the track's current gain"
        );
    }

    #[test]
    fn memorize_round_trips_the_current_track() {
        let current = CurrentBgm::with_track("Elven", 0.66, 1.0);
        let memorized = MemorizedBgm(current.track());
        assert_eq!(
            memorized.0,
            Some(BgmTrack {
                name: "Elven".into(),
                volume: 0.66,
                speed: 1.0,
                fade_in: 0.0,
            })
        );
        assert_eq!(
            memorized.0.as_ref().map(BgmTrack::replay),
            Some(AudioRequest::Bgm {
                name: "Elven".into(),
                volume: 0.66,
                speed: 1.0,
                fade_in: 0.0,
            })
        );
    }

    #[test]
    fn memorize_of_silence_round_trips_to_nothing() {
        let silent = CurrentBgm::default();
        let memorized = MemorizedBgm(silent.track());
        assert!(memorized.0.is_none());
    }
}

//! Game audio: sound effects and background music, driven by the event
//! interpreter. The interpreter stays decoupled from Bevy's audio types by
//! emitting an [`AudioRequest`] message; this plugin consumes it and spawns the
//! actual players. A buffered message is chosen over a shared resource queue
//! because it is the idiomatic Bevy 0.19 producer/consumer channel and needs no
//! manual draining or clearing.
//!
//! Sound effects are WAV and play immediately. Music is mostly MIDI, which Bevy
//! cannot decode (and no synthesizer is installed), so a track plays only if a
//! converted `.ogg` or an ambient `.wav` exists under `audio/Music/`; a
//! MIDI-only track is skipped without an error or per-frame logging.

use crate::assets::asset_root;
use bevy::audio::Volume;
use bevy::prelude::*;

/// RM2000's sentinel BGM name meaning "silence": stop whatever is playing.
const BGM_OFF: &str = "(OFF)";

/// A playback request emitted by the interpreter. Volume is linear (0..1) and
/// speed a rate multiplier (1.0 = normal), already mapped from the command's
/// 0..100 volume and percent tempo so the player system stays a thin spawn step.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum AudioRequest {
    /// Play a one-shot sound effect; the entity despawns when it finishes.
    Sound {
        name: String,
        volume: f32,
        speed: f32,
    },
    /// Start looping background music, replacing any current track.
    Bgm {
        name: String,
        volume: f32,
        speed: f32,
    },
    /// Stop the current background music.
    StopBgm,
}

impl AudioRequest {
    /// Map a `PlaySound` (11550): `.string` is the SE name, params are
    /// `[volume, tempo, balance]` (volume 0..100, tempo a percent).
    pub fn play_sound(name: &str, params: &[i32]) -> Self {
        Self::Sound {
            name: name.to_string(),
            volume: linear_volume(params.first().copied().unwrap_or(100)),
            speed: playback_speed(params.get(1).copied().unwrap_or(100)),
        }
    }

    /// Map a `PlayBgm` (11510): `.string` is the track, params are
    /// `[fade_ms, volume, tempo, balance]`. The `(OFF)` sentinel and an empty
    /// name mean silence.
    pub fn play_bgm(name: &str, params: &[i32]) -> Self {
        if name.is_empty() || name == BGM_OFF {
            return Self::StopBgm;
        }
        Self::Bgm {
            name: name.to_string(),
            volume: linear_volume(params.get(1).copied().unwrap_or(100)),
            speed: playback_speed(params.get(2).copied().unwrap_or(100)),
        }
    }

    /// A looping BGM request from a System `Music` entry: its track `name`, its
    /// `0..=100` `volume`, and its percent `tempo`. An `(OFF)`/empty name stops
    /// the BGM. Used by the battle system for the battle / victory / game-over
    /// music.
    pub fn bgm(name: &str, volume: u32, tempo: u32) -> Self {
        if name.is_empty() || name == BGM_OFF {
            return Self::StopBgm;
        }
        Self::Bgm {
            name: name.to_string(),
            volume: linear_volume(volume as i32),
            speed: playback_speed(tempo as i32),
        }
    }

    /// A one-shot SE request from a System `Sound` entry, or `None` for an
    /// `(OFF)`/empty name (a disabled effect plays nothing). Used by the battle
    /// system for its per-hit sound effects.
    pub fn se(name: &str, volume: u32, tempo: u32) -> Option<Self> {
        if name.is_empty() || name == BGM_OFF {
            return None;
        }
        Some(Self::Sound {
            name: name.to_string(),
            volume: linear_volume(volume as i32),
            speed: playback_speed(tempo as i32),
        })
    }
}

/// A snapshot of a looping BGM — its track `name` and already-mapped linear
/// `volume` and playback `speed` — enough to replay it. The battle system
/// memorizes the map BGM at [`AudioRequest::bgm`] granularity when a fight starts
/// and restores it when the fight ends.
#[derive(Clone, Debug, PartialEq)]
pub struct BgmTrack {
    pub name: String,
    pub volume: f32,
    pub speed: f32,
}

/// Convert an RM2000 0..100 volume to a linear 0..1 gain.
fn linear_volume(percent: i32) -> f32 {
    percent.clamp(0, 100) as f32 / 100.0
}

/// Convert an RM2000 percent tempo (100 = normal) to a rate multiplier, guarding
/// a zero or negative that would otherwise stall playback.
fn playback_speed(tempo_percent: i32) -> f32 {
    if tempo_percent <= 0 {
        1.0
    } else {
        tempo_percent as f32 / 100.0
    }
}

/// The single active BGM: its entity (present only while a playable file loops)
/// and the requested track name, volume, and speed. The name keeps a re-requested
/// track — an autorun page replays it every cycle — from restarting, and a
/// MIDI-only track from being re-logged. The volume/speed are kept so a consumer
/// (the battle system) can memorize and later replay the exact track.
#[derive(Resource, Default)]
pub(crate) struct CurrentBgm {
    entity: Option<Entity>,
    name: String,
    volume: f32,
    speed: f32,
}

impl CurrentBgm {
    /// The currently-playing track as a replayable [`BgmTrack`], or `None` when
    /// nothing is playing (the name is cleared on stop). A MIDI-only track still
    /// reports here — its name is remembered even without a playable file — so a
    /// memorize/restore round-trip preserves the map's silence too.
    pub(crate) fn track(&self) -> Option<BgmTrack> {
        if self.name.is_empty() {
            return None;
        }
        Some(BgmTrack {
            name: self.name.clone(),
            volume: self.volume,
            speed: self.speed,
        })
    }

    /// Construct a `CurrentBgm` reporting `name` (with `volume`/`speed`) as the
    /// playing track, for tests that memorize the map BGM without spinning up the
    /// audio player.
    #[cfg(test)]
    pub(crate) fn with_track(name: &str, volume: f32, speed: f32) -> Self {
        Self {
            entity: None,
            name: name.to_string(),
            volume,
            speed,
        }
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AudioRequest>()
            .init_resource::<CurrentBgm>()
            .add_systems(Update, play_requests);
    }
}

/// Drain queued [`AudioRequest`]s: spawn a self-despawning player per sound
/// effect, and start or stop the looping BGM.
fn play_requests(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut requests: MessageReader<AudioRequest>,
    mut current: ResMut<CurrentBgm>,
) {
    for request in requests.read() {
        match request {
            AudioRequest::Sound {
                name,
                volume,
                speed,
            } => {
                if let Some(path) = resolve_audio("Sound", name, &["wav"]) {
                    commands.spawn((
                        AudioPlayer::new(asset_server.load(path)),
                        PlaybackSettings::DESPAWN
                            .with_volume(Volume::Linear(*volume))
                            .with_speed(*speed),
                    ));
                }
            }
            AudioRequest::Bgm {
                name,
                volume,
                speed,
            } => {
                start_bgm(
                    &mut commands,
                    &asset_server,
                    &mut current,
                    name,
                    *volume,
                    *speed,
                );
            }
            AudioRequest::StopBgm => stop_bgm(&mut commands, &mut current),
        }
    }
}

/// Switch the looping BGM to `name`. A request for the already-playing track is
/// ignored so RM2000's per-cycle replays stay seamless. Switching stops the old
/// track first; a MIDI-only track has no playable file, so it stops the old one
/// and stays silent until an `.ogg`/`.wav` for it exists.
fn start_bgm(
    commands: &mut Commands,
    asset_server: &AssetServer,
    current: &mut CurrentBgm,
    name: &str,
    volume: f32,
    speed: f32,
) {
    if current.name == name {
        return;
    }
    if let Some(entity) = current.entity.take() {
        commands.entity(entity).despawn();
    }
    current.name = name.to_string();
    current.volume = volume;
    current.speed = speed;
    match resolve_audio("Music", name, &["ogg", "wav"]) {
        Some(path) => {
            let entity = commands
                .spawn((
                    AudioPlayer::new(asset_server.load(path)),
                    PlaybackSettings::LOOP
                        .with_volume(Volume::Linear(volume))
                        .with_speed(speed),
                ))
                .id();
            current.entity = Some(entity);
        }
        None => debug!("bgm '{name}' has no playable audio (MIDI only); skipping"),
    }
}

/// Stop and forget the current BGM. Bevy has no built-in audio fade, so a
/// `FadeOutBGM` is honoured as an immediate stop.
fn stop_bgm(commands: &mut Commands, current: &mut CurrentBgm) {
    if let Some(entity) = current.entity.take() {
        commands.entity(entity).despawn();
    }
    current.name.clear();
}

/// Resolve an audio `name` (no extension) to its asset-relative path under
/// `audio/<subdir>/`, trying `exts` in order and matching the on-disk filename
/// case-insensitively (RM2000 names differ in case). `None` if nothing matches.
fn resolve_audio(subdir: &str, name: &str, exts: &[&str]) -> Option<String> {
    let dir = format!("{}/audio/{subdir}", asset_root());
    let files: Vec<String> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let lower = name.to_lowercase();
    for ext in exts {
        let target = format!("{lower}.{ext}");
        if let Some(file) = files.iter().find(|f| f.to_lowercase() == target) {
            return Some(format!("audio/{subdir}/{file}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_sound_maps_name_volume_and_speed() {
        let request = AudioRequest::play_sound("Bird1", &[90, 100, 50]);
        assert_eq!(
            request,
            AudioRequest::Sound {
                name: "Bird1".into(),
                volume: 0.9,
                speed: 1.0
            }
        );
    }

    #[test]
    fn play_sound_tempo_becomes_speed() {
        // Door2 plays at half tempo in the game data.
        let request = AudioRequest::play_sound("Door2", &[90, 50, 50]);
        assert_eq!(
            request,
            AudioRequest::Sound {
                name: "Door2".into(),
                volume: 0.9,
                speed: 0.5
            }
        );
    }

    #[test]
    fn play_sound_defaults_when_params_missing() {
        let request = AudioRequest::play_sound("Attack", &[]);
        assert_eq!(
            request,
            AudioRequest::Sound {
                name: "Attack".into(),
                volume: 1.0,
                speed: 1.0
            }
        );
    }

    #[test]
    fn play_bgm_reads_volume_from_second_param() {
        let request = AudioRequest::play_bgm("Morning", &[5000, 80, 100, 50]);
        assert_eq!(
            request,
            AudioRequest::Bgm {
                name: "Morning".into(),
                volume: 0.8,
                speed: 1.0
            }
        );
    }

    #[test]
    fn play_bgm_off_sentinel_and_empty_are_stop() {
        assert_eq!(
            AudioRequest::play_bgm("(OFF)", &[0, 100, 100, 50]),
            AudioRequest::StopBgm
        );
        assert_eq!(
            AudioRequest::play_bgm("", &[0, 0, 0, 0]),
            AudioRequest::StopBgm
        );
    }

    #[test]
    fn bgm_maps_system_music_volume_and_tempo() {
        assert_eq!(
            AudioRequest::bgm("Battle", 90, 100),
            AudioRequest::Bgm {
                name: "Battle".into(),
                volume: 0.9,
                speed: 1.0
            }
        );
        // An (OFF) or empty track stops the BGM rather than playing silence.
        assert_eq!(AudioRequest::bgm("(OFF)", 100, 100), AudioRequest::StopBgm);
        assert_eq!(AudioRequest::bgm("", 100, 100), AudioRequest::StopBgm);
    }

    #[test]
    fn se_maps_sound_and_skips_off() {
        assert_eq!(
            AudioRequest::se("Bite", 80, 100),
            Some(AudioRequest::Sound {
                name: "Bite".into(),
                volume: 0.8,
                speed: 1.0
            })
        );
        // A disabled effect plays nothing.
        assert_eq!(AudioRequest::se("(OFF)", 100, 100), None);
        assert_eq!(AudioRequest::se("", 100, 100), None);
    }

    #[test]
    fn volume_clamps_out_of_range() {
        assert_eq!(linear_volume(-10), 0.0);
        assert_eq!(linear_volume(150), 1.0);
        assert_eq!(linear_volume(50), 0.5);
    }

    #[test]
    fn speed_guards_zero_and_negative_tempo() {
        assert_eq!(playback_speed(0), 1.0);
        assert_eq!(playback_speed(-5), 1.0);
        assert_eq!(playback_speed(200), 2.0);
    }
}

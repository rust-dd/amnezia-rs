//! The interpreter→player audio channel: the [`AudioRequest`] message the event
//! interpreter (and the title/inn/game-over scenes) emit, the volume/tempo
//! mapping onto Bevy's player, and the [`BgmFade`] envelope that ramps a track's
//! gain for `PlayBGM`'s fade-in and `FadeOutBGM`'s fade-out.

use amnezia_data::MusicDef;
use bevy::prelude::*;

/// RM2000's sentinel BGM/SE name meaning "silence": stop whatever is playing.
const BGM_OFF: &str = "(OFF)";

/// A playback request emitted by the interpreter. Volume is a linear gain (0..1)
/// on RPG_RT's logarithmic scale and speed a rate multiplier (1.0 = normal),
/// already mapped from the command's 0..100 volume and percent tempo so the
/// player system stays a thin spawn step.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum AudioRequest {
    /// Play a one-shot sound effect; the entity despawns when it finishes.
    Sound {
        name: String,
        volume: f32,
        speed: f32,
    },
    /// Start looping background music, replacing any current track. `fade_in` is
    /// the ramp-up time in seconds (0 = start at full volume at once).
    Bgm {
        name: String,
        volume: f32,
        speed: f32,
        fade_in: f32,
    },
    /// Ramp the current BGM to silence over `duration` seconds, then stop it.
    FadeOutBgm { duration: f32 },
    /// Stop the current background music at once.
    StopBgm,
    /// Remember the current BGM so [`AudioRequest::PlayMemorizedBgm`] can restore it.
    MemorizeBgm,
    /// Replay the BGM remembered by [`AudioRequest::MemorizeBgm`] (silence if none).
    PlayMemorizedBgm,
}

impl AudioRequest {
    /// Map a `PlaySound` (11550): `.string` is the SE name, params are
    /// `[volume, tempo, balance]` (volume 0..100, tempo a percent).
    pub fn play_sound(name: &str, params: &[i32]) -> Self {
        Self::Sound {
            name: name.to_string(),
            volume: log_volume(params.first().copied().unwrap_or(100)),
            speed: playback_speed(params.get(1).copied().unwrap_or(100)),
        }
    }

    /// Map a `PlayBgm` (11510): `.string` is the track, params are
    /// `[fade_ms, volume, tempo, balance]`. The `(OFF)` sentinel and an empty
    /// name mean silence; `fade_ms` becomes the fade-in ramp.
    pub fn play_bgm(name: &str, params: &[i32]) -> Self {
        if name.is_empty() || name == BGM_OFF {
            return Self::StopBgm;
        }
        Self::Bgm {
            name: name.to_string(),
            volume: log_volume(params.get(1).copied().unwrap_or(100)),
            speed: playback_speed(params.get(2).copied().unwrap_or(100)),
            fade_in: fade_seconds(params.first().copied().unwrap_or(0)),
        }
    }

    /// Map a `FadeOutBGM` (11520): params[0] is the fade-out time in milliseconds.
    /// A zero (or missing) time stops the BGM at once.
    pub fn fade_out(params: &[i32]) -> Self {
        let duration = fade_seconds(params.first().copied().unwrap_or(0));
        if duration <= 0.0 {
            Self::StopBgm
        } else {
            Self::FadeOutBgm { duration }
        }
    }

    /// A looping BGM request from a System `Music` entry: its track `name`, its
    /// `0..=100` `volume`, and its percent `tempo`. An `(OFF)`/empty name stops
    /// the BGM. Used by the battle system for the battle / victory / game-over
    /// music.
    #[cfg(test)]
    pub fn bgm(name: &str, volume: u32, tempo: u32) -> Self {
        if name.is_empty() || name == BGM_OFF {
            return Self::StopBgm;
        }
        Self::Bgm {
            name: name.to_string(),
            volume: log_volume(volume as i32),
            speed: playback_speed(tempo as i32),
            fade_in: 0.0,
        }
    }

    /// A BGM request from a system [`MusicDef`] slot (title, inn, game over),
    /// honouring its `fadein`; an `(OFF)`/empty name stops the BGM.
    pub fn from_music(music: &MusicDef) -> Self {
        if music.name.is_empty() || music.name == BGM_OFF {
            return Self::StopBgm;
        }
        Self::Bgm {
            name: music.name.clone(),
            volume: log_volume(music.volume as i32),
            speed: playback_speed(music.tempo as i32),
            fade_in: fade_seconds(music.fadein as i32),
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
            volume: log_volume(volume as i32),
            speed: playback_speed(tempo as i32),
        })
    }
}

/// A snapshot of a looping BGM — its track `name` and already-mapped linear
/// `volume`, playback `speed` and fade-in duration. The battle system
/// memorizes the map BGM when a fight starts and restores it when the fight ends;
/// the inn and `MemorizeBGM` (11530) memorize it the same way.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BgmTrack {
    pub name: String,
    pub volume: f32,
    pub speed: f32,
    #[serde(default)]
    pub fade_in: f32,
}

impl BgmTrack {
    /// Replay from the beginning with the original fade-in setting.
    /// Restores a memorized BGM (battle teardown, inn checkout, `PlayMemorizedBGM`).
    pub fn replay(&self) -> AudioRequest {
        AudioRequest::Bgm {
            name: self.name.clone(),
            volume: self.volume,
            speed: self.speed,
            fade_in: self.fade_in,
        }
    }
}

/// A linear volume ramp between two gains over a fixed duration: `PlayBGM`'s
/// fade-in (silence → target) and `FadeOutBGM`'s fade-out (target → silence, then
/// stop). Bevy has no built-in audio fade, so the audio plugin advances this each
/// frame and writes the interpolated gain to the sink.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BgmFade {
    from: f32,
    to: f32,
    elapsed: f32,
    duration: f32,
    /// Whether the BGM stops once the ramp completes (a fade-out).
    pub(crate) stop_at_end: bool,
}

impl BgmFade {
    /// A fade-in from silence up to `target` over `duration` seconds.
    pub(crate) fn fade_in(target: f32, duration: f32) -> Self {
        Self {
            from: 0.0,
            to: target,
            elapsed: 0.0,
            duration,
            stop_at_end: false,
        }
    }

    /// A fade-out from `start` down to silence over `duration` seconds, stopping
    /// the BGM once complete.
    pub(crate) fn fade_out(start: f32, duration: f32) -> Self {
        Self {
            from: start,
            to: 0.0,
            elapsed: 0.0,
            duration,
            stop_at_end: true,
        }
    }

    /// The interpolated linear gain at the current elapsed time.
    pub(crate) fn volume(&self) -> f32 {
        if self.duration <= 0.0 {
            return self.to;
        }
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * t
    }

    /// Advance the ramp by `dt` seconds and return the new interpolated gain.
    pub(crate) fn advance(&mut self, dt: f32) -> f32 {
        self.elapsed += dt;
        self.volume()
    }

    /// Whether the ramp has run its full duration.
    pub(crate) fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Convert an RM2000 `0..=100` volume to a linear gain on RPG_RT's logarithmic
/// (DirectSound) scale, matching EasyRPG's `AudioDecoderBase::AdjustVolume`:
/// `100` → `1.0` (0 dB), `0` → silence, and the steps between attenuate along a
/// -35 dB curve. A plain linear mapping made half-volume play far too loud —
/// DirectSound puts `50` near -17.5 dB (~0.133 gain).
fn log_volume(percent: i32) -> f32 {
    let volume = percent.clamp(0, 100);
    if volume <= 0 {
        0.0
    } else {
        10f32.powf(-(35.0 / 20.0) * (1.0 - volume as f32 / 100.0))
    }
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

/// Convert an RM2000 fade time in milliseconds (clamped non-negative) to seconds.
fn fade_seconds(ms: i32) -> f32 {
    ms.max(0) as f32 / 1000.0
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
                volume: log_volume(90),
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
                volume: log_volume(90),
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
    fn play_bgm_reads_volume_from_second_param_and_fade_from_first() {
        let request = AudioRequest::play_bgm("Morning", &[3000, 80, 100, 50]);
        assert_eq!(
            request,
            AudioRequest::Bgm {
                name: "Morning".into(),
                volume: log_volume(80),
                speed: 1.0,
                fade_in: 3.0,
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
    fn fade_out_reads_duration_in_seconds_and_zero_is_immediate() {
        assert_eq!(
            AudioRequest::fade_out(&[2000]),
            AudioRequest::FadeOutBgm { duration: 2.0 }
        );
        // A zero-length fade is an instant stop.
        assert_eq!(AudioRequest::fade_out(&[0]), AudioRequest::StopBgm);
        assert_eq!(AudioRequest::fade_out(&[]), AudioRequest::StopBgm);
    }

    #[test]
    fn bgm_maps_system_music_volume_and_tempo() {
        assert_eq!(
            AudioRequest::bgm("Battle", 90, 100),
            AudioRequest::Bgm {
                name: "Battle".into(),
                volume: log_volume(90),
                speed: 1.0,
                fade_in: 0.0,
            }
        );
        // An (OFF) or empty track stops the BGM rather than playing silence.
        assert_eq!(AudioRequest::bgm("(OFF)", 100, 100), AudioRequest::StopBgm);
        assert_eq!(AudioRequest::bgm("", 100, 100), AudioRequest::StopBgm);
    }

    #[test]
    fn from_music_honours_name_volume_and_fade() {
        let music = MusicDef {
            name: "Theme".into(),
            volume: 100,
            tempo: 100,
            balance: 50,
            fadein: 1500,
        };
        assert_eq!(
            AudioRequest::from_music(&music),
            AudioRequest::Bgm {
                name: "Theme".into(),
                volume: 1.0,
                speed: 1.0,
                fade_in: 1.5,
            }
        );
        let off = MusicDef {
            name: "(OFF)".into(),
            ..music
        };
        assert_eq!(AudioRequest::from_music(&off), AudioRequest::StopBgm);
    }

    #[test]
    fn se_maps_sound_and_skips_off() {
        assert_eq!(
            AudioRequest::se("Bite", 80, 100),
            Some(AudioRequest::Sound {
                name: "Bite".into(),
                volume: log_volume(80),
                speed: 1.0
            })
        );
        // A disabled effect plays nothing.
        assert_eq!(AudioRequest::se("(OFF)", 100, 100), None);
        assert_eq!(AudioRequest::se("", 100, 100), None);
    }

    #[test]
    fn log_volume_matches_directsound_curve() {
        // 100 -> full (0 dB), 0/negative -> silence, above-range clamps to full.
        assert_eq!(log_volume(100), 1.0);
        assert_eq!(log_volume(0), 0.0);
        assert_eq!(log_volume(-10), 0.0);
        assert_eq!(log_volume(150), 1.0);
        // 50 -> ~-17.5 dB, ~0.1334 gain: far quieter than the old linear 0.5.
        let half = log_volume(50);
        assert!((half - 0.133_35).abs() < 1e-4, "log_volume(50) = {half}");
        assert!(
            half < 0.5,
            "the log curve must be quieter than linear at 50"
        );
        // Monotonically increasing across the range.
        assert!(log_volume(25) < log_volume(50));
        assert!(log_volume(50) < log_volume(75));
        assert!(log_volume(75) < log_volume(100));
    }

    #[test]
    fn speed_guards_zero_and_negative_tempo() {
        assert_eq!(playback_speed(0), 1.0);
        assert_eq!(playback_speed(-5), 1.0);
        assert_eq!(playback_speed(200), 2.0);
    }

    #[test]
    fn fade_out_ramps_to_silence_over_its_duration() {
        let mut fade = BgmFade::fade_out(1.0, 2.0);
        assert_eq!(fade.volume(), 1.0);
        assert_eq!(fade.advance(1.0), 0.5); // halfway down
        assert!(!fade.finished());
        assert_eq!(fade.advance(1.0), 0.0); // reached silence
        assert!(fade.finished());
        assert!(fade.stop_at_end);
        // Past the end stays clamped at silence.
        assert_eq!(fade.advance(5.0), 0.0);
    }

    #[test]
    fn fade_in_ramps_from_silence_up_to_target() {
        let mut fade = BgmFade::fade_in(0.8, 4.0);
        assert_eq!(fade.volume(), 0.0);
        assert_eq!(fade.advance(2.0), 0.4); // halfway up
        assert!(!fade.stop_at_end);
        assert_eq!(fade.advance(2.0), 0.8); // reached target
        assert!(fade.finished());
    }

    #[test]
    fn zero_duration_fade_is_immediate() {
        let fade = BgmFade::fade_in(1.0, 0.0);
        assert_eq!(fade.volume(), 1.0);
        assert!(fade.finished());
    }

    #[test]
    fn replay_rebuilds_the_track_request() {
        let track = BgmTrack {
            name: "Elven".into(),
            volume: 0.66,
            speed: 1.0,
            fade_in: 0.0,
        };
        assert_eq!(
            track.replay(),
            AudioRequest::Bgm {
                name: "Elven".into(),
                volume: 0.66,
                speed: 1.0,
                fade_in: 0.0,
            }
        );
    }
}

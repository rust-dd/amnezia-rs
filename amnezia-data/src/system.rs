use serde::{Deserialize, Serialize};

/// A background-music entry (RM2000 `Music`): the track `name` under
/// `audio/Music/` (`(OFF)` = silence), its `0..=100` `volume`, percent `tempo`
/// (`100` = normal), stereo `balance` (`50` = centred), and `fadein` in
/// milliseconds. The game maps `volume`/`tempo` onto the audio request's linear
/// volume and playback speed; `balance` and `fadein` are carried for the volume
/// and fade work that lands separately.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicDef {
    pub name: String,
    pub volume: u32,
    pub tempo: u32,
    #[serde(default)]
    pub balance: u32,
    #[serde(default)]
    pub fadein: u32,
}

/// A sound-effect entry (RM2000 `Sound`): the effect `name` under `audio/Sound/`
/// (`(OFF)` = silence), its `0..=100` `volume`, percent `tempo`, and stereo
/// `balance`. A `Sound` has no fade-in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundDef {
    pub name: String,
    pub volume: u32,
    pub tempo: u32,
    #[serde(default)]
    pub balance: u32,
}

/// The system font selection and scene audio, converted from `lcf::System`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemDef {
    /// Western font selection: 0 = RM2000-compatible, 1 = RMG2000-compatible.
    #[serde(default)]
    pub font_id: u32,
    pub title_music: MusicDef,
    pub battle_music: MusicDef,
    pub battle_end_music: MusicDef,
    pub gameover_music: MusicDef,
    pub inn_music: MusicDef,
    pub boat_music: MusicDef,
    pub ship_music: MusicDef,
    pub airship_music: MusicDef,
    pub cursor_se: SoundDef,
    pub decision_se: SoundDef,
    pub cancel_se: SoundDef,
    pub buzzer_se: SoundDef,
    pub battle_se: SoundDef,
    pub escape_se: SoundDef,
    pub enemy_attack_se: SoundDef,
    pub enemy_damaged_se: SoundDef,
    pub actor_damaged_se: SoundDef,
    pub dodge_se: SoundDef,
    pub enemy_defeated_se: SoundDef,
    pub item_se: SoundDef,
}

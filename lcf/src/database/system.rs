//! The system definition from the database (`ChunkData::system`, `0x16`). Unlike
//! the other LDB sections this is a single struct, not a `[count]`-prefixed list:
//! its data is a bare chunk stream. The audio-relevant fields are the title /
//! battle / victory / game-over / inn / vehicle music tracks and the UI and
//! battle sound effects, each a nested `Music` or `Sound` sub-struct. Chunk ids
//! follow liblcf `ChunkSystem`, `ChunkMusic`, and `ChunkSound`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// A background-music entry (RM2000 `rpg::Music`): the track `name` under
/// `audio/Music/` (the sentinel `(OFF)` means silence), its `0..=100` `volume`,
/// percent `tempo` (`100` = normal), stereo `balance` (`50` = centred), and
/// `fadein` in milliseconds. Omitted fields take these RM2000 defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Music {
    pub name: String,
    pub volume: u32,
    pub tempo: u32,
    pub balance: u32,
    pub fadein: u32,
}

impl Music {
    /// The RM2000 default (silent) music entry every field is initialised to
    /// before the chunk stream overrides whatever it carries.
    fn off() -> Self {
        Self {
            name: MUSIC_OFF.to_string(),
            volume: DEFAULT_VOLUME,
            tempo: DEFAULT_TEMPO,
            balance: DEFAULT_BALANCE,
            fadein: 0,
        }
    }
}

/// A sound-effect entry (RM2000 `rpg::Sound`): the effect `name` under
/// `audio/Sound/` (the sentinel `(OFF)` means silence), its `0..=100` `volume`,
/// percent `tempo`, and stereo `balance`. A `Sound` has no fade-in. Omitted
/// fields take the RM2000 defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    pub name: String,
    pub volume: u32,
    pub tempo: u32,
    pub balance: u32,
}

impl Sound {
    /// The RM2000 default (silent) sound entry.
    fn off() -> Self {
        Self {
            name: MUSIC_OFF.to_string(),
            volume: DEFAULT_VOLUME,
            tempo: DEFAULT_TEMPO,
            balance: DEFAULT_BALANCE,
        }
    }
}

/// The audio-relevant half of the RM2000 system definition: the music tracks and
/// sound effects the title, map, and battle scenes play. The many non-audio
/// system fields (party, transitions, battle-test data, engine limits) are read
/// past and dropped. `enemy_defeated_se` is liblcf's `enemy_death_se` (chunk
/// `0x33`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    pub title_music: Music,
    pub battle_music: Music,
    pub battle_end_music: Music,
    pub gameover_music: Music,
    pub inn_music: Music,
    pub boat_music: Music,
    pub ship_music: Music,
    pub airship_music: Music,
    pub cursor_se: Sound,
    pub decision_se: Sound,
    pub cancel_se: Sound,
    pub buzzer_se: Sound,
    pub battle_se: Sound,
    pub escape_se: Sound,
    pub enemy_attack_se: Sound,
    pub enemy_damaged_se: Sound,
    pub actor_damaged_se: Sound,
    pub dodge_se: Sound,
    pub enemy_defeated_se: Sound,
    pub item_se: Sound,
}

const SYSTEM_SECTION: u32 = 0x16;

const TITLE_MUSIC: u32 = 0x1F;
const BATTLE_MUSIC: u32 = 0x20;
const BATTLE_END_MUSIC: u32 = 0x21;
const INN_MUSIC: u32 = 0x22;
const BOAT_MUSIC: u32 = 0x23;
const SHIP_MUSIC: u32 = 0x24;
const AIRSHIP_MUSIC: u32 = 0x25;
const GAMEOVER_MUSIC: u32 = 0x26;

const CURSOR_SE: u32 = 0x29;
const DECISION_SE: u32 = 0x2A;
const CANCEL_SE: u32 = 0x2B;
const BUZZER_SE: u32 = 0x2C;
const BATTLE_SE: u32 = 0x2D;
const ESCAPE_SE: u32 = 0x2E;
const ENEMY_ATTACK_SE: u32 = 0x2F;
const ENEMY_DAMAGED_SE: u32 = 0x30;
const ACTOR_DAMAGED_SE: u32 = 0x31;
const DODGE_SE: u32 = 0x32;
const ENEMY_DEATH_SE: u32 = 0x33;
const ITEM_SE: u32 = 0x34;

const MUSIC_NAME: u32 = 0x01;
const MUSIC_FADEIN: u32 = 0x02;
const MUSIC_VOLUME: u32 = 0x03;
const MUSIC_TEMPO: u32 = 0x04;
const MUSIC_BALANCE: u32 = 0x05;

// A `Sound` shares the `Music` chunk ids but omits fade-in, so `0x02` is unused.
const SOUND_NAME: u32 = 0x01;
const SOUND_VOLUME: u32 = 0x03;
const SOUND_TEMPO: u32 = 0x04;
const SOUND_BALANCE: u32 = 0x05;

const MUSIC_OFF: &str = "(OFF)";
const DEFAULT_VOLUME: u32 = 100;
const DEFAULT_TEMPO: u32 = 100;
const DEFAULT_BALANCE: u32 = 50;

/// Parse a nested `Music` sub-struct (a bounded chunk stream): name `0x01`,
/// fadein `0x02`, volume `0x03`, tempo `0x04`, balance `0x05`. Omitted fields
/// keep the RM2000 defaults.
fn parse_music(data: &[u8]) -> Result<Music, LcfError> {
    let mut reader = Reader::new(data);
    let mut music = Music::off();
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        match id {
            MUSIC_NAME => music.name = decode_cp1250(field),
            MUSIC_FADEIN => music.fadein = Reader::new(field).varint()?,
            MUSIC_VOLUME => music.volume = Reader::new(field).varint()?,
            MUSIC_TEMPO => music.tempo = Reader::new(field).varint()?,
            MUSIC_BALANCE => music.balance = Reader::new(field).varint()?,
            _ => {}
        }
    }
    Ok(music)
}

/// Parse a nested `Sound` sub-struct (a bounded chunk stream): name `0x01`,
/// volume `0x03`, tempo `0x04`, balance `0x05`. Omitted fields keep the defaults.
fn parse_sound(data: &[u8]) -> Result<Sound, LcfError> {
    let mut reader = Reader::new(data);
    let mut sound = Sound::off();
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let field = reader.take(size)?;
        match id {
            SOUND_NAME => sound.name = decode_cp1250(field),
            SOUND_VOLUME => sound.volume = Reader::new(field).varint()?,
            SOUND_TEMPO => sound.tempo = Reader::new(field).varint()?,
            SOUND_BALANCE => sound.balance = Reader::new(field).varint()?,
            _ => {}
        }
    }
    Ok(sound)
}

/// Parse the system definition (`ChunkData::system` = `0x16`) out of an LDB byte
/// slice. The section is a single struct — a bare `[id][size][data]` chunk stream
/// with no `[count]` header — so every music (`0x1F..=0x26`) and sound
/// (`0x29..=0x34`) field is read directly; all other system fields are skipped by
/// their length. Missing music/sound fields default to the RM2000 silent entry.
pub fn parse_system(bytes: &[u8]) -> Result<System, LcfError> {
    let section = find_section(bytes, SYSTEM_SECTION, LcfError::MissingSystem)?;
    let mut reader = Reader::new(section);
    let mut system = System {
        title_music: Music::off(),
        battle_music: Music::off(),
        battle_end_music: Music::off(),
        gameover_music: Music::off(),
        inn_music: Music::off(),
        boat_music: Music::off(),
        ship_music: Music::off(),
        airship_music: Music::off(),
        cursor_se: Sound::off(),
        decision_se: Sound::off(),
        cancel_se: Sound::off(),
        buzzer_se: Sound::off(),
        battle_se: Sound::off(),
        escape_se: Sound::off(),
        enemy_attack_se: Sound::off(),
        enemy_damaged_se: Sound::off(),
        actor_damaged_se: Sound::off(),
        dodge_se: Sound::off(),
        enemy_defeated_se: Sound::off(),
        item_se: Sound::off(),
    };
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let data = reader.take(size)?;
        match id {
            TITLE_MUSIC => system.title_music = parse_music(data)?,
            BATTLE_MUSIC => system.battle_music = parse_music(data)?,
            BATTLE_END_MUSIC => system.battle_end_music = parse_music(data)?,
            GAMEOVER_MUSIC => system.gameover_music = parse_music(data)?,
            INN_MUSIC => system.inn_music = parse_music(data)?,
            BOAT_MUSIC => system.boat_music = parse_music(data)?,
            SHIP_MUSIC => system.ship_music = parse_music(data)?,
            AIRSHIP_MUSIC => system.airship_music = parse_music(data)?,
            CURSOR_SE => system.cursor_se = parse_sound(data)?,
            DECISION_SE => system.decision_se = parse_sound(data)?,
            CANCEL_SE => system.cancel_se = parse_sound(data)?,
            BUZZER_SE => system.buzzer_se = parse_sound(data)?,
            BATTLE_SE => system.battle_se = parse_sound(data)?,
            ESCAPE_SE => system.escape_se = parse_sound(data)?,
            ENEMY_ATTACK_SE => system.enemy_attack_se = parse_sound(data)?,
            ENEMY_DAMAGED_SE => system.enemy_damaged_se = parse_sound(data)?,
            ACTOR_DAMAGED_SE => system.actor_damaged_se = parse_sound(data)?,
            DODGE_SE => system.dodge_se = parse_sound(data)?,
            ENEMY_DEATH_SE => system.enemy_defeated_se = parse_sound(data)?,
            ITEM_SE => system.item_se = parse_sound(data)?,
            _ => {}
        }
    }
    Ok(system)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{make_ldb, subchunk, varint};
    use crate::{LcfError, Music, Sound, parse_system};

    /// Build a nested `Music`/`Sound` sub-struct: its sub-chunks then the struct's
    /// terminating id 0, as liblcf writes an embedded struct.
    fn nested(subchunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in subchunks {
            out.extend_from_slice(chunk);
        }
        out.extend(varint(0));
        out
    }

    #[test]
    fn parses_battle_music_and_sound_effects() {
        // battle_music (0x20): "Battle1", volume 80, tempo 120, balance 40, fadein 500.
        let battle_music = nested(&[
            subchunk(0x01, b"Battle1"),
            subchunk(0x02, &varint(500)),
            subchunk(0x03, &varint(80)),
            subchunk(0x04, &varint(120)),
            subchunk(0x05, &varint(40)),
        ]);
        // battle_end_music (0x21): "Victory1" with defaults elsewhere.
        let victory = nested(&[subchunk(0x01, b"Victory1")]);
        // enemy_attack_se (0x2F): "Sword", volume 90.
        let enemy_attack = nested(&[subchunk(0x01, b"Sword"), subchunk(0x03, &varint(90))]);
        // enemy_death_se (0x33) -> enemy_defeated_se: "Monster1".
        let enemy_death = nested(&[subchunk(0x01, b"Monster1")]);

        let mut section = Vec::new();
        section.extend(subchunk(0x20, &battle_music));
        section.extend(subchunk(0x21, &victory));
        section.extend(subchunk(0x2F, &enemy_attack));
        section.extend(subchunk(0x33, &enemy_death));
        // A non-audio field (party, 0x16) is skipped by its length.
        section.extend(subchunk(0x16, &[0x01, 0x00]));

        let ldb = make_ldb(&[(0x16, section)]);
        let system = parse_system(&ldb).unwrap();

        assert_eq!(
            system.battle_music,
            Music {
                name: "Battle1".to_string(),
                volume: 80,
                tempo: 120,
                balance: 40,
                fadein: 500,
            }
        );
        assert_eq!(system.battle_end_music.name, "Victory1");
        // battle_end_music's omitted fields fall back to the RM2000 defaults.
        assert_eq!(system.battle_end_music.volume, 100);
        assert_eq!(system.battle_end_music.balance, 50);
        assert_eq!(
            system.enemy_attack_se,
            Sound {
                name: "Sword".to_string(),
                volume: 90,
                tempo: 100,
                balance: 50,
            }
        );
        assert_eq!(system.enemy_defeated_se.name, "Monster1");
    }

    #[test]
    fn absent_fields_default_to_the_silent_entry() {
        // An empty system section: every music/sound field is the (OFF) default.
        let ldb = make_ldb(&[(0x16, Vec::new())]);
        let system = parse_system(&ldb).unwrap();
        assert_eq!(system.battle_music, Music::off());
        assert_eq!(system.enemy_damaged_se, Sound::off());
        assert_eq!(system.battle_se.name, "(OFF)");
        assert_eq!(system.dodge_se.volume, 100);
    }

    #[test]
    fn parse_system_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, vec![0])]);
        assert!(matches!(parse_system(&ldb), Err(LcfError::MissingSystem)));
    }
}

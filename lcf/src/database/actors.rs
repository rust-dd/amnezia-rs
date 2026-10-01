//! Actor definitions (`ChunkData::actors`, `0x0B`); chunk IDs follow liblcf `ChunkActor`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// A level-triggered skill acquisition (liblcf `rpg::Learning`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Learning {
    pub level: u32,
    pub skill_id: u32,
}

/// The six per-level stat curves an actor grows along. Each vector holds one
/// entry per level with length `max_level`; the value for level `L` sits at
/// index `L - 1`. The order mirrors the RM2000 `Parameters` blob: max HP, max
/// SP, attack, defense, spirit, agility. Values are clamped to non-negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatCurves {
    pub max_hp: Vec<i32>,
    pub max_sp: Vec<i32>,
    pub attack: Vec<i32>,
    pub defense: Vec<i32>,
    pub spirit: Vec<i32>,
    pub agility: Vec<i32>,
}

/// Playable actor defaults. Initial HP/SP come from the curves at `initial_level`.
/// Equipment IDs use 0 for an empty slot; dual wielding puts a weapon in `shield`.
/// `face_index` selects a 48×48 cell in the FaceSet's row-major 4×4 grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub character_name: String,
    pub character_index: u32,
    pub rename_skill: bool,
    pub skill_name: String,
    pub critical_hit: bool,
    pub critical_hit_chance: u32,
    pub attribute_ranks: Vec<u8>,
    pub state_ranks: Vec<u8>,
    pub id: u32,
    pub name: String,
    pub title: String,
    pub face_name: String,
    pub face_index: u32,
    pub initial_level: u32,
    pub max_level: u32,
    pub initial_hp: u32,
    pub initial_sp: u32,
    pub stat_curves: StatCurves,
    pub skills: Vec<Learning>,
    pub exp_base: u32,
    pub exp_inflation: u32,
    pub exp_correction: u32,
    pub weapon: u32,
    pub shield: u32,
    pub armor: u32,
    pub helmet: u32,
    pub accessory: u32,
    pub two_weapons: bool,
    pub fix_equipment: bool,
    pub unarmed_animation: u32,
}

const ACTOR_SECTION: u32 = 0x0B;
const ACTOR_NAME: u32 = 0x01;
const ACTOR_TITLE: u32 = 0x02;
const ACTOR_INITIAL_LEVEL: u32 = 0x07;
const ACTOR_FINAL_LEVEL: u32 = 0x08;
const ACTOR_FACE_NAME: u32 = 0x0F;
const ACTOR_FACE_INDEX: u32 = 0x10;
const ACTOR_TWO_WEAPON: u32 = 0x15;
const ACTOR_LOCK_EQUIPMENT: u32 = 0x16;
const ACTOR_PARAMETERS: u32 = 0x1F;
const ACTOR_SKILLS: u32 = 0x3F;
const LEARNING_LEVEL: u32 = 0x01;
const LEARNING_SKILL_ID: u32 = 0x02;
const ACTOR_EXP_BASE: u32 = 0x29;
const ACTOR_EXP_INFLATION: u32 = 0x2A;
const ACTOR_EXP_CORRECTION: u32 = 0x2B;
const ACTOR_INITIAL_EQUIPMENT: u32 = 0x33;
const ACTOR_UNARMED_ANIMATION: u32 = 0x38;
const ACTOR_DEFAULT_LEVEL: u32 = 1;
const ACTOR_DEFAULT_EXP_BASE: u32 = 30;
const ACTOR_DEFAULT_EXP_INFLATION: u32 = 30;
const ACTOR_DEFAULT_EXP_CORRECTION: u32 = 0;
const PARAMETER_STATS: usize = 6;
const EQUIPMENT_SLOTS: usize = 5;

/// Number of levels stored in a `Parameters` blob: six equal Int16 arrays, two
/// bytes per level. Clamped to at least one so callers can always index level 1.
fn curve_len(parameters: &[u8]) -> usize {
    (parameters.len() / (PARAMETER_STATS * 2)).max(1)
}

/// Read one stat curve (`stat_index` in `0..6`) out of the `Parameters` blob
/// into a `max_level`-long vector. The blob concatenates six equal Int16 (LE)
/// arrays; the per-array stride comes from the blob itself, and levels past the
/// stored curve repeat its last value. Each value is clamped to non-negative.
fn read_curve(parameters: &[u8], stat_index: usize, max_level: u32) -> Vec<i32> {
    let stride = parameters.len() / PARAMETER_STATS;
    let stored = curve_len(parameters);
    (0..max_level as usize)
        .map(|level| {
            let clamped = level.min(stored - 1);
            let byte = stat_index * stride + clamped * 2;
            parameters
                .get(byte..byte + 2)
                .map(|pair| i16::from_le_bytes([pair[0], pair[1]]).max(0) as i32)
                .unwrap_or(0)
        })
        .collect()
}

/// Split the actor `Parameters` chunk (`0x1F`) into the six per-level stat
/// curves — max HP, max SP, attack, defense, spirit, agility, in that order
/// (liblcf `RawStruct<rpg::Parameters>::ReadLcf`).
fn parse_stat_curves(parameters: &[u8], max_level: u32) -> StatCurves {
    StatCurves {
        max_hp: read_curve(parameters, 0, max_level),
        max_sp: read_curve(parameters, 1, max_level),
        attack: read_curve(parameters, 2, max_level),
        defense: read_curve(parameters, 3, max_level),
        spirit: read_curve(parameters, 4, max_level),
        agility: read_curve(parameters, 5, max_level),
    }
}

/// Decode the actor `initial_equipment` struct (`0x33`): five Int16 (LE) item
/// ids in slot order — weapon, shield, armor, helmet, accessory. Trailing slots
/// the blob omits read as 0, and negative ids clamp to 0.
fn read_equipment(data: &[u8]) -> [u32; EQUIPMENT_SLOTS] {
    let mut slots = [0u32; EQUIPMENT_SLOTS];
    for (slot, value) in slots.iter_mut().enumerate() {
        let byte = slot * 2;
        if let Some(pair) = data.get(byte..byte + 2) {
            *value = i16::from_le_bytes([pair[0], pair[1]]).max(0) as u32;
        }
    }
    slots
}

/// Decode the count-prefixed learning list (`0x3F`): level `0x01`, skill `0x02`.
/// An omitted skill ID defaults to 1; explicit 0 denotes a blank row and is dropped.
fn parse_learnings(data: &[u8]) -> Result<Vec<Learning>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut learnings = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _entry_id = reader.varint()?;
        let mut learning = Learning {
            level: 1,
            skill_id: 1,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                LEARNING_LEVEL => learning.level = Reader::new(sub_data).varint()?,
                LEARNING_SKILL_ID => learning.skill_id = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        if learning.skill_id != 0 {
            learnings.push(learning);
        }
    }
    Ok(learnings)
}

/// Parse the actor table (`ChunkData::actors` = `0x0B`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkActor`): name `0x01`, title `0x02`,
/// face_name `0x0F`, face_index `0x10`, initial_level `0x07`, final_level
/// `0x08`, two_weapon `0x15`, lock_equipment `0x16`, parameters `0x1F`, skills
/// `0x3F` (the `rpg::Learning` list), exp_base `0x29`, exp_inflation `0x2A`,
/// exp_correction `0x2B`, initial_equipment `0x33` (five Int16 item ids),
/// unarmed_animation `0x38`.
pub fn parse_actors(bytes: &[u8]) -> Result<Vec<Actor>, LcfError> {
    let section = find_section(bytes, ACTOR_SECTION, LcfError::MissingActors)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut actors = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut name = String::new();
        let mut title = String::new();
        let mut character_name = String::new();
        let mut character_index = 0;
        let mut rename_skill = false;
        let mut skill_name = String::new();
        let mut face_name = String::new();
        let mut face_index = 0;
        let mut initial_level = ACTOR_DEFAULT_LEVEL;
        let mut final_level: Option<u32> = None;
        let mut parameters: &[u8] = &[];
        let mut skills = Vec::new();
        let mut exp_base = ACTOR_DEFAULT_EXP_BASE;
        let mut exp_inflation = ACTOR_DEFAULT_EXP_INFLATION;
        let mut exp_correction = ACTOR_DEFAULT_EXP_CORRECTION;
        let mut equipment = [0u32; EQUIPMENT_SLOTS];
        let mut two_weapons = false;
        let mut fix_equipment = false;
        let mut unarmed_animation = 1;
        let mut state_ranks = Vec::new();
        let mut attribute_ranks = Vec::new();
        let mut critical_hit = true;
        let mut critical_hit_chance = 30;
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                ACTOR_NAME => name = decode_cp1250(sub_data),
                ACTOR_TITLE => title = decode_cp1250(sub_data),
                0x03 => character_name = decode_cp1250(sub_data),
                0x04 => character_index = Reader::new(sub_data).varint()?,
                0x42 => rename_skill = Reader::new(sub_data).varint()? != 0,
                0x43 => skill_name = decode_cp1250(sub_data),
                ACTOR_FACE_NAME => face_name = decode_cp1250(sub_data),
                ACTOR_FACE_INDEX => face_index = Reader::new(sub_data).varint()?,
                ACTOR_INITIAL_LEVEL => initial_level = Reader::new(sub_data).varint()?,
                ACTOR_FINAL_LEVEL => final_level = Some(Reader::new(sub_data).varint()?),
                0x09 => critical_hit = Reader::new(sub_data).varint()? != 0,
                0x0A => critical_hit_chance = Reader::new(sub_data).varint()?,
                ACTOR_TWO_WEAPON => two_weapons = Reader::new(sub_data).varint()? != 0,
                ACTOR_LOCK_EQUIPMENT => fix_equipment = Reader::new(sub_data).varint()? != 0,
                ACTOR_PARAMETERS => parameters = sub_data,
                ACTOR_SKILLS => skills = parse_learnings(sub_data)?,
                ACTOR_EXP_BASE => exp_base = Reader::new(sub_data).varint()?,
                ACTOR_EXP_INFLATION => exp_inflation = Reader::new(sub_data).varint()?,
                ACTOR_EXP_CORRECTION => exp_correction = Reader::new(sub_data).varint()?,
                ACTOR_INITIAL_EQUIPMENT => equipment = read_equipment(sub_data),
                ACTOR_UNARMED_ANIMATION => unarmed_animation = Reader::new(sub_data).varint()?,
                0x48 => state_ranks = sub_data.to_vec(),
                0x4A => attribute_ranks = sub_data.to_vec(),
                _ => {}
            }
        }
        // The omitted editor-default final level equals the stored curve length.
        let max_level = final_level
            .filter(|&l| l > 0)
            .unwrap_or(curve_len(parameters) as u32);
        let stat_curves = parse_stat_curves(parameters, max_level);
        let level_index = (initial_level.max(1) - 1).min(max_level.saturating_sub(1)) as usize;
        let initial_hp = stat_curves.max_hp.get(level_index).copied().unwrap_or(0) as u32;
        let initial_sp = stat_curves.max_sp.get(level_index).copied().unwrap_or(0) as u32;
        actors.push(Actor {
            character_name,
            character_index,
            rename_skill,
            skill_name,
            critical_hit,
            critical_hit_chance,
            attribute_ranks,
            state_ranks,
            id,
            name,
            title,
            face_name,
            face_index,
            initial_level,
            max_level,
            initial_hp,
            initial_sp,
            stat_curves,
            skills,
            exp_base,
            exp_inflation,
            exp_correction,
            weapon: equipment[0],
            shield: equipment[1],
            armor: equipment[2],
            helmet: equipment[3],
            accessory: equipment[4],
            two_weapons,
            fix_equipment,
            unarmed_animation,
        });
    }
    Ok(actors)
}

#[cfg(test)]
mod tests;

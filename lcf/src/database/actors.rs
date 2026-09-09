//! Actor (playable character) definitions from the database
//! (`ChunkData::actors`, `0x0B`). Beyond the identity fields, each actor carries
//! six per-level stat curves packed into one `Parameters` blob, an experience
//! curve stored as three scalar chunks, and its initial equipment — the numbers
//! a faithful level-up and equip screen need. Chunk ids follow liblcf
//! `ChunkActor`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// One entry in an actor's skill-learning list (liblcf `rpg::Learning`): the
/// `level` at which the actor learns skill `skill_id`. The actor knows every
/// skill whose `level` is at or below its current level.
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

/// An actor (playable character) definition: the fields the status, equip, and
/// message screens plus the level-up system need. `name` expands the `\N[k]`
/// message control code; `initial_hp`/`initial_sp` are the max-HP/max-SP curve
/// values at `initial_level`. `stat_curves` holds the full per-level tables and
/// `exp_base`/`exp_inflation`/`exp_correction` parameterise the RM2000
/// experience curve.
///
/// `weapon`/`shield`/`armor`/`helmet`/`accessory` are the item ids the actor
/// starts equipped with (0 = that slot is empty). `two_weapons` marks a
/// dual-wielding actor (the shield slot holds a second weapon), `fix_equipment`
/// an actor whose gear can't be changed, and `unarmed_animation` the battle
/// animation id used when the actor attacks with no weapon.
///
/// `face_name` names the actor's FaceSet graphic and `face_index` selects its
/// 48×48 portrait cell within that sheet's 4×4 grid. `skills` is the actor's
/// `Learning` list: the `(level, skill_id)` pairs it learns as it levels up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
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

/// Parse an actor's `skills` list (`0x3F`), the array of `rpg::Learning` entries
/// that says which skill the actor learns at which level. Same nested
/// struct-list shape as elsewhere in the LCF: a `[count]` header then, per entry,
/// a 1-based index id and a chunk stream (level `0x01`, skill_id `0x02`). An entry
/// whose skill id stays 0 (chunk omitted) is dropped, matching RM2000 ignoring a
/// blank learning row.
fn parse_learnings(data: &[u8]) -> Result<Vec<Learning>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut learnings = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _entry_id = reader.varint()?;
        let mut learning = Learning {
            level: 1,
            skill_id: 0,
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
        let mut unarmed_animation = 0;
        let mut state_ranks = Vec::new();
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
                ACTOR_FACE_NAME => face_name = decode_cp1250(sub_data),
                ACTOR_FACE_INDEX => face_index = Reader::new(sub_data).varint()?,
                ACTOR_INITIAL_LEVEL => initial_level = Reader::new(sub_data).varint()?,
                ACTOR_FINAL_LEVEL => final_level = Some(Reader::new(sub_data).varint()?),
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
                _ => {}
            }
        }
        // RM2000 omits `final_level` when it equals the editor default (50),
        // which is exactly the length of the stored parameter curve, so fall
        // back to that.
        let max_level = final_level
            .filter(|&l| l > 0)
            .unwrap_or(curve_len(parameters) as u32);
        let stat_curves = parse_stat_curves(parameters, max_level);
        let level_index = (initial_level.max(1) - 1).min(max_level.saturating_sub(1)) as usize;
        let initial_hp = stat_curves.max_hp.get(level_index).copied().unwrap_or(0) as u32;
        let initial_sp = stat_curves.max_sp.get(level_index).copied().unwrap_or(0) as u32;
        actors.push(Actor {
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
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{LcfError, Learning, StatCurves, parse_actors};

    /// Build a `skills` chunk (`0x3F`): a `[count]` header then per learning a
    /// 1-based index and its `level` (`0x01`) / `skill_id` (`0x02`) sub-chunks.
    fn learnings(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut out = varint(entries.len() as u32);
        for (i, &(level, skill_id)) in entries.iter().enumerate() {
            out.extend_from_slice(&element(
                i as u32 + 1,
                &[
                    subchunk(0x01, &varint(level)),
                    subchunk(0x02, &varint(skill_id)),
                ],
            ));
        }
        out
    }

    #[test]
    fn parses_skill_learning_list() {
        let hero = element(1, &[subchunk(0x3F, &learnings(&[(1, 5), (3, 8), (7, 12)]))]);
        let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
        let actor = &parse_actors(&ldb).unwrap()[0];
        assert_eq!(
            actor.skills,
            vec![
                Learning {
                    level: 1,
                    skill_id: 5
                },
                Learning {
                    level: 3,
                    skill_id: 8
                },
                Learning {
                    level: 7,
                    skill_id: 12
                },
            ]
        );
    }

    #[test]
    fn skill_learning_list_defaults_empty_and_drops_blank_rows() {
        // An entry with no skill_id chunk (skill 0) is a blank row RM2000 ignores.
        let hero = element(2, &[subchunk(0x3F, &learnings(&[(4, 0)]))]);
        let ldb = make_ldb(&[(0x0B, section(&[hero, element(3, &[])]))]);
        let actors = parse_actors(&ldb).unwrap();
        assert!(actors[0].skills.is_empty(), "blank learning row dropped");
        assert!(actors[1].skills.is_empty(), "omitted list defaults empty");
    }

    /// Build a `Parameters` chunk (`0x1F`) by concatenating the six Int16 stat
    /// curves (max HP, max SP, attack, defense, spirit, agility) as little-endian
    /// bytes. The caller passes six curves of equal length.
    fn parameters(curves: &[&[i16]]) -> Vec<u8> {
        curves
            .iter()
            .flat_map(|curve| curve.iter())
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    #[test]
    fn parses_actor_definition_fields() {
        // Name bytes 0x41 0x64 0xE9 0x6C are CP1250 "Adél"; title bytes
        // 0xC9 ... 0xE1 ... decode "Énekeslány" (Tiffany's real title).
        let params = parameters(&[&[10, 20], &[5, 8], &[0, 0], &[0, 0], &[0, 0], &[0, 0]]);
        let hero = element(
            1,
            &[
                subchunk(0x01, &[0x41, 0x64, 0xE9, 0x6C]),
                subchunk(
                    0x02,
                    &[0xC9, 0x6E, 0x65, 0x6B, 0x65, 0x73, 0x6C, 0xE1, 0x6E, 0x79],
                ),
                subchunk(0x07, &varint(2)),
                subchunk(0x1F, &params),
            ],
        );
        let ldb = make_ldb(&[(0x05, vec![9, 9]), (0x0B, section(&[hero]))]);
        let actors = parse_actors(&ldb).unwrap();
        assert_eq!(actors.len(), 1);
        let ron = &actors[0];
        assert_eq!(ron.id, 1);
        assert_eq!(ron.name, "Adél");
        assert_eq!(ron.title, "Énekeslány");
        assert_eq!(ron.initial_level, 2);
        assert_eq!(
            ron.max_level, 2,
            "max level falls back to the 2-level curve length"
        );
        assert_eq!(ron.initial_hp, 20, "maxhp curve at level 2");
        assert_eq!(ron.initial_sp, 8, "maxsp curve at level 2");
        assert_eq!(ron.stat_curves.max_hp, vec![10, 20]);
        assert_eq!(ron.stat_curves.max_sp, vec![5, 8]);
        assert_eq!(
            (ron.exp_base, ron.exp_inflation, ron.exp_correction),
            (30, 30, 0),
            "experience fields default when omitted"
        );
    }

    #[test]
    fn parses_stat_curves_and_experience() {
        let params = parameters(&[
            &[30, 40, 55],
            &[10, 14, 20],
            &[5, 7, 9],
            &[4, 6, 8],
            &[3, 5, 7],
            &[6, 9, 12],
        ]);
        let hero = element(
            1,
            &[
                subchunk(0x07, &varint(2)),
                subchunk(0x1F, &params),
                subchunk(0x29, &varint(31)),
                subchunk(0x2A, &varint(29)),
                subchunk(0x2B, &varint(40)),
            ],
        );
        let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
        let actor = &parse_actors(&ldb).unwrap()[0];
        assert_eq!(actor.max_level, 3, "curve length gives the max level");
        assert_eq!(
            actor.stat_curves,
            StatCurves {
                max_hp: vec![30, 40, 55],
                max_sp: vec![10, 14, 20],
                attack: vec![5, 7, 9],
                defense: vec![4, 6, 8],
                spirit: vec![3, 5, 7],
                agility: vec![6, 9, 12],
            }
        );
        assert_eq!(actor.initial_hp, 40, "maxhp curve at level 2");
        assert_eq!(actor.initial_sp, 14, "maxsp curve at level 2");
        assert_eq!(
            (actor.exp_base, actor.exp_inflation, actor.exp_correction),
            (31, 29, 40)
        );
    }

    #[test]
    fn actor_final_level_overrides_curve_length() {
        let params = parameters(&[&[10, 20], &[5, 8], &[0, 0], &[0, 0], &[0, 0], &[0, 0]]);
        let hero = element(1, &[subchunk(0x08, &varint(50)), subchunk(0x1F, &params)]);
        let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
        let actors = parse_actors(&ldb).unwrap();
        assert_eq!(actors[0].max_level, 50);
        assert_eq!(actors[0].initial_level, 1, "initial level defaults to 1");
        assert_eq!(actors[0].initial_hp, 10, "maxhp curve at default level 1");
        assert_eq!(
            actors[0].stat_curves.max_hp.len(),
            50,
            "curves are padded to max_level, repeating the last stored value"
        );
        assert_eq!(actors[0].stat_curves.max_hp[49], 20);
    }

    #[test]
    fn actor_defaults_when_fields_omitted() {
        let actor = element(3, &[]);
        let ldb = make_ldb(&[(0x0B, section(&[actor]))]);
        let actors = parse_actors(&ldb).unwrap();
        let a = &actors[0];
        assert_eq!(a.id, 3);
        assert!(a.name.is_empty());
        assert!(a.title.is_empty());
        assert_eq!(a.initial_level, 1);
        assert_eq!(a.max_level, 1);
        assert_eq!((a.initial_hp, a.initial_sp), (0, 0));
        assert_eq!(a.stat_curves.max_hp, vec![0]);
        assert_eq!((a.exp_base, a.exp_inflation, a.exp_correction), (30, 30, 0));
        assert_eq!(
            (a.weapon, a.shield, a.armor, a.helmet, a.accessory),
            (0, 0, 0, 0, 0),
            "empty equipment slots default to 0"
        );
        assert!(!a.two_weapons && !a.fix_equipment);
        assert_eq!(a.unarmed_animation, 0);
    }

    #[test]
    fn parses_initial_equipment_and_flags() {
        // initial_equipment (0x33) is five Int16 (LE) item ids: weapon 1,
        // shield 0, armor 64, helmet 83, accessory 0 — Ron's starting gear.
        let equipment = [1, 0, 0, 0, 64, 0, 83, 0, 0, 0];
        let hero = element(
            1,
            &[
                subchunk(0x15, &varint(1)),
                subchunk(0x16, &varint(1)),
                subchunk(0x33, &equipment),
                subchunk(0x38, &varint(9)),
            ],
        );
        let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
        let actor = &parse_actors(&ldb).unwrap()[0];
        assert_eq!(
            (
                actor.weapon,
                actor.shield,
                actor.armor,
                actor.helmet,
                actor.accessory
            ),
            (1, 0, 64, 83, 0)
        );
        assert!(actor.two_weapons, "dual wielding");
        assert!(actor.fix_equipment, "equipment locked");
        assert_eq!(actor.unarmed_animation, 9);
    }

    #[test]
    fn parses_face_graphic() {
        // face_name (0x0F) is a CP1250 string; face_index (0x10) a varint cell.
        let hero = element(1, &[subchunk(0x0F, b"Ron"), subchunk(0x10, &varint(3))]);
        let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
        let actor = &parse_actors(&ldb).unwrap()[0];
        assert_eq!(actor.face_name, "Ron");
        assert_eq!(actor.face_index, 3);
    }

    #[test]
    fn face_graphic_defaults_when_omitted() {
        let ldb = make_ldb(&[(0x0B, section(&[element(2, &[])]))]);
        let actor = &parse_actors(&ldb).unwrap()[0];
        assert!(actor.face_name.is_empty());
        assert_eq!(actor.face_index, 0);
    }

    #[test]
    fn parse_actors_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_actors(&ldb), Err(LcfError::MissingActors)));
    }

    #[test]
    fn actor_state_ranks_keep_each_byte_and_a_missing_tail() {
        let ldb = make_ldb(&[(
            0x0B,
            section(&[element(1, &[subchunk(0x48, &[0, 4, 2])]), element(2, &[])]),
        )]);
        let actors = parse_actors(&ldb).unwrap();
        assert_eq!(actors[0].state_ranks, [0, 4, 2]);
        assert!(actors[1].state_ranks.is_empty());
    }
}

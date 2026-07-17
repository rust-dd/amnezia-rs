//! Skill (spell/ability) definitions from the database (`ChunkData::skills`,
//! `0x0C`). Each skill stores its cost and battle numbers as scalar chunks, the
//! same shape enemies use, plus two `vector<bool>` chunks that mark which
//! elements it carries and which states it affects. Chunk ids follow liblcf
//! `ChunkSkill`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// A skill (spell/ability) definition: the fields a skill menu and the battle
/// system need. `sp_cost` is the SP spent to cast it, `power` the base effect
/// magnitude, and `hit` the base success rate (percent).
///
/// The battle fields describe how the skill resolves. `scope` picks its targets
/// (`0` one enemy, `1` all enemies, `2` the caster, `3` one ally, `4` all
/// allies) and `skill_type` its family (`0` normal — the only battle-relevant
/// kind — `1` teleport, `2` escape, `3` switch). `physical_rate`/`magical_rate`
/// (0–10) weight how much the caster's attack versus spirit feeds the damage
/// formula. `affect_hp`/`affect_sp` say which pool the effect changes and
/// `absorb` whether the caster drains what it deals. `attributes` lists the
/// 1-based element ids the damage is checked against and `affected_states` the
/// 1-based state ids it inflicts (RM2000's `state_effects`; this game never sets
/// `reverse_state_effect`, so these are always inflicted, never cured).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub sp_cost: u32,
    pub power: u32,
    pub hit: u32,
    pub skill_type: u32,
    pub scope: u32,
    pub physical_rate: u32,
    pub magical_rate: u32,
    pub affect_hp: bool,
    pub affect_sp: bool,
    pub absorb: bool,
    pub attributes: Vec<u32>,
    pub affected_states: Vec<u32>,
}

const SKILL_SECTION: u32 = 0x0C;
const SKILL_NAME: u32 = 0x01;
const SKILL_DESCRIPTION: u32 = 0x02;
const SKILL_TYPE: u32 = 0x08;
const SKILL_SP_COST: u32 = 0x0B;
const SKILL_SCOPE: u32 = 0x0C;
const SKILL_PHYSICAL_RATE: u32 = 0x15;
const SKILL_MAGICAL_RATE: u32 = 0x16;
const SKILL_POWER: u32 = 0x18;
const SKILL_HIT: u32 = 0x19;
const SKILL_AFFECT_HP: u32 = 0x1F;
const SKILL_AFFECT_SP: u32 = 0x20;
const SKILL_ABSORB_DAMAGE: u32 = 0x25;
const SKILL_STATE_EFFECTS: u32 = 0x2A;
const SKILL_ATTRIBUTE_EFFECTS: u32 = 0x2C;

// RM2000 omits `magical_rate` when it equals the editor default of 3.
const SKILL_DEFAULT_MAGICAL_RATE: u32 = 3;

/// Decode an LCF `vector<bool>` payload — one byte per element, the byte at
/// index `i` (0-based) flagging element id `i + 1` — into the ascending list of
/// 1-based ids whose byte is set. RM2000 stores a skill's affected states
/// (`state_effects`) and its elements (`attribute_effects`) this way.
fn decode_flag_ids(data: &[u8]) -> Vec<u32> {
    data.iter()
        .enumerate()
        .filter_map(|(i, &b)| (b != 0).then_some(i as u32 + 1))
        .collect()
}

/// Parse the skill table (`ChunkData::skills` = `0x0C`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkSkill`): name `0x01`, description `0x02`, type
/// `0x08`, sp_cost `0x0B`, scope `0x0C`, physical_rate `0x15`, magical_rate
/// `0x16`, power `0x18`, hit `0x19`, affect_hp `0x1F`, affect_sp `0x20`,
/// absorb_damage `0x25`, state_effects `0x2A`, attribute_effects `0x2C`. Scalar
/// fields are integer chunks; the two effect lists are `vector<bool>` chunks
/// (one byte per element). Omitted fields default to 0/false, except
/// `magical_rate` which defaults to the RM2000 editor value 3.
pub fn parse_skills(bytes: &[u8]) -> Result<Vec<Skill>, LcfError> {
    let section = find_section(bytes, SKILL_SECTION, LcfError::MissingSkills)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut skills = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut skill = Skill {
            id,
            name: String::new(),
            description: String::new(),
            sp_cost: 0,
            power: 0,
            hit: 0,
            skill_type: 0,
            scope: 0,
            physical_rate: 0,
            magical_rate: SKILL_DEFAULT_MAGICAL_RATE,
            affect_hp: false,
            affect_sp: false,
            absorb: false,
            attributes: Vec::new(),
            affected_states: Vec::new(),
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                SKILL_NAME => skill.name = decode_cp1250(sub_data),
                SKILL_DESCRIPTION => skill.description = decode_cp1250(sub_data),
                SKILL_TYPE => skill.skill_type = Reader::new(sub_data).varint()?,
                SKILL_SP_COST => skill.sp_cost = Reader::new(sub_data).varint()?,
                SKILL_SCOPE => skill.scope = Reader::new(sub_data).varint()?,
                SKILL_PHYSICAL_RATE => skill.physical_rate = Reader::new(sub_data).varint()?,
                SKILL_MAGICAL_RATE => skill.magical_rate = Reader::new(sub_data).varint()?,
                SKILL_POWER => skill.power = Reader::new(sub_data).varint()?,
                SKILL_HIT => skill.hit = Reader::new(sub_data).varint()?,
                SKILL_AFFECT_HP => skill.affect_hp = Reader::new(sub_data).varint()? != 0,
                SKILL_AFFECT_SP => skill.affect_sp = Reader::new(sub_data).varint()? != 0,
                SKILL_ABSORB_DAMAGE => skill.absorb = Reader::new(sub_data).varint()? != 0,
                SKILL_STATE_EFFECTS => skill.affected_states = decode_flag_ids(sub_data),
                SKILL_ATTRIBUTE_EFFECTS => skill.attributes = decode_flag_ids(sub_data),
                _ => {}
            }
        }
        skills.push(skill);
    }
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{LcfError, Skill, parse_skills};

    #[test]
    fn parses_skill_battle_fields() {
        // Name bytes are CP1250 "Tűzgolyó" (fireball): 0xFB = 'ű', 0xF3 = 'ó'.
        // Description bytes decode "Égeti" (0xC9 = 'É'). It hits one enemy (scope
        // 0), is a magical fire attack (attribute id 5), and inflicts Poison
        // (state id 3).
        let fireball = element(
            1,
            &[
                subchunk(0x01, &[0x54, 0xFB, 0x7A, 0x67, 0x6F, 0x6C, 0x79, 0xF3]),
                subchunk(0x02, &[0xC9, 0x67, 0x65, 0x74, 0x69]),
                subchunk(0x08, &varint(0)),
                subchunk(0x0B, &varint(8)),
                subchunk(0x0C, &varint(0)),
                subchunk(0x15, &varint(0)),
                subchunk(0x16, &varint(10)),
                subchunk(0x18, &varint(35)),
                subchunk(0x19, &varint(90)),
                subchunk(0x1F, &varint(1)),
                subchunk(0x2A, &[0, 0, 1]),
                subchunk(0x2C, &[0, 0, 0, 0, 1]),
            ],
        );
        let ldb = make_ldb(&[(0x0D, vec![1]), (0x0C, section(&[fireball]))]);
        let skills = parse_skills(&ldb).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(
            skills[0],
            Skill {
                id: 1,
                name: "Tűzgolyó".to_string(),
                description: "Égeti".to_string(),
                sp_cost: 8,
                power: 35,
                hit: 90,
                skill_type: 0,
                scope: 0,
                physical_rate: 0,
                magical_rate: 10,
                affect_hp: true,
                affect_sp: false,
                absorb: false,
                attributes: vec![5],
                affected_states: vec![3],
            }
        );
    }

    #[test]
    fn skill_fields_default_when_omitted() {
        let heal = element(2, &[subchunk(0x01, b"Heal"), subchunk(0x0B, &varint(4))]);
        let ldb = make_ldb(&[(0x0C, section(&[heal]))]);
        let skills = parse_skills(&ldb).unwrap();
        let s = &skills[0];
        assert_eq!(s.id, 2);
        assert_eq!(s.name, "Heal");
        assert_eq!(s.sp_cost, 4);
        assert_eq!((s.power, s.hit), (0, 0));
        assert_eq!((s.skill_type, s.scope, s.physical_rate), (0, 0, 0));
        assert_eq!(s.magical_rate, 3, "omitted magical_rate defaults to 3");
        assert!(!s.affect_hp && !s.affect_sp && !s.absorb);
        assert!(s.attributes.is_empty() && s.affected_states.is_empty());
        assert!(s.description.is_empty());
    }

    #[test]
    fn parses_scope_type_and_absorb() {
        // A self-cast (scope 2) switch-type skill (type 3) that drains SP.
        let drain = element(
            3,
            &[
                subchunk(0x08, &varint(3)),
                subchunk(0x0C, &varint(2)),
                subchunk(0x20, &varint(1)),
                subchunk(0x25, &varint(1)),
            ],
        );
        let ldb = make_ldb(&[(0x0C, section(&[drain]))]);
        let s = &parse_skills(&ldb).unwrap()[0];
        assert_eq!(s.skill_type, 3, "switch");
        assert_eq!(s.scope, 2, "self");
        assert!(s.affect_sp, "affects SP");
        assert!(s.absorb, "drains what it deals");
        assert!(!s.affect_hp);
    }

    #[test]
    fn effect_lists_flag_multiple_ids() {
        // state_effects [0,1,0,0,0,0,0,0,1] -> ids 2 and 9;
        // attribute_effects [1,0,1] -> ids 1 and 3.
        let multi = element(
            4,
            &[
                subchunk(0x2A, &[0, 1, 0, 0, 0, 0, 0, 0, 1]),
                subchunk(0x2C, &[1, 0, 1]),
            ],
        );
        let ldb = make_ldb(&[(0x0C, section(&[multi]))]);
        let s = &parse_skills(&ldb).unwrap()[0];
        assert_eq!(s.affected_states, vec![2, 9]);
        assert_eq!(s.attributes, vec![1, 3]);
    }

    #[test]
    fn parse_skills_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_skills(&ldb), Err(LcfError::MissingSkills)));
    }
}

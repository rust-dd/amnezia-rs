//! Skill (spell/ability) definitions from the database (`ChunkData::skills`,
//! `0x0C`). Each skill stores its cost and battle numbers as scalar chunks, the
//! same shape enemies use. Chunk ids follow liblcf `ChunkSkill`.

use super::find_section;
use crate::{decode_cp1250, LcfError, Reader};

/// A skill (spell/ability) definition: the fields a skill menu and the battle
/// system need. `sp_cost` is the SP spent to cast it, `power` the base effect
/// magnitude, and `hit` the base success rate (percent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub sp_cost: u32,
    pub power: u32,
    pub hit: u32,
}

const SKILL_SECTION: u32 = 0x0C;
const SKILL_NAME: u32 = 0x01;
const SKILL_DESCRIPTION: u32 = 0x02;
const SKILL_SP_COST: u32 = 0x0B;
const SKILL_POWER: u32 = 0x18;
const SKILL_HIT: u32 = 0x19;

/// Parse the skill table (`ChunkData::skills` = `0x0C`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkSkill`): name `0x01`, description `0x02`,
/// sp_cost `0x0B`, power `0x18`, hit `0x19`. Each numeric field is a scalar
/// integer chunk; omitted fields default to 0.
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
                SKILL_SP_COST => skill.sp_cost = Reader::new(sub_data).varint()?,
                SKILL_POWER => skill.power = Reader::new(sub_data).varint()?,
                SKILL_HIT => skill.hit = Reader::new(sub_data).varint()?,
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
    use crate::{parse_skills, LcfError, Skill};

    #[test]
    fn parses_skill_battle_fields() {
        // Name bytes are CP1250 "Tűzgolyó" (fireball): 0xFB = 'ű', 0xF3 = 'ó'.
        // Description bytes decode "Égeti" (0xC9 = 'É').
        let fireball = element(
            1,
            &[
                subchunk(0x01, &[0x54, 0xFB, 0x7A, 0x67, 0x6F, 0x6C, 0x79, 0xF3]),
                subchunk(0x02, &[0xC9, 0x67, 0x65, 0x74, 0x69]),
                subchunk(0x0B, &varint(8)),
                subchunk(0x18, &varint(35)),
                subchunk(0x19, &varint(90)),
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
            }
        );
    }

    #[test]
    fn skill_fields_default_to_zero_when_omitted() {
        let heal = element(2, &[subchunk(0x01, b"Heal"), subchunk(0x0B, &varint(4))]);
        let ldb = make_ldb(&[(0x0C, section(&[heal]))]);
        let skills = parse_skills(&ldb).unwrap();
        let s = &skills[0];
        assert_eq!(s.id, 2);
        assert_eq!(s.name, "Heal");
        assert_eq!(s.sp_cost, 4);
        assert_eq!((s.power, s.hit), (0, 0));
        assert!(s.description.is_empty());
    }

    #[test]
    fn parse_skills_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_skills(&ldb), Err(LcfError::MissingSkills)));
    }
}

//! Monster (enemy) definitions from the database (`ChunkData::enemies`,
//! `0x0E`). RM2000 stores each battle stat as a single scalar chunk, unlike the
//! per-level curves actors use. Chunk ids follow liblcf `ChunkEnemy`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// A monster (enemy) definition: the battle-relevant scalar stats plus the
/// experience and gold it yields when defeated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster {
    pub id: u32,
    pub name: String,
    pub max_hp: u32,
    pub max_sp: u32,
    pub attack: u32,
    pub defense: u32,
    pub spirit: u32,
    pub agility: u32,
    pub exp: u32,
    pub gold: u32,
}

const MONSTER_SECTION: u32 = 0x0E;
const MONSTER_NAME: u32 = 0x01;
const MONSTER_MAX_HP: u32 = 0x04;
const MONSTER_MAX_SP: u32 = 0x05;
const MONSTER_ATTACK: u32 = 0x06;
const MONSTER_DEFENSE: u32 = 0x07;
const MONSTER_SPIRIT: u32 = 0x08;
const MONSTER_AGILITY: u32 = 0x09;
const MONSTER_EXP: u32 = 0x0B;
const MONSTER_GOLD: u32 = 0x0C;

/// Parse the enemy table (`ChunkData::enemies` = `0x0E`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkEnemy`): name `0x01`, max_hp `0x04`, max_sp
/// `0x05`, attack `0x06`, defense `0x07`, spirit `0x08`, agility `0x09`, exp
/// `0x0B`, gold `0x0C`. Each stat is a scalar integer chunk; omitted fields
/// default to 0.
pub fn parse_monsters(bytes: &[u8]) -> Result<Vec<Monster>, LcfError> {
    let section = find_section(bytes, MONSTER_SECTION, LcfError::MissingMonsters)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut monsters = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut monster = Monster {
            id,
            name: String::new(),
            max_hp: 0,
            max_sp: 0,
            attack: 0,
            defense: 0,
            spirit: 0,
            agility: 0,
            exp: 0,
            gold: 0,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                MONSTER_NAME => monster.name = decode_cp1250(sub_data),
                MONSTER_MAX_HP => monster.max_hp = Reader::new(sub_data).varint()?,
                MONSTER_MAX_SP => monster.max_sp = Reader::new(sub_data).varint()?,
                MONSTER_ATTACK => monster.attack = Reader::new(sub_data).varint()?,
                MONSTER_DEFENSE => monster.defense = Reader::new(sub_data).varint()?,
                MONSTER_SPIRIT => monster.spirit = Reader::new(sub_data).varint()?,
                MONSTER_AGILITY => monster.agility = Reader::new(sub_data).varint()?,
                MONSTER_EXP => monster.exp = Reader::new(sub_data).varint()?,
                MONSTER_GOLD => monster.gold = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        monsters.push(monster);
    }
    Ok(monsters)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{LcfError, Monster, parse_monsters};

    #[test]
    fn parses_monster_battle_stats() {
        // Name bytes are CP1250 "Sárkány" (dragon): 0xE1 = 'á'.
        let dragon = element(
            1,
            &[
                subchunk(0x01, &[0x53, 0xE1, 0x72, 0x6B, 0xE1, 0x6E, 0x79]),
                subchunk(0x04, &varint(999)),
                subchunk(0x05, &varint(120)),
                subchunk(0x06, &varint(180)),
                subchunk(0x07, &varint(90)),
                subchunk(0x08, &varint(70)),
                subchunk(0x09, &varint(45)),
                subchunk(0x0B, &varint(1500)),
                subchunk(0x0C, &varint(800)),
            ],
        );
        let ldb = make_ldb(&[(0x0D, vec![1]), (0x0E, section(&[dragon]))]);
        let monsters = parse_monsters(&ldb).unwrap();
        assert_eq!(monsters.len(), 1);
        assert_eq!(
            monsters[0],
            Monster {
                id: 1,
                name: "Sárkány".to_string(),
                max_hp: 999,
                max_sp: 120,
                attack: 180,
                defense: 90,
                spirit: 70,
                agility: 45,
                exp: 1500,
                gold: 800,
            }
        );
    }

    #[test]
    fn monster_fields_default_to_zero_when_omitted() {
        let slime = element(2, &[subchunk(0x01, b"Slime"), subchunk(0x04, &varint(30))]);
        let ldb = make_ldb(&[(0x0E, section(&[slime]))]);
        let monsters = parse_monsters(&ldb).unwrap();
        let m = &monsters[0];
        assert_eq!(m.id, 2);
        assert_eq!(m.name, "Slime");
        assert_eq!(m.max_hp, 30);
        assert_eq!((m.attack, m.defense, m.spirit, m.agility), (0, 0, 0, 0));
        assert_eq!((m.exp, m.gold), (0, 0));
    }

    #[test]
    fn parse_monsters_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(
            parse_monsters(&ldb),
            Err(LcfError::MissingMonsters)
        ));
    }
}

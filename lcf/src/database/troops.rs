//! Troop (enemy party) definitions from the database (`ChunkData::troops`,
//! `0x0F`), including positioned enemies and conditional battle-event pages.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

mod pages;
pub use pages::{TroopPage, TroopPageCondition};

/// One member of a troop: which enemy fights (`enemy_id`, a monster's 1-based
/// id) and where it stands on the battle backdrop (`x`,`y` in screen pixels).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TroopMember {
    pub enemy_id: u32,
    pub x: u32,
    pub y: u32,
}

/// A troop's identity, positioned enemies and conditional battle-event pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Troop {
    pub id: u32,
    pub name: String,
    pub members: Vec<TroopMember>,
    pub pages: Vec<TroopPage>,
}

const TROOP_SECTION: u32 = 0x0F;
const TROOP_NAME: u32 = 0x01;
const TROOP_MEMBERS: u32 = 0x02;
const TROOP_PAGES: u32 = 0x0B;
const MEMBER_ENEMY_ID: u32 = 0x01;
const MEMBER_X: u32 = 0x02;
const MEMBER_Y: u32 = 0x03;
const MEMBER_DEFAULT_ENEMY_ID: u32 = 1;

/// Parse a troop's `members` array (`0x02`): a `[count]` header then, per
/// member, a 1-based index id and a chunk stream (enemy_id `0x01`, x `0x02`, y
/// `0x03`), matching the nested struct-list shape used elsewhere in the LCF.
fn parse_members(data: &[u8]) -> Result<Vec<TroopMember>, LcfError> {
    let mut reader = Reader::new(data);
    let count = reader.varint()?;
    let mut members = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _member_id = reader.varint()?;
        let mut member = TroopMember {
            enemy_id: MEMBER_DEFAULT_ENEMY_ID,
            x: 0,
            y: 0,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                MEMBER_ENEMY_ID => member.enemy_id = Reader::new(sub_data).varint()?,
                MEMBER_X => member.x = Reader::new(sub_data).varint()?,
                MEMBER_Y => member.y = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        members.push(member);
    }
    Ok(members)
}

/// Parse the troop table (`ChunkData::troops` = `0x0F`) out of an LDB byte
/// slice, preserving the members and battle-event pages in source order.
pub fn parse_troops(bytes: &[u8]) -> Result<Vec<Troop>, LcfError> {
    let section = find_section(bytes, TROOP_SECTION, LcfError::MissingTroops)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut troops = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut troop = Troop {
            id,
            name: String::new(),
            members: Vec::new(),
            pages: Vec::new(),
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                TROOP_NAME => troop.name = decode_cp1250(sub_data),
                TROOP_MEMBERS => troop.members = parse_members(sub_data)?,
                TROOP_PAGES => troop.pages = pages::parse_pages(sub_data)?,
                _ => {}
            }
        }
        troops.push(troop);
    }
    Ok(troops)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{LcfError, TroopMember, parse_troops};

    #[test]
    fn preserves_battle_pages_conditions_and_commands() {
        let mut condition = subchunk(0x01, &[0x29]);
        condition.extend(subchunk(0x02, &varint(545)));
        condition.extend(subchunk(0x06, &varint(2)));
        condition.extend(subchunk(0x07, &varint(1)));
        condition.extend(subchunk(0x0A, &varint(3)));
        condition.extend(subchunk(0x0C, &varint(0)));
        condition.push(0);
        let command = [
            varint(10210),
            varint(1),
            varint(0),
            varint(4),
            varint(0),
            varint(617),
            varint(617),
            varint(0),
        ]
        .concat();
        let page = element(1, &[subchunk(2, &condition), subchunk(0x0C, &command)]);
        let troop = element(1, &[subchunk(0x0B, &section(&[page, element(2, &[])]))]);
        let troops = parse_troops(&make_ldb(&[(0x0F, section(&[troop]))])).unwrap();
        assert_eq!(troops[0].pages.len(), 2);
        let page = &troops[0].pages[0];
        assert_eq!(page.condition.flags, 0x29);
        assert_eq!(page.condition.switch_a_id, 545);
        assert_eq!((page.condition.turn_a, page.condition.turn_b), (2, 1));
        assert_eq!(
            (page.condition.enemy_index, page.condition.enemy_hp_max),
            (3, 0)
        );
        assert_eq!(page.commands[0].code, 10210);
        assert_eq!(page.commands[0].indent, 1);
        assert_eq!(page.commands[0].params, [0, 617, 617, 0]);
        let defaults = &troops[0].pages[1].condition;
        assert_eq!(defaults.flags, 0);
        assert_eq!(
            (
                defaults.switch_a_id,
                defaults.switch_b_id,
                defaults.variable_id,
                defaults.actor_id
            ),
            (1, 1, 1, 1)
        );
        assert_eq!((defaults.enemy_hp_min, defaults.enemy_hp_max), (0, 100));
        assert_eq!((defaults.actor_hp_min, defaults.actor_hp_max), (0, 100));
        assert_eq!((defaults.fatigue_min, defaults.fatigue_max), (0, 100));
    }

    #[test]
    fn parses_troop_with_members() {
        let m1 = element(
            1,
            &[
                subchunk(0x01, &varint(3)),
                subchunk(0x02, &varint(80)),
                subchunk(0x03, &varint(120)),
            ],
        );
        let m2 = element(
            2,
            &[subchunk(0x01, &varint(5)), subchunk(0x02, &varint(160))],
        );
        let mut members = varint(2);
        members.extend_from_slice(&m1);
        members.extend_from_slice(&m2);
        // Name bytes are CP1250 "Őrök" (guards): 0xD5 = 'Ő', 0xF6 = 'ö'.
        let troop = element(
            1,
            &[
                subchunk(0x01, &[0xD5, 0x72, 0xF6, 0x6B]),
                subchunk(0x02, &members),
            ],
        );
        let ldb = make_ldb(&[(0x0E, section(&[])), (0x0F, section(&[troop]))]);
        let troops = parse_troops(&ldb).unwrap();
        assert_eq!(troops.len(), 1);
        assert_eq!(troops[0].id, 1);
        assert_eq!(troops[0].name, "Őrök");
        assert_eq!(
            troops[0].members,
            vec![
                TroopMember {
                    enemy_id: 3,
                    x: 80,
                    y: 120
                },
                TroopMember {
                    enemy_id: 5,
                    x: 160,
                    y: 0
                },
            ]
        );
    }

    #[test]
    fn troop_member_enemy_id_defaults_to_one() {
        let m1 = element(1, &[subchunk(0x02, &varint(50))]);
        let mut members = varint(1);
        members.extend_from_slice(&m1);
        let troop = element(1, &[subchunk(0x02, &members)]);
        let ldb = make_ldb(&[(0x0F, section(&[troop]))]);
        let troops = parse_troops(&ldb).unwrap();
        assert_eq!(
            troops[0].members,
            vec![TroopMember {
                enemy_id: 1,
                x: 50,
                y: 0
            }]
        );
        assert!(troops[0].name.is_empty());
    }

    #[test]
    fn parse_troops_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_troops(&ldb), Err(LcfError::MissingTroops)));
    }
}

//! Troop (enemy party) definitions from the database (`ChunkData::troops`,
//! `0x0F`). A troop is the fixed enemy party of a battle: a name and a list of
//! members, each placing one monster on the backdrop. Chunk ids follow liblcf
//! `ChunkTroop` / `ChunkTroopMember`.

use super::find_section;
use crate::{decode_cp1250, LcfError, Reader};

/// One member of a troop: which enemy fights (`enemy_id`, a monster's 1-based
/// id) and where it stands on the battle backdrop (`x`,`y` in screen pixels).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TroopMember {
    pub enemy_id: u32,
    pub x: u32,
    pub y: u32,
}

/// A troop (enemy party) definition: its 1-based id, name, and the enemies it
/// fields with their battle positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Troop {
    pub id: u32,
    pub name: String,
    pub members: Vec<TroopMember>,
}

const TROOP_SECTION: u32 = 0x0F;
const TROOP_NAME: u32 = 0x01;
const TROOP_MEMBERS: u32 = 0x02;
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
        let mut member = TroopMember { enemy_id: MEMBER_DEFAULT_ENEMY_ID, x: 0, y: 0 };
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
/// slice. Chunk ids (liblcf `ChunkTroop`): name `0x01`, members `0x02`.
pub fn parse_troops(bytes: &[u8]) -> Result<Vec<Troop>, LcfError> {
    let section = find_section(bytes, TROOP_SECTION, LcfError::MissingTroops)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut troops = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut troop = Troop { id, name: String::new(), members: Vec::new() };
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
    use crate::{parse_troops, LcfError, TroopMember};

    #[test]
    fn parses_troop_with_members() {
        let m1 = element(
            1,
            &[subchunk(0x01, &varint(3)), subchunk(0x02, &varint(80)), subchunk(0x03, &varint(120))],
        );
        let m2 = element(2, &[subchunk(0x01, &varint(5)), subchunk(0x02, &varint(160))]);
        let mut members = varint(2);
        members.extend_from_slice(&m1);
        members.extend_from_slice(&m2);
        // Name bytes are CP1250 "Őrök" (guards): 0xD5 = 'Ő', 0xF6 = 'ö'.
        let troop = element(1, &[subchunk(0x01, &[0xD5, 0x72, 0xF6, 0x6B]), subchunk(0x02, &members)]);
        let ldb = make_ldb(&[(0x0E, section(&[])), (0x0F, section(&[troop]))]);
        let troops = parse_troops(&ldb).unwrap();
        assert_eq!(troops.len(), 1);
        assert_eq!(troops[0].id, 1);
        assert_eq!(troops[0].name, "Őrök");
        assert_eq!(
            troops[0].members,
            vec![
                TroopMember { enemy_id: 3, x: 80, y: 120 },
                TroopMember { enemy_id: 5, x: 160, y: 0 },
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
        assert_eq!(troops[0].members, vec![TroopMember { enemy_id: 1, x: 50, y: 0 }]);
        assert!(troops[0].name.is_empty());
    }

    #[test]
    fn parse_troops_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_troops(&ldb), Err(LcfError::MissingTroops)));
    }
}

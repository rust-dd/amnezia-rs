//! Database (`RPG_RT.ldb`) parsing: the chipset, actor, skill, item, monster,
//! troop, attribute (element), and state (status condition) definition tables.
//! Every section shares the LDB struct-list shape — a `[count]` header then per
//! entry a 1-based id followed by a chunk stream (terminated by id 0). Chunk ids
//! follow EasyRPG/liblcf `src/generated/lcf/ldb/chunks.h`.

use crate::LcfError;
use crate::Reader;

mod actors;
mod attributes;
mod common_events;
mod items;
mod monsters;
mod skills;
mod states;
mod troops;

pub use actors::{Actor, StatCurves, parse_actors};
pub use attributes::{Attribute, parse_attributes};
pub use common_events::{CommonEvent, parse_common_events};
pub use items::{Item, parse_items};
pub use monsters::{Monster, parse_monsters};
pub use skills::{Skill, parse_skills};
pub use states::{State, parse_states};
pub use troops::{Troop, TroopMember, parse_troops};

/// Locate one top-level LDB section (`ChunkData`) by id, returning its raw
/// bytes. Verifies the `LcfDataBase` signature and skips every other section.
fn find_section(bytes: &[u8], section_id: u32, missing: LcfError) -> Result<&[u8], LcfError> {
    let mut reader = Reader::new(bytes);
    let signature_len = reader.byte()? as usize;
    let signature = reader.take(signature_len)?;
    if signature != b"LcfDataBase" {
        return Err(LcfError::BadSignature {
            expected: "LcfDataBase",
        });
    }
    while !reader.is_empty() {
        let id = reader.varint()?;
        if id == 0 {
            break;
        }
        let size = reader.varint()? as usize;
        let data = reader.take(size)?;
        if id == section_id {
            return Ok(data);
        }
    }
    Err(missing)
}

/// A chipset entry from the database: its 1-based id, the base name of its
/// `ChipSet/<name>` graphic, and its passability arrays (per tile-type
/// bitfields; lower is 162 bytes, upper is 144, padded with `0x0F`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chipset {
    pub id: u32,
    pub name: String,
    pub passages_down: Vec<u8>,
    pub passages_up: Vec<u8>,
}

const CHIPSET_SECTION: u32 = 0x14;
const CHIPSET_NAME: u32 = 0x02;
const CHIPSET_PASSAGES_DOWN: u32 = 0x04;
const CHIPSET_PASSAGES_UP: u32 = 0x05;
const PASSAGES_DOWN_LEN: usize = 162;
const PASSAGES_UP_LEN: usize = 144;
const PASSAGE_DEFAULT: u8 = 0x0F;

/// Parse the chipset graphic names out of an LDB (`RPG_RT.ldb`) byte slice.
/// Only the chipset section (`ChunkData::chipsets` = `0x14`) is read.
pub fn parse_chipsets(bytes: &[u8]) -> Result<Vec<Chipset>, LcfError> {
    let section = find_section(bytes, CHIPSET_SECTION, LcfError::MissingChipsets)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut chipsets = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut name = String::new();
        let mut passages_down: Vec<u8> = Vec::new();
        let mut passages_up: Vec<u8> = Vec::new();
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                CHIPSET_NAME => name = String::from_utf8_lossy(sub_data).into_owned(),
                CHIPSET_PASSAGES_DOWN => passages_down = sub_data.to_vec(),
                CHIPSET_PASSAGES_UP => passages_up = sub_data.to_vec(),
                _ => {}
            }
        }
        passages_down.resize(PASSAGES_DOWN_LEN, PASSAGE_DEFAULT);
        passages_up.resize(PASSAGES_UP_LEN, PASSAGE_DEFAULT);
        chipsets.push(Chipset {
            id,
            name,
            passages_down,
            passages_up,
        });
    }
    Ok(chipsets)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk};
    use crate::{LcfError, parse_chipsets};

    #[test]
    fn parses_chipset_graphic_names() {
        let element1 = element(1, &[subchunk(0x01, b"World"), subchunk(0x02, b"basis")]);
        let element2 = element(2, &[subchunk(0x02, b"outline")]);
        let ldb = make_ldb(&[
            (0x0B, vec![1, 2, 3]),
            (0x14, section(&[element1, element2])),
        ]);
        let chipsets = parse_chipsets(&ldb).unwrap();
        assert_eq!(chipsets.len(), 2);
        assert_eq!(chipsets[0].id, 1);
        assert_eq!(chipsets[0].name, "basis");
        assert_eq!(chipsets[1].name, "outline");
        assert_eq!(chipsets[0].passages_down.len(), 162);
        assert_eq!(chipsets[0].passages_up.len(), 144);
        assert!(chipsets[0].passages_down.iter().all(|&b| b == 0x0F));
    }

    #[test]
    fn chipset_name_defaults_to_empty_when_omitted() {
        let element = element(5, &[]);
        let ldb = make_ldb(&[(0x14, section(&[element]))]);
        let chipsets = parse_chipsets(&ldb).unwrap();
        assert_eq!(chipsets[0].id, 5);
        assert!(chipsets[0].name.is_empty());
        assert_eq!(chipsets[0].passages_down.len(), 162);
    }

    #[test]
    fn captures_and_pads_passability() {
        let element = element(
            1,
            &[
                subchunk(0x02, b"x"),
                subchunk(0x04, &[0x00, 0x0F]),
                subchunk(0x05, &[0x1F]),
            ],
        );
        let ldb = make_ldb(&[(0x14, section(&[element]))]);
        let chipset = &parse_chipsets(&ldb).unwrap()[0];
        assert_eq!(chipset.passages_down.len(), 162);
        assert_eq!(chipset.passages_up.len(), 144);
        assert_eq!(&chipset.passages_down[..2], &[0x00, 0x0F]);
        assert_eq!(chipset.passages_down[2], 0x0F);
        assert_eq!(chipset.passages_up[0], 0x1F);
    }

    #[test]
    fn parse_chipsets_rejects_bad_signature() {
        let ldb = {
            let mut out = vec![10u8];
            out.extend_from_slice(b"LcfMapUnit");
            out.push(0);
            out
        };
        assert!(matches!(
            parse_chipsets(&ldb),
            Err(LcfError::BadSignature {
                expected: "LcfDataBase"
            })
        ));
    }

    #[test]
    fn parse_chipsets_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x0B, vec![1, 2, 3])]);
        assert!(matches!(
            parse_chipsets(&ldb),
            Err(LcfError::MissingChipsets)
        ));
    }
}

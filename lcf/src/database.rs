//! Database (`RPG_RT.ldb`) parsing: the chipset, actor, skill, item, monster,
//! and troop definition tables. Every section shares the LDB struct-list shape — a
//! `[count]` header then per entry a 1-based id followed by a chunk stream
//! (terminated by id 0). Chunk ids follow EasyRPG/liblcf
//! `src/generated/lcf/ldb/chunks.h`.

use crate::LcfError;
use crate::{Reader, decode_cp1250};

mod common_events;
mod monsters;
mod skills;
mod troops;

pub use common_events::{CommonEvent, parse_common_events};
pub use monsters::{Monster, parse_monsters};
pub use skills::{Skill, parse_skills};
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

/// An actor (playable character) definition: the fields the status, equip, and
/// message screens need. `name` expands the `\N[k]` message control code;
/// `initial_hp`/`initial_sp` are read off the level-parameter curve at
/// `initial_level`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: u32,
    pub name: String,
    pub title: String,
    pub initial_level: u32,
    pub max_level: u32,
    pub initial_hp: u32,
    pub initial_sp: u32,
}

const ACTOR_SECTION: u32 = 0x0B;
const ACTOR_NAME: u32 = 0x01;
const ACTOR_TITLE: u32 = 0x02;
const ACTOR_INITIAL_LEVEL: u32 = 0x07;
const ACTOR_FINAL_LEVEL: u32 = 0x08;
const ACTOR_PARAMETERS: u32 = 0x1F;
const ACTOR_DEFAULT_LEVEL: u32 = 1;
const PARAMETER_STATS: usize = 6;

/// Read the `initial_level`-th value (1-based) of an int16 curve stored at
/// `stat_offset` bytes into the `Parameters` blob, clamped to non-negative.
fn curve_value(parameters: &[u8], stat_offset: usize, level: u32, curve_len: u32) -> u32 {
    if curve_len == 0 {
        return 0;
    }
    let index = (level.max(1) - 1).min(curve_len - 1) as usize;
    let byte = stat_offset + index * 2;
    match parameters.get(byte..byte + 2) {
        Some(pair) => i16::from_le_bytes([pair[0], pair[1]]).max(0) as u32,
        None => 0,
    }
}

/// Split the actor `Parameters` chunk (`0x1F`) into `(initial_hp, initial_sp,
/// levels)`. The blob is six equal int16 arrays — maxhp, maxsp, attack,
/// defense, spirit, agility (liblcf `RawStruct<rpg::Parameters>::ReadLcf`) — so
/// each stat spans `len / 6` bytes and the max-hp/max-sp curves come first.
fn actor_stats(parameters: &[u8], initial_level: u32) -> (u32, u32, u32) {
    let stat_len = parameters.len() / PARAMETER_STATS;
    let levels = (stat_len / 2) as u32;
    let hp = curve_value(parameters, 0, initial_level, levels);
    let sp = curve_value(parameters, stat_len, initial_level, levels);
    (hp, sp, levels)
}

/// Parse the actor table (`ChunkData::actors` = `0x0B`) out of an LDB byte
/// slice. Chunk ids (liblcf `ChunkActor`): name `0x01`, title `0x02`,
/// initial_level `0x07`, final_level `0x08`, parameters `0x1F`.
pub fn parse_actors(bytes: &[u8]) -> Result<Vec<Actor>, LcfError> {
    let section = find_section(bytes, ACTOR_SECTION, LcfError::MissingActors)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut actors = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut name = String::new();
        let mut title = String::new();
        let mut initial_level = ACTOR_DEFAULT_LEVEL;
        let mut final_level: Option<u32> = None;
        let mut parameters: &[u8] = &[];
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
                ACTOR_INITIAL_LEVEL => initial_level = Reader::new(sub_data).varint()?,
                ACTOR_FINAL_LEVEL => final_level = Some(Reader::new(sub_data).varint()?),
                ACTOR_PARAMETERS => parameters = sub_data,
                _ => {}
            }
        }
        let (initial_hp, initial_sp, curve_levels) = actor_stats(parameters, initial_level);
        // RM2000 omits `final_level` when it equals the editor default, which is
        // exactly the length of the parameter curve, so fall back to that.
        let max_level = final_level
            .filter(|&l| l > 0)
            .unwrap_or(curve_levels.max(1));
        actors.push(Actor {
            id,
            name,
            title,
            initial_level,
            max_level,
            initial_hp,
            initial_sp,
        });
    }
    Ok(actors)
}

/// An item definition: the fields a shop and item menu need. `item_type` is the
/// raw RM2000 category index (0 normal, 1 weapon, 2 shield, 3 armor, 4 helmet,
/// 5 accessory, 6 medicine, 7 book, 8 material, 9 special, 10 switch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub item_type: u32,
    pub price: u32,
}

const ITEM_SECTION: u32 = 0x0D;
const ITEM_NAME: u32 = 0x01;
const ITEM_DESCRIPTION: u32 = 0x02;
const ITEM_TYPE: u32 = 0x03;
const ITEM_PRICE: u32 = 0x05;

/// Parse the item table (`ChunkData::items` = `0x0D`) out of an LDB byte slice.
/// Chunk ids (liblcf `ChunkItem`): name `0x01`, description `0x02`, type
/// `0x03`, price `0x05`.
pub fn parse_items(bytes: &[u8]) -> Result<Vec<Item>, LcfError> {
    let section = find_section(bytes, ITEM_SECTION, LcfError::MissingItems)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut item = Item {
            id,
            name: String::new(),
            description: String::new(),
            item_type: 0,
            price: 0,
        };
        loop {
            let sub_id = reader.varint()?;
            if sub_id == 0 {
                break;
            }
            let sub_size = reader.varint()? as usize;
            let sub_data = reader.take(sub_size)?;
            match sub_id {
                ITEM_NAME => item.name = decode_cp1250(sub_data),
                ITEM_DESCRIPTION => item.description = decode_cp1250(sub_data),
                ITEM_TYPE => item.item_type = Reader::new(sub_data).varint()?,
                ITEM_PRICE => item.price = Reader::new(sub_data).varint()?,
                _ => {}
            }
        }
        items.push(item);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use crate::test_util::{element, make_ldb, section, subchunk, varint};
    use crate::{Item, LcfError, parse_actors, parse_chipsets, parse_items};

    /// Build a `Parameters` chunk (`0x1F`) from six int16 stat curves, laid out
    /// contiguously as maxhp, maxsp, attack, defense, spirit, agility.
    fn parameters(curves: &[[i16; 2]; 6]) -> Vec<u8> {
        curves
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

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

    #[test]
    fn parses_actor_definition_fields() {
        // Name bytes 0x41 0x64 0xE9 0x6C are CP1250 "Adél"; title bytes
        // 0xC9 ... 0xE1 ... decode "Énekeslány" (Tiffany's real title).
        let params = parameters(&[[10, 20], [5, 8], [0, 0], [0, 0], [0, 0], [0, 0]]);
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
    }

    #[test]
    fn actor_final_level_overrides_curve_length() {
        let params = parameters(&[[10, 20], [5, 8], [0, 0], [0, 0], [0, 0], [0, 0]]);
        let hero = element(1, &[subchunk(0x08, &varint(50)), subchunk(0x1F, &params)]);
        let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
        let actors = parse_actors(&ldb).unwrap();
        assert_eq!(actors[0].max_level, 50);
        assert_eq!(actors[0].initial_level, 1, "initial level defaults to 1");
        assert_eq!(actors[0].initial_hp, 10, "maxhp curve at default level 1");
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
    }

    #[test]
    fn parse_actors_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_actors(&ldb), Err(LcfError::MissingActors)));
    }

    #[test]
    fn parses_item_definitions() {
        // Description bytes decode CP1250 "Éles penge" (0xC9 = 'É').
        let desc = [0xC9, 0x6C, 0x65, 0x73, 0x20, 0x70, 0x65, 0x6E, 0x67, 0x65];
        let sword = element(
            1,
            &[
                subchunk(0x01, b"Ton-Kard"),
                subchunk(0x02, &desc),
                subchunk(0x03, &varint(1)),
                subchunk(0x05, &varint(1200)),
            ],
        );
        let potion = element(2, &[subchunk(0x01, b"Ital"), subchunk(0x03, &varint(6))]);
        let ldb = make_ldb(&[(0x0C, vec![7]), (0x0D, section(&[sword, potion]))]);
        let items = parse_items(&ldb).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0],
            Item {
                id: 1,
                name: "Ton-Kard".to_string(),
                description: "Éles penge".to_string(),
                item_type: 1,
                price: 1200,
            }
        );
        assert_eq!(items[1].name, "Ital");
        assert_eq!(items[1].item_type, 6, "medicine");
        assert_eq!(items[1].price, 0, "omitted price defaults to 0");
        assert!(items[1].description.is_empty());
    }

    #[test]
    fn parse_items_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_items(&ldb), Err(LcfError::MissingItems)));
    }
}

//! Item definitions (`ChunkData::items`, `0x0D`), following liblcf `ChunkItem`.

use super::find_section;
use crate::{LcfError, Reader, decode_cp1250};

/// Item categories: 0 normal, 1 weapon, 2 shield, 3 armor, 4 helmet, 5 accessory,
/// 6 medicine, 7 book, 8 material, 9 special, 10 switch. `scope`: 0 ally, 1 party.
/// Recovery combines fixed amounts with `_rate` percentages of maximum HP/SP;
/// `ko_only` restricts recovery to fallen actors; `uses = 0` means unlimited.
/// Equipment stats are bonuses; hit/crit are percentages.
/// State/attribute sets contain ascending 1-based IDs: weapon effects, armor
/// protections, or consumable cures, depending on the item type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub prevent_critical: bool,
    pub raise_evasion: bool,
    pub half_sp_cost: bool,
    pub actor_set: Vec<bool>,
    pub state_chance: u32,
    pub id: u32,
    pub name: String,
    pub description: String,
    pub item_type: u32,
    pub price: u32,
    pub uses: u32,
    pub recover_hp: u32,
    pub recover_hp_rate: u32,
    pub recover_sp: u32,
    pub recover_sp_rate: u32,
    pub scope: u32,
    pub only_field: bool,
    pub ko_only: bool,
    pub atk: u32,
    pub def: u32,
    pub spi: u32,
    pub agi: u32,
    pub two_handed: bool,
    pub hit: u32,
    pub crit: u32,
    pub weapon_animation: u32,
    pub state_set: Vec<u32>,
    pub attribute_set: Vec<u32>,
}

const ITEM_SECTION: u32 = 0x0D;
const ITEM_NAME: u32 = 0x01;
const ITEM_DESCRIPTION: u32 = 0x02;
const ITEM_TYPE: u32 = 0x03;
const ITEM_PRICE: u32 = 0x05;
const ITEM_USES: u32 = 0x06;
const ITEM_ATK: u32 = 0x0B;
const ITEM_DEF: u32 = 0x0C;
const ITEM_SPI: u32 = 0x0D;
const ITEM_AGI: u32 = 0x0E;
const ITEM_TWO_HANDED: u32 = 0x0F;
const ITEM_HIT: u32 = 0x11;
const ITEM_CRITICAL_HIT: u32 = 0x12;
const ITEM_ANIMATION_ID: u32 = 0x14;
const ITEM_ENTIRE_PARTY: u32 = 0x1F;
const ITEM_RECOVER_HP_RATE: u32 = 0x20;
const ITEM_RECOVER_HP: u32 = 0x21;
const ITEM_RECOVER_SP_RATE: u32 = 0x22;
const ITEM_RECOVER_SP: u32 = 0x23;
const ITEM_OCCASION_FIELD: u32 = 0x25;
const ITEM_KO_ONLY: u32 = 0x26;
const ITEM_STATE_SET: u32 = 0x40;
const ITEM_ATTRIBUTE_SET: u32 = 0x42;

/// Decode an LCF `vector<bool>` payload — one byte per element, the byte at
/// index `i` (0-based) flagging id `i + 1` — into the ascending list of 1-based
/// ids whose byte is set. The paired `_size` chunk is redundant here: unset and
/// trailing-omitted elements both contribute no id.
fn decode_flag_ids(data: &[u8]) -> Vec<u32> {
    data.iter()
        .enumerate()
        .filter_map(|(i, &b)| (b != 0).then_some(i as u32 + 1))
        .collect()
}

/// Parse the item table (`ChunkData::items` = `0x0D`) out of an LDB byte slice.
/// Chunk ids (liblcf `ChunkItem`): name `0x01`, description `0x02`, type `0x03`,
/// price `0x05`, uses `0x06`, atk `0x0B`, def `0x0C`, spi `0x0D`, agi `0x0E`,
/// two_handed `0x0F`, hit `0x11`, critical_hit `0x12`, animation_id `0x14`,
/// entire_party `0x1F`, recover_hp_rate `0x20`, recover_hp `0x21`,
/// recover_sp_rate `0x22`, recover_sp `0x23`, occasion_field1 `0x25`, ko_only
/// `0x26`, state_set `0x40`, attribute_set `0x42`. Scalars are integer chunks;
/// `two_handed`, `entire_party`, `occasion_field1`, and `ko_only` are 0/1 flags; the sets are
/// `vector<bool>` chunks (one byte per element). Omitted uses and animation id
/// default to `1`, hit to `90`, and the other fields to 0/false/empty.
pub fn parse_items(bytes: &[u8]) -> Result<Vec<Item>, LcfError> {
    let section = find_section(bytes, ITEM_SECTION, LcfError::MissingItems)?;
    let mut reader = Reader::new(section);
    let count = reader.varint()?;
    let mut items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.varint()?;
        let mut item = Item {
            prevent_critical: false,
            raise_evasion: false,
            half_sp_cost: false,
            actor_set: Vec::new(),
            state_chance: 0,
            id,
            name: String::new(),
            description: String::new(),
            item_type: 0,
            price: 0,
            uses: 1,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            scope: 0,
            only_field: false,
            ko_only: false,
            atk: 0,
            def: 0,
            spi: 0,
            agi: 0,
            two_handed: false,
            hit: 90,
            crit: 0,
            weapon_animation: 1,
            state_set: Vec::new(),
            attribute_set: Vec::new(),
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
                ITEM_USES => item.uses = Reader::new(sub_data).varint()?,
                ITEM_ATK => item.atk = Reader::new(sub_data).varint()?,
                ITEM_DEF => item.def = Reader::new(sub_data).varint()?,
                ITEM_SPI => item.spi = Reader::new(sub_data).varint()?,
                ITEM_AGI => item.agi = Reader::new(sub_data).varint()?,
                ITEM_TWO_HANDED => item.two_handed = Reader::new(sub_data).varint()? != 0,
                ITEM_HIT => item.hit = Reader::new(sub_data).varint()?,
                ITEM_CRITICAL_HIT => item.crit = Reader::new(sub_data).varint()?,
                ITEM_ANIMATION_ID => item.weapon_animation = Reader::new(sub_data).varint()?,
                ITEM_ENTIRE_PARTY => item.scope = Reader::new(sub_data).varint()?,
                ITEM_RECOVER_HP_RATE => item.recover_hp_rate = Reader::new(sub_data).varint()?,
                ITEM_RECOVER_HP => item.recover_hp = Reader::new(sub_data).varint()?,
                ITEM_RECOVER_SP_RATE => item.recover_sp_rate = Reader::new(sub_data).varint()?,
                ITEM_RECOVER_SP => item.recover_sp = Reader::new(sub_data).varint()?,
                ITEM_OCCASION_FIELD => item.only_field = Reader::new(sub_data).varint()? != 0,
                ITEM_KO_ONLY => item.ko_only = Reader::new(sub_data).varint()? != 0,
                ITEM_STATE_SET => item.state_set = decode_flag_ids(sub_data),
                0x3E => item.actor_set = sub_data.iter().map(|byte| *byte != 0).collect(),
                0x19 => item.prevent_critical = Reader::new(sub_data).varint()? != 0,
                0x1A => item.raise_evasion = Reader::new(sub_data).varint()? != 0,
                0x1B => item.half_sp_cost = Reader::new(sub_data).varint()? != 0,
                0x43 => item.state_chance = Reader::new(sub_data).varint()?,
                ITEM_ATTRIBUTE_SET => item.attribute_set = decode_flag_ids(sub_data),
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
    use crate::{Item, LcfError, parse_items};

    #[test]
    fn actor_flags_keep_false_entries_and_the_omitted_tail() {
        let ldb = make_ldb(&[(
            0x0D,
            section(&[
                element(1, &[subchunk(0x3E, &[1, 0, 1, 0])]),
                element(2, &[]),
            ]),
        )]);
        let items = parse_items(&ldb).unwrap();
        assert_eq!(items[0].actor_set, [true, false, true, false]);
        assert!(items[1].actor_set.is_empty());
    }

    #[test]
    fn parses_weapon_equipment_fields() {
        let desc = [0xC9, 0x6C, 0x65, 0x73, 0x20, 0x70, 0x65, 0x6E, 0x67, 0x65];
        let sword = element(
            1,
            &[
                subchunk(0x01, b"Ton-Kard"),
                subchunk(0x02, &desc),
                subchunk(0x03, &varint(1)),
                subchunk(0x05, &varint(1200)),
                subchunk(0x0B, &varint(10)),
                subchunk(0x0C, &varint(5)),
                subchunk(0x0F, &varint(1)),
                subchunk(0x11, &varint(85)),
                subchunk(0x12, &varint(5)),
                subchunk(0x14, &varint(2)),
                subchunk(0x40, &[0, 0, 1]),
                subchunk(0x42, &[1]),
            ],
        );
        let ldb = make_ldb(&[(0x0C, vec![7]), (0x0D, section(&[sword]))]);
        let items = parse_items(&ldb).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0],
            Item {
                prevent_critical: false,
                raise_evasion: false,
                half_sp_cost: false,
                actor_set: Vec::new(),
                state_chance: 0,
                id: 1,
                name: "Ton-Kard".to_string(),
                description: "Éles penge".to_string(),
                item_type: 1,
                price: 1200,
                uses: 1,
                recover_hp: 0,
                recover_hp_rate: 0,
                recover_sp: 0,
                recover_sp_rate: 0,
                scope: 0,
                only_field: false,
                ko_only: false,
                atk: 10,
                def: 5,
                spi: 0,
                agi: 0,
                two_handed: true,
                hit: 85,
                crit: 5,
                weapon_animation: 2,
                state_set: vec![3],
                attribute_set: vec![1],
            }
        );
    }

    #[test]
    fn parses_medicine_use_effect() {
        let potion = element(
            2,
            &[
                subchunk(0x01, b"Ital"),
                subchunk(0x03, &varint(6)),
                subchunk(0x05, &varint(50)),
                subchunk(0x1F, &varint(1)),
                subchunk(0x20, &varint(30)),
                subchunk(0x21, &varint(20)),
                subchunk(0x23, &varint(5)),
                subchunk(0x25, &varint(1)),
                subchunk(0x26, &varint(1)),
                subchunk(0x40, &[1, 0, 0, 1]),
            ],
        );
        let ldb = make_ldb(&[(0x0D, section(&[potion]))]);
        let item = &parse_items(&ldb).unwrap()[0];
        assert_eq!(item.item_type, 6, "medicine");
        assert_eq!(item.recover_hp_rate, 30);
        assert_eq!(item.recover_hp, 20);
        assert_eq!(item.recover_sp_rate, 0);
        assert_eq!(item.recover_sp, 5);
        assert_eq!(item.scope, 1, "whole party");
        assert!(item.only_field, "usable only from the map");
        assert!(item.ko_only, "effects restricted to fallen actors");
        assert_eq!(item.uses, 1);
        assert_eq!(item.state_set, vec![1, 4], "cured states");
        assert!(item.attribute_set.is_empty());
    }

    #[test]
    fn item_fields_default_when_omitted() {
        let normal = element(3, &[subchunk(0x01, b"Kulcs"), subchunk(0x03, &varint(0))]);
        let ldb = make_ldb(&[(0x0D, section(&[normal]))]);
        let item = &parse_items(&ldb).unwrap()[0];
        assert_eq!(item.id, 3);
        assert_eq!(item.name, "Kulcs");
        assert_eq!(item.item_type, 0, "normal");
        assert_eq!(item.price, 0, "omitted price defaults to 0");
        assert_eq!((item.atk, item.def, item.spi, item.agi), (0, 0, 0, 0));
        assert_eq!((item.hit, item.crit, item.weapon_animation), (90, 0, 1));
        assert_eq!(item.uses, 1);
        assert!(!item.two_handed && !item.only_field && !item.ko_only);
        assert_eq!(item.scope, 0);
        assert!(item.state_set.is_empty() && item.attribute_set.is_empty());
        assert!(item.description.is_empty());
    }

    #[test]
    fn explicit_zeroes_override_nonzero_item_defaults() {
        let item = element(
            1,
            &[
                subchunk(0x06, &varint(0)),
                subchunk(0x11, &varint(0)),
                subchunk(0x14, &varint(0)),
            ],
        );
        let ldb = make_ldb(&[(0x0D, section(&[item]))]);
        let item = &parse_items(&ldb).unwrap()[0];
        assert_eq!((item.uses, item.hit, item.weapon_animation), (0, 0, 0));
    }

    #[test]
    fn parse_items_errors_when_section_absent() {
        let ldb = make_ldb(&[(0x14, section(&[]))]);
        assert!(matches!(parse_items(&ldb), Err(LcfError::MissingItems)));
    }
}

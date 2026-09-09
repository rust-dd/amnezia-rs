use amnezia_data::ItemDef;
use std::path::Path;

fn varint(mut v: u32) -> Vec<u8> {
    let mut groups = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v != 0 {
        groups.push((v & 0x7F) as u8);
        v >>= 7;
    }
    groups.reverse();
    let last = groups.len() - 1;
    groups
        .iter()
        .enumerate()
        .map(|(i, b)| if i < last { b | 0x80 } else { *b })
        .collect()
}

fn subchunk(id: u32, data: &[u8]) -> Vec<u8> {
    let mut out = varint(id);
    out.extend(varint(data.len() as u32));
    out.extend_from_slice(data);
    out
}

fn element(id: u32, subchunks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = varint(id);
    for chunk in subchunks {
        out.extend_from_slice(chunk);
    }
    out.extend(varint(0));
    out
}

fn make_ldb(section_id: u32, elements: &[Vec<u8>]) -> Vec<u8> {
    let mut section = varint(elements.len() as u32);
    for e in elements {
        section.extend_from_slice(e);
    }
    let sig = b"LcfDataBase";
    let mut out = vec![sig.len() as u8];
    out.extend_from_slice(sig);
    out.extend(varint(section_id));
    out.extend(varint(section.len() as u32));
    out.extend_from_slice(&section);
    out.push(0);
    out
}

#[test]
fn converts_ldb_to_items_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_items_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let sword = element(
        1,
        &[
            subchunk(0x01, b"Ton-Kard"),
            subchunk(0x02, &[0xC9, 0x6C, 0x65, 0x73]),
            subchunk(0x03, &varint(1)),
            subchunk(0x05, &varint(1200)),
            subchunk(0x0B, &varint(10)),
            subchunk(0x0C, &varint(5)),
            subchunk(0x0F, &varint(1)),
            subchunk(0x11, &varint(85)),
            subchunk(0x12, &varint(5)),
            subchunk(0x14, &varint(2)),
            subchunk(0x19, &varint(1)),
            subchunk(0x1A, &varint(1)),
            subchunk(0x1B, &varint(1)),
            subchunk(0x3E, &[1, 0, 0]),
            subchunk(0x42, &[1]),
        ],
    );
    let potion = element(
        2,
        &[
            subchunk(0x01, b"Ital"),
            subchunk(0x03, &varint(6)),
            subchunk(0x1F, &varint(1)),
            subchunk(0x20, &varint(30)),
            subchunk(0x21, &varint(20)),
            subchunk(0x23, &varint(5)),
            subchunk(0x25, &varint(1)),
            subchunk(0x26, &varint(1)),
            subchunk(0x40, &[1, 0, 0, 1]),
        ],
    );
    let ldb = make_ldb(0x0D, &[sword, potion]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_items(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("items.ron")).unwrap();
    let items = ron::from_str::<Vec<ItemDef>>(&text).unwrap();
    assert_eq!(
        items[0],
        ItemDef {
            prevent_critical: true,
            raise_evasion: true,
            half_sp_cost: true,
            actor_set: vec![true, false, false],
            state_chance: 0,
            id: 1,
            name: "Ton-Kard".to_string(),
            description: "Éles".to_string(),
            item_type: 1,
            price: 1200,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            cure_states: vec![],
            scope: 0,
            only_field: false,
            ko_only: false,
            uses: 1,
            atk: 10,
            def: 5,
            spi: 0,
            agi: 0,
            attribute_defense: vec![1],
            state_defense: vec![],
            two_handed: true,
            hit: 85,
            crit: 5,
            weapon_animation: 2,
        }
    );
    let potion = &items[1];
    assert_eq!(potion.name, "Ital");
    assert_eq!(potion.item_type, 6);
    assert_eq!(potion.price, 0);
    assert_eq!(potion.recover_hp_rate, 30);
    assert_eq!(potion.recover_hp, 20);
    assert_eq!(potion.recover_sp, 5);
    assert_eq!(potion.scope, 1, "whole party");
    assert!(potion.only_field);
    assert!(potion.ko_only);
    assert_eq!(potion.uses, 1);
    assert_eq!(potion.cure_states, vec![1, 4]);
    assert!(
        potion.state_defense.is_empty() && potion.attribute_defense.is_empty(),
        "a consumable's sets are cure_states, not gear defenses"
    );
}

#[test]
fn omitted_item_fields_use_original_defaults_in_ron() {
    let item =
        ron::from_str::<ItemDef>(r#"(id:1,name:"Ital",description:"",item_type:6,price:10)"#)
            .unwrap();
    assert_eq!((item.uses, item.hit, item.weapon_animation), (1, 90, 1));
    assert!(!item.ko_only);
}

#[test]
fn explicit_zero_item_fields_survive_ron_loading() {
    let item = ron::from_str::<ItemDef>(
        r#"(id:1,name:"Ital",description:"",item_type:6,price:10,uses:0,hit:0,weapon_animation:0)"#,
    )
    .unwrap();
    assert_eq!((item.uses, item.hit, item.weapon_animation), (0, 0, 0));
    assert!(item.actor_set.is_empty());
    assert!(!item.prevent_critical && !item.raise_evasion && !item.half_sp_cost);
    assert!(item.usable_by_actor(1) && item.usable_by_actor(999));
    assert!(!item.usable_by_actor(0));
}

#[test]
fn equipment_state_probability_survives_ldb_and_ron_conversion() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_equipment_state_chance");
    let input = tmp.join("in");
    let output = tmp.join("out");
    std::fs::create_dir_all(&input).unwrap();
    let ldb = make_ldb(
        0x0D,
        &[
            element(
                1,
                &[subchunk(0x03, &varint(2)), subchunk(0x43, &varint(75))],
            ),
            element(2, &[]),
            element(3, &[subchunk(0x43, &varint(0))]),
        ],
    );
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();
    amnezia_convert::convert_items(&input, &output).unwrap();
    let items =
        ron::from_str::<Vec<ItemDef>>(&std::fs::read_to_string(output.join("items.ron")).unwrap())
            .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| item.state_chance)
            .collect::<Vec<_>>(),
        [75, 0, 0]
    );
}

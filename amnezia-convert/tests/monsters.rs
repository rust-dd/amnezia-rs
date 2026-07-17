use amnezia_data::MonsterDef;
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
fn converts_ldb_to_monsters_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_monsters_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // Name bytes are CP1250 "Sárkány" (dragon): 0xE1 = 'á'.
    let dragon = element(
        1,
        &[
            subchunk(0x01, &[0x53, 0xE1, 0x72, 0x6B, 0xE1, 0x6E, 0x79]),
            subchunk(0x02, b"Dragon1"),
            subchunk(0x04, &varint(999)),
            subchunk(0x06, &varint(180)),
            subchunk(0x0B, &varint(1500)),
            subchunk(0x0C, &varint(800)),
        ],
    );
    let slime = element(2, &[subchunk(0x01, b"Slime"), subchunk(0x04, &varint(30))]);
    let ldb = make_ldb(0x0E, &[dragon, slime]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_monsters(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("monsters.ron")).unwrap();
    let monsters: Vec<MonsterDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        monsters[0],
        MonsterDef {
            id: 1,
            name: "Sárkány".to_string(),
            battler: "Dragon1".to_string(),
            max_hp: 999,
            max_sp: 0,
            attack: 180,
            defense: 0,
            spirit: 0,
            agility: 0,
            exp: 1500,
            gold: 800,
            attribute_ranks: vec![],
            state_ranks: vec![],
            actions: vec![],
        }
    );
    assert_eq!(monsters[1].name, "Slime");
    assert_eq!(monsters[1].max_hp, 30);
    assert_eq!(monsters[1].attack, 0);
}

use amnezia_data::{TroopDef, TroopMemberDef};
use std::path::Path;

#[path = "troops/pages.rs"]
mod pages;

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
fn converts_ldb_to_troops_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_troops_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

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
    let ldb = make_ldb(0x0F, &[troop]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_troops(&input, &output).unwrap();
    assert_eq!(count, 1);

    let text = std::fs::read_to_string(output.join("troops.ron")).unwrap();
    let troops: Vec<TroopDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        troops[0],
        TroopDef {
            id: 1,
            name: "Őrök".to_string(),
            pages: Vec::new(),
            members: vec![
                TroopMemberDef {
                    enemy_id: 3,
                    x: 80,
                    y: 120
                },
                TroopMemberDef {
                    enemy_id: 5,
                    x: 160,
                    y: 0
                },
            ],
        }
    );
}

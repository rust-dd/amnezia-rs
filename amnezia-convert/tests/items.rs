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

    // Description bytes are CP1250: 0xC9 0x6C 0x65 0x73 -> "Éles".
    let sword = element(
        1,
        &[
            subchunk(0x01, b"Ton-Kard"),
            subchunk(0x02, &[0xC9, 0x6C, 0x65, 0x73]),
            subchunk(0x03, &varint(1)),
            subchunk(0x05, &varint(1200)),
        ],
    );
    let potion = element(2, &[subchunk(0x01, b"Ital"), subchunk(0x03, &varint(6))]);
    let ldb = make_ldb(0x0D, &[sword, potion]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_items(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("items.ron")).unwrap();
    let items: Vec<ItemDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        items[0],
        ItemDef {
            id: 1,
            name: "Ton-Kard".to_string(),
            description: "Éles".to_string(),
            item_type: 1,
            price: 1200,
        }
    );
    assert_eq!(items[1].name, "Ital");
    assert_eq!(items[1].item_type, 6);
    assert_eq!(items[1].price, 0);
}

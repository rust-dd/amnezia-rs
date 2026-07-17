use amnezia_data::AttributeDef;
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
fn converts_ldb_to_attributes_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_attributes_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // A physical weapon element "Kard" (sword) with a full rate grid except C,
    // which the editor never writes (defaults to 100).
    let sword = element(
        1,
        &[
            subchunk(0x01, b"Kard"),
            subchunk(0x02, &varint(0)),
            subchunk(0x0B, &varint(150)),
            subchunk(0x0C, &varint(125)),
            subchunk(0x0E, &varint(75)),
            subchunk(0x0F, &varint(50)),
        ],
    );
    // A magical element that overrides only A and B; C/D/E fall back to defaults.
    let fire = element(
        2,
        &[
            subchunk(0x01, b"Tuz"),
            subchunk(0x02, &varint(1)),
            subchunk(0x0B, &varint(200)),
            subchunk(0x0C, &varint(150)),
        ],
    );
    let ldb = make_ldb(0x11, &[sword, fire]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_attributes(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("attributes.ron")).unwrap();
    let attributes: Vec<AttributeDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        attributes[0],
        AttributeDef {
            id: 1,
            name: "Kard".to_string(),
            attribute_type: 0,
            a_rate: 150,
            b_rate: 125,
            c_rate: 100,
            d_rate: 75,
            e_rate: 50,
        }
    );
    assert_eq!(attributes[1].name, "Tuz");
    assert_eq!(attributes[1].attribute_type, 1, "magical");
    assert_eq!(
        (
            attributes[1].a_rate,
            attributes[1].b_rate,
            attributes[1].c_rate,
            attributes[1].d_rate,
            attributes[1].e_rate,
        ),
        (200, 150, 100, 50, 0)
    );
}

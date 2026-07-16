use amnezia_data::Chipset;
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

fn element(id: u32, name: &[u8]) -> Vec<u8> {
    let mut out = varint(id);
    out.extend(subchunk(0x02, name));
    out.extend(varint(0));
    out
}

fn make_ldb(elements: &[Vec<u8>]) -> Vec<u8> {
    let mut section = varint(elements.len() as u32);
    for e in elements {
        section.extend_from_slice(e);
    }
    let sig = b"LcfDataBase";
    let mut out = vec![sig.len() as u8];
    out.extend_from_slice(sig);
    out.extend(varint(0x14));
    out.extend(varint(section.len() as u32));
    out.extend_from_slice(&section);
    out.push(0);
    out
}

#[test]
fn converts_ldb_to_chipsets_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_chipsets_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let ldb = make_ldb(&[element(1, b"basis"), element(2, b"outline")]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_chipsets(&input, &output).unwrap();
    assert_eq!(count, 2);

    let ron = std::fs::read_to_string(output.join("chipsets.ron")).unwrap();
    let chipsets: Vec<Chipset> = ron::from_str(&ron).unwrap();
    assert_eq!(chipsets.len(), 2);
    assert_eq!(
        chipsets[0],
        Chipset {
            id: 1,
            graphic: "basis".to_string(),
            passages_down: vec![0x0F; 162],
            passages_up: vec![0x0F; 144],
        }
    );
    assert_eq!(chipsets[1].graphic, "outline");
}

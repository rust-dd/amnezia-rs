use amnezia_data::ActorDef;
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

fn parameters(curves: [[i16; 2]; 6]) -> Vec<u8> {
    curves.iter().flatten().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn converts_ldb_to_actors_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_actors_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let params = parameters([[40, 44], [12, 15], [0, 0], [0, 0], [0, 0], [0, 0]]);
    let ron = element(
        1,
        &[
            subchunk(0x01, b"Ron"),
            subchunk(0x02, b"Zsoldos"),
            subchunk(0x07, &varint(2)),
            subchunk(0x1F, &params),
        ],
    );
    let ldb = make_ldb(0x0B, &[ron]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_actors(&input, &output).unwrap();
    assert_eq!(count, 1);

    let text = std::fs::read_to_string(output.join("actors.ron")).unwrap();
    let actors: Vec<ActorDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        actors[0],
        ActorDef {
            id: 1,
            name: "Ron".to_string(),
            title: "Zsoldos".to_string(),
            level: 2,
            max_level: 2,
            hp: 44,
            sp: 15,
        }
    );
}

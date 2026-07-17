use amnezia_data::Hero;
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

fn actor(id: u32, name: &[u8]) -> Vec<u8> {
    let mut out = varint(id);
    out.extend(subchunk(0x01, name));
    out.extend(varint(0));
    out
}

fn make_ldb(actors: &[Vec<u8>]) -> Vec<u8> {
    let mut section = varint(actors.len() as u32);
    for a in actors {
        section.extend_from_slice(a);
    }
    let sig = b"LcfDataBase";
    let mut out = vec![sig.len() as u8];
    out.extend_from_slice(sig);
    out.extend(varint(0x0B));
    out.extend(varint(section.len() as u32));
    out.extend_from_slice(&section);
    out.push(0);
    out
}

#[test]
fn converts_ldb_to_hero_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_hero_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let ldb = make_ldb(&[actor(1, b"Ron"), actor(2, b"Tiffany")]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let name = amnezia_convert::convert_hero(&input, &output).unwrap();
    assert_eq!(name, "Ron");

    let ron = std::fs::read_to_string(output.join("hero.ron")).unwrap();
    let hero: Hero = ron::from_str(&ron).unwrap();
    assert_eq!(
        hero,
        Hero {
            name: "Ron".to_string()
        }
    );
}

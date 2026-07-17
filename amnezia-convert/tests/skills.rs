use amnezia_data::SkillDef;
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
fn converts_ldb_to_skills_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_skills_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // Name bytes are CP1250 "Tűzgolyó" (fireball): 0xFB = 'ű', 0xF3 = 'ó'.
    let fireball = element(
        1,
        &[
            subchunk(0x01, &[0x54, 0xFB, 0x7A, 0x67, 0x6F, 0x6C, 0x79, 0xF3]),
            subchunk(0x02, &[0xC9, 0x67, 0x65, 0x74, 0x69]),
            subchunk(0x0B, &varint(8)),
            subchunk(0x18, &varint(35)),
            subchunk(0x19, &varint(90)),
        ],
    );
    let heal = element(2, &[subchunk(0x01, b"Heal"), subchunk(0x0B, &varint(4))]);
    let ldb = make_ldb(0x0C, &[fireball, heal]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_skills(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("skills.ron")).unwrap();
    let skills: Vec<SkillDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        skills[0],
        SkillDef {
            id: 1,
            name: "Tűzgolyó".to_string(),
            description: "Égeti".to_string(),
            sp_cost: 8,
            power: 35,
            hit: 90,
        }
    );
    assert_eq!(skills[1].name, "Heal");
    assert_eq!(skills[1].sp_cost, 4);
    assert_eq!((skills[1].power, skills[1].hit), (0, 0));
}

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

    let fireball = element(
        1,
        &[
            subchunk(0x01, &[0x54, 0xFB, 0x7A, 0x67, 0x6F, 0x6C, 0x79, 0xF3]),
            subchunk(0x02, &[0xC9, 0x67, 0x65, 0x74, 0x69]),
            subchunk(0x03, b" t\xfczet id\xe9z"),
            subchunk(0x04, b"\xc9get"),
            subchunk(0x07, &varint(3)),
            subchunk(0x08, &varint(0)),
            subchunk(0x0B, &varint(8)),
            subchunk(0x0C, &varint(0)),
            subchunk(0x0E, &varint(12)),
            subchunk(0x16, &varint(10)),
            subchunk(0x17, &varint(6)),
            subchunk(0x18, &varint(35)),
            subchunk(0x19, &varint(90)),
            subchunk(0x1F, &varint(1)),
            subchunk(0x21, &varint(1)),
            subchunk(0x22, &varint(0)),
            subchunk(0x23, &varint(1)),
            subchunk(0x24, &varint(0)),
            subchunk(0x26, &varint(1)),
            subchunk(0x2A, &[0, 0, 1]),
            subchunk(0x2C, &[0, 0, 0, 0, 1]),
        ],
    );
    let heal = element(2, &[subchunk(0x01, b"Heal"), subchunk(0x0B, &varint(4))]);
    let ldb = make_ldb(0x0C, &[fireball, heal]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_skills(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("skills.ron")).unwrap();
    let skills = ron::from_str::<Vec<SkillDef>>(&text).unwrap();
    assert_eq!(
        skills[0],
        SkillDef {
            using_message1: " tüzet idéz".into(),
            using_message2: "Éget".into(),
            affect_stats: [true, false, true, false],
            ignore_defense: true,
            id: 1,
            name: "Tűzgolyó".to_string(),
            description: "Égeti".to_string(),
            sp_cost: 8,
            power: 35,
            hit: 90,
            failure_message: 3,
            skill_type: 0,
            scope: 0,
            animation_id: 12,
            physical_rate: 0,
            magical_rate: 10,
            variance: 6,
            affect_hp: true,
            affect_sp: false,
            absorb: false,
            attributes: vec![5],
            affected_states: vec![3],
        }
    );
    assert_eq!(skills[1].name, "Heal");
    assert!(skills[1].using_message1.is_empty());
    assert!(skills[1].using_message2.is_empty());
    assert_eq!(skills[1].animation_id, 1);
    assert_eq!(skills[1].affect_stats, [false; 4]);
    assert!(!skills[1].ignore_defense);
    assert_eq!(skills[1].sp_cost, 4);
    assert_eq!((skills[1].power, skills[1].hit), (0, 100));
    assert_eq!(skills[1].failure_message, 0);
    assert_eq!(
        skills[1].magical_rate, 3,
        "omitted magical_rate keeps the RM2000 default"
    );
    assert_eq!(
        skills[1].variance, 4,
        "omitted variance keeps the RM2000 default"
    );
    assert!(skills[1].attributes.is_empty() && skills[1].affected_states.is_empty());
}

#[test]
fn skill_animation_default_and_explicit_zero_survive_conversion_and_legacy_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("skill_animation_default");
    let input = tmp.join("in");
    let output = tmp.join("out");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(
        input.join("RPG_RT.ldb"),
        make_ldb(
            0x0C,
            &[element(1, &[]), element(2, &[subchunk(0x0E, &varint(0))])],
        ),
    )
    .unwrap();
    amnezia_convert::convert_skills(&input, &output).unwrap();
    let skills = ron::from_str::<Vec<SkillDef>>(
        &std::fs::read_to_string(output.join("skills.ron")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        skills.iter().map(|s| s.animation_id).collect::<Vec<_>>(),
        [1, 0]
    );
    let legacy = ron::from_str::<SkillDef>(
        &ron::to_string(&skills[0])
            .unwrap()
            .replace("animation_id:1,", "")
            .replace("using_message1:\"\",", "")
            .replace("using_message2:\"\",", ""),
    )
    .unwrap();
    assert_eq!(legacy.animation_id, 1);
    assert!(legacy.using_message1.is_empty());
    assert!(legacy.using_message2.is_empty());
}

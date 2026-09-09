use amnezia_data::{ActorCurves, ActorDef, Learning};
use std::path::Path;

#[test]
fn legacy_actor_ron_retains_an_empty_state_rank_tail() {
    let legacy = r#"(id:1,name:"Ron",title:"",level:1,max_level:50,hp:10,sp:10)"#;
    let actor = ron::from_str::<ActorDef>(legacy).unwrap();
    assert!(actor.state_ranks.is_empty());
    assert!(actor.attribute_ranks.is_empty());
    assert!(actor.critical_hit);
    assert_eq!(actor.critical_hit_chance, 30);
}

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
    curves
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// Build a `skills` chunk (`0x3F`): the `rpg::Learning` array as a `[count]`
/// header then, per entry, a 1-based index and its level (`0x01`) / skill_id
/// (`0x02`) sub-chunks.
fn learnings(entries: &[(u32, u32)]) -> Vec<u8> {
    let mut out = varint(entries.len() as u32);
    for (i, &(level, skill_id)) in entries.iter().enumerate() {
        out.extend_from_slice(&element(
            i as u32 + 1,
            &[
                subchunk(0x01, &varint(level)),
                subchunk(0x02, &varint(skill_id)),
            ],
        ));
    }
    out
}

#[test]
fn converts_ldb_to_actors_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_actors_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    let params = parameters([[40, 44], [12, 15], [5, 7], [4, 6], [3, 5], [6, 9]]);
    // initial_equipment (0x33) is five Int16 (LE) item ids: weapon 1, shield 0,
    // armor 64, helmet 83, accessory 0; 0x15 marks dual wielding; 0x38 the
    // unarmed attack animation.
    let ron = element(
        1,
        &[
            subchunk(0x01, b"Ron"),
            subchunk(0x02, b"Zsoldos"),
            subchunk(0x0F, b"Ron"),
            subchunk(0x10, &varint(0)),
            subchunk(0x07, &varint(2)),
            subchunk(0x09, &varint(0)),
            subchunk(0x0A, &varint(20)),
            subchunk(0x15, &varint(1)),
            subchunk(0x1F, &params),
            subchunk(0x3F, &learnings(&[(1, 10), (5, 12)])),
            subchunk(0x29, &varint(31)),
            subchunk(0x2A, &varint(29)),
            subchunk(0x2B, &varint(40)),
            subchunk(0x33, &[1, 0, 0, 0, 64, 0, 83, 0, 0, 0]),
            subchunk(0x38, &varint(9)),
            subchunk(0x48, &[0, 4, 2]),
            subchunk(0x4A, &[2, 1, 4]),
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
            critical_hit: false,
            critical_hit_chance: 20,
            attribute_ranks: vec![2, 1, 4],
            state_ranks: vec![0, 4, 2],
            id: 1,
            name: "Ron".to_string(),
            title: "Zsoldos".to_string(),
            level: 2,
            max_level: 2,
            hp: 44,
            sp: 15,
            curves: ActorCurves {
                max_hp: vec![40, 44],
                max_sp: vec![12, 15],
                attack: vec![5, 7],
                defense: vec![4, 6],
                spirit: vec![3, 5],
                agility: vec![6, 9],
            },
            learnings: vec![
                Learning {
                    level: 1,
                    skill_id: 10,
                },
                Learning {
                    level: 5,
                    skill_id: 12,
                },
            ],
            exp_base: 31,
            exp_inflation: 29,
            exp_correction: 40,
            weapon: 1,
            shield: 0,
            armor: 64,
            helmet: 83,
            accessory: 0,
            two_weapons: true,
            fix_equipment: false,
            unarmed_animation: 9,
            face_name: "Ron".to_string(),
            face_index: 0,
        }
    );
}

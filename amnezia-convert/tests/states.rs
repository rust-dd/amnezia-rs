use amnezia_data::StateDef;
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
fn converts_ldb_to_states_ron() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("converts_states_ron");
    let input = tmp.join("in");
    let output = tmp.join("out");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&input).unwrap();

    // Sleep: can't act (restriction 1), held 1 turn, 25% per-turn wake-up,
    // 50% wake-up when hit. Name bytes are ASCII "Alvas".
    let sleep = element(
        1,
        &[
            subchunk(0x01, b"Alvas"),
            subchunk(0x04, &varint(55)),
            subchunk(0x05, &varint(1)),
            subchunk(0x15, &varint(1)),
            subchunk(0x16, &varint(25)),
            subchunk(0x17, &varint(50)),
        ],
    );
    // Poison: acts normally (restriction omitted -> 0), priority omitted -> 50,
    // and carries an explicit per-turn HP-change block: type 0x2D=1, max-percent
    // 0x3D=10, flat val 0x3E=5.
    let poison = element(
        2,
        &[
            subchunk(0x01, b"Mereg"),
            subchunk(0x2D, &varint(1)),
            subchunk(0x3D, &varint(10)),
            subchunk(0x3E, &varint(5)),
        ],
    );
    let ldb = make_ldb(0x12, &[sleep, poison]);
    std::fs::write(input.join("RPG_RT.ldb"), ldb).unwrap();

    let count = amnezia_convert::convert_states(&input, &output).unwrap();
    assert_eq!(count, 2);

    let text = std::fs::read_to_string(output.join("states.ron")).unwrap();
    let states: Vec<StateDef> = ron::from_str(&text).unwrap();
    assert_eq!(
        states[0],
        StateDef {
            id: 1,
            name: "Alvas".to_string(),
            restriction: 1,
            priority: 55,
            hold_turn: 1,
            auto_release_prob: 25,
            release_by_damage: 50,
            hp_change_type: 0,
            hp_change_max: 0,
            hp_change_val: 0,
            hp_change_map_steps: 0,
            hp_change_map_val: 0,
        }
    );
    assert_eq!(states[1].name, "Mereg");
    assert_eq!(states[1].restriction, 0);
    assert_eq!(states[1].priority, 50, "omitted priority defaults to 50");
    assert_eq!(
        (
            states[1].hold_turn,
            states[1].auto_release_prob,
            states[1].release_by_damage
        ),
        (0, 0, 0)
    );
    assert_eq!(
        (
            states[1].hp_change_type,
            states[1].hp_change_max,
            states[1].hp_change_val
        ),
        (1, 10, 5),
        "poison's per-turn HP drain round-trips through the RON"
    );
    assert_eq!(
        (states[1].hp_change_map_steps, states[1].hp_change_map_val),
        (0, 0),
        "omitted map-step drain defaults to 0"
    );
}

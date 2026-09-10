use amnezia_data::StateDef;
use std::path::Path;

#[test]
fn legacy_state_ron_uses_editor_rates_without_overwriting_explicit_zero() {
    let legacy = r#"(id:1,name:"",restriction:0,priority:50,hold_turn:0,auto_release_prob:0,release_by_damage:0)"#;
    assert_eq!(
        ron::from_str::<StateDef>(legacy).unwrap().rates,
        [100, 80, 60, 30, 0]
    );
    let explicit = legacy.replacen('(', "(rates:(0,0,0,0,0),", 1);
    assert_eq!(ron::from_str::<StateDef>(&explicit).unwrap().rates, [0; 5]);
    let state = ron::from_str::<StateDef>(legacy).unwrap();
    assert_eq!(state.reduce_hit_ratio, 100);
    assert_eq!(state.color, 6);
    assert!(state.message_actor.is_empty());
    assert!(state.message_enemy.is_empty());
    assert!(state.message_already.is_empty());
    assert!(state.message_affected.is_empty());
    assert!(state.message_recovery.is_empty());
    let zero_color = legacy.replacen('(', "(color:0,", 1);
    assert_eq!(ron::from_str::<StateDef>(&zero_color).unwrap().color, 0);
    assert_eq!(state.affect_stats, [false; 4]);
    assert!(!state.restrict_magic && !state.restrict_skill);
    let zero = legacy.replacen('(', "(reduce_hit_ratio:0,", 1);
    assert_eq!(
        ron::from_str::<StateDef>(&zero).unwrap().reduce_hit_ratio,
        0
    );
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
            subchunk(0x03, &[0]),
            subchunk(0x33, b" alszik"),
            subchunk(0x34, b" elalszik"),
            subchunk(0x35, b" m\xe1r alszik"),
            subchunk(0x36, b" pihen"),
            subchunk(0x37, b" fel\xe9bred"),
            subchunk(0x04, &varint(55)),
            subchunk(0x05, &varint(1)),
            subchunk(0x15, &varint(1)),
            subchunk(0x16, &varint(25)),
            subchunk(0x17, &varint(50)),
            subchunk(0x0B, &varint(0)),
            subchunk(0x0E, &varint(40)),
            subchunk(0x1E, &varint(1)),
            subchunk(0x1F, &varint(1)),
            subchunk(0x20, &varint(1)),
            subchunk(0x21, &varint(0)),
            subchunk(0x22, &varint(1)),
            subchunk(0x23, &varint(20)),
            subchunk(0x29, &varint(1)),
            subchunk(0x2A, &varint(4)),
            subchunk(0x2B, &varint(1)),
            subchunk(0x2C, &varint(1)),
            subchunk(0x2E, &varint(1)),
            subchunk(0x41, &varint(2)),
            subchunk(0x42, &varint(1)),
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
            color: 0,
            message_actor: " alszik".into(),
            message_enemy: " elalszik".into(),
            message_already: " már alszik".into(),
            message_affected: " pihen".into(),
            message_recovery: " felébred".into(),
            affect_type: 1,
            affect_stats: [true, true, false, true],
            reduce_hit_ratio: 20,
            restrict_skill: true,
            restrict_skill_level: 4,
            restrict_magic: true,
            restrict_magic_level: 1,
            sp_change_type: 1,
            sp_change_max: 2,
            sp_change_val: 1,
            rates: [0, 80, 60, 40, 0],
            persistence: 0,
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
    assert_eq!(states[1].color, 6);
    assert!(states[1].message_actor.is_empty());
    assert_eq!(states[1].rates, [100, 80, 60, 30, 0]);
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

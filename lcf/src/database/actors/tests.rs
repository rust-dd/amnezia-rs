use crate::test_util::{element, make_ldb, section, subchunk, varint};
use crate::{LcfError, Learning, StatCurves, parse_actors};

/// Build a `skills` chunk (`0x3F`): a `[count]` header then per learning a
/// 1-based index and its `level` (`0x01`) / `skill_id` (`0x02`) sub-chunks.
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
fn parses_skill_learning_list() {
    let hero = element(1, &[subchunk(0x3F, &learnings(&[(1, 5), (3, 8), (7, 12)]))]);
    let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
    let actor = &parse_actors(&ldb).unwrap()[0];
    assert_eq!(
        actor.skills,
        vec![
            Learning {
                level: 1,
                skill_id: 5
            },
            Learning {
                level: 3,
                skill_id: 8
            },
            Learning {
                level: 7,
                skill_id: 12
            },
        ]
    );
}

#[test]
fn skill_learning_list_defaults_empty_and_drops_blank_rows() {
    // An entry with no skill_id chunk (skill 0) is a blank row RM2000 ignores.
    let hero = element(2, &[subchunk(0x3F, &learnings(&[(4, 0)]))]);
    let ldb = make_ldb(&[(0x0B, section(&[hero, element(3, &[])]))]);
    let actors = parse_actors(&ldb).unwrap();
    assert!(actors[0].skills.is_empty(), "blank learning row dropped");
    assert!(actors[1].skills.is_empty(), "omitted list defaults empty");
}

/// Build a `Parameters` chunk (`0x1F`) by concatenating the six Int16 stat
/// curves (max HP, max SP, attack, defense, spirit, agility) as little-endian
/// bytes. The caller passes six curves of equal length.
fn parameters(curves: &[&[i16]]) -> Vec<u8> {
    curves
        .iter()
        .flat_map(|curve| curve.iter())
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[test]
fn parses_actor_definition_fields() {
    let params = parameters(&[&[10, 20], &[5, 8], &[0, 0], &[0, 0], &[0, 0], &[0, 0]]);
    let hero = element(
        1,
        &[
            subchunk(0x01, &[0x41, 0x64, 0xE9, 0x6C]),
            subchunk(
                0x02,
                &[0xC9, 0x6E, 0x65, 0x6B, 0x65, 0x73, 0x6C, 0xE1, 0x6E, 0x79],
            ),
            subchunk(0x07, &varint(2)),
            subchunk(0x1F, &params),
        ],
    );
    let ldb = make_ldb(&[(0x05, vec![9, 9]), (0x0B, section(&[hero]))]);
    let actors = parse_actors(&ldb).unwrap();
    assert_eq!(actors.len(), 1);
    let ron = &actors[0];
    assert_eq!(ron.id, 1);
    assert_eq!(ron.name, "Adél");
    assert_eq!(ron.title, "Énekeslány");
    assert_eq!(ron.initial_level, 2);
    assert_eq!(
        ron.max_level, 2,
        "max level falls back to the 2-level curve length"
    );
    assert_eq!(ron.initial_hp, 20, "maxhp curve at level 2");
    assert_eq!(ron.initial_sp, 8, "maxsp curve at level 2");
    assert_eq!(ron.stat_curves.max_hp, vec![10, 20]);
    assert_eq!(ron.stat_curves.max_sp, vec![5, 8]);
    assert_eq!(
        (ron.exp_base, ron.exp_inflation, ron.exp_correction),
        (30, 30, 0),
        "experience fields default when omitted"
    );
}

#[test]
fn parses_stat_curves_and_experience() {
    let params = parameters(&[
        &[30, 40, 55],
        &[10, 14, 20],
        &[5, 7, 9],
        &[4, 6, 8],
        &[3, 5, 7],
        &[6, 9, 12],
    ]);
    let hero = element(
        1,
        &[
            subchunk(0x07, &varint(2)),
            subchunk(0x1F, &params),
            subchunk(0x29, &varint(31)),
            subchunk(0x2A, &varint(29)),
            subchunk(0x2B, &varint(40)),
        ],
    );
    let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
    let actor = &parse_actors(&ldb).unwrap()[0];
    assert_eq!(actor.max_level, 3, "curve length gives the max level");
    assert_eq!(
        actor.stat_curves,
        StatCurves {
            max_hp: vec![30, 40, 55],
            max_sp: vec![10, 14, 20],
            attack: vec![5, 7, 9],
            defense: vec![4, 6, 8],
            spirit: vec![3, 5, 7],
            agility: vec![6, 9, 12],
        }
    );
    assert_eq!(actor.initial_hp, 40, "maxhp curve at level 2");
    assert_eq!(actor.initial_sp, 14, "maxsp curve at level 2");
    assert_eq!(
        (actor.exp_base, actor.exp_inflation, actor.exp_correction),
        (31, 29, 40)
    );
}

#[test]
fn actor_final_level_overrides_curve_length() {
    let params = parameters(&[&[10, 20], &[5, 8], &[0, 0], &[0, 0], &[0, 0], &[0, 0]]);
    let hero = element(1, &[subchunk(0x08, &varint(50)), subchunk(0x1F, &params)]);
    let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
    let actors = parse_actors(&ldb).unwrap();
    assert_eq!(actors[0].max_level, 50);
    assert_eq!(actors[0].initial_level, 1, "initial level defaults to 1");
    assert_eq!(actors[0].initial_hp, 10, "maxhp curve at default level 1");
    assert_eq!(
        actors[0].stat_curves.max_hp.len(),
        50,
        "curves are padded to max_level, repeating the last stored value"
    );
    assert_eq!(actors[0].stat_curves.max_hp[49], 20);
}

#[test]
fn actor_defaults_when_fields_omitted() {
    let actor = element(3, &[]);
    let ldb = make_ldb(&[(0x0B, section(&[actor]))]);
    let actors = parse_actors(&ldb).unwrap();
    let a = &actors[0];
    assert_eq!(a.id, 3);
    assert!(a.name.is_empty());
    assert!(a.title.is_empty());
    assert_eq!(a.initial_level, 1);
    assert_eq!(a.max_level, 1);
    assert_eq!((a.initial_hp, a.initial_sp), (0, 0));
    assert_eq!(a.stat_curves.max_hp, vec![0]);
    assert_eq!((a.exp_base, a.exp_inflation, a.exp_correction), (30, 30, 0));
    assert_eq!(
        (a.weapon, a.shield, a.armor, a.helmet, a.accessory),
        (0, 0, 0, 0, 0),
        "empty equipment slots default to 0"
    );
    assert!(!a.two_weapons && !a.fix_equipment);
    assert_eq!(a.unarmed_animation, 1);
    assert!(a.character_name.is_empty());
    assert_eq!(a.character_index, 0);
    assert!(!a.rename_skill);
    assert!(a.skill_name.is_empty());
}

#[test]
fn parses_initial_equipment_and_flags() {
    let equipment = [1, 0, 0, 0, 64, 0, 83, 0, 0, 0];
    let hero = element(
        1,
        &[
            subchunk(0x15, &varint(1)),
            subchunk(0x16, &varint(1)),
            subchunk(0x33, &equipment),
            subchunk(0x38, &varint(9)),
        ],
    );
    let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
    let actor = &parse_actors(&ldb).unwrap()[0];
    assert_eq!(
        (
            actor.weapon,
            actor.shield,
            actor.armor,
            actor.helmet,
            actor.accessory
        ),
        (1, 0, 64, 83, 0)
    );
    assert!(actor.two_weapons, "dual wielding");
    assert!(actor.fix_equipment, "equipment locked");
    assert_eq!(actor.unarmed_animation, 9);
}

#[test]
fn parses_face_graphic() {
    let hero = element(1, &[subchunk(0x0F, b"Ron"), subchunk(0x10, &varint(3))]);
    let ldb = make_ldb(&[(0x0B, section(&[hero]))]);
    let actor = &parse_actors(&ldb).unwrap()[0];
    assert_eq!(actor.face_name, "Ron");
    assert_eq!(actor.face_index, 3);
}

#[test]
fn face_graphic_defaults_when_omitted() {
    let ldb = make_ldb(&[(0x0B, section(&[element(2, &[])]))]);
    let actor = &parse_actors(&ldb).unwrap()[0];
    assert!(actor.face_name.is_empty());
    assert_eq!(actor.face_index, 0);
}

#[test]
fn parse_actors_errors_when_section_absent() {
    let ldb = make_ldb(&[(0x14, section(&[]))]);
    assert!(matches!(parse_actors(&ldb), Err(LcfError::MissingActors)));
}

#[test]
fn actor_state_ranks_keep_each_byte_and_a_missing_tail() {
    let ldb = make_ldb(&[(
        0x0B,
        section(&[element(1, &[subchunk(0x48, &[0, 4, 2])]), element(2, &[])]),
    )]);
    let actors = parse_actors(&ldb).unwrap();
    assert_eq!(actors[0].state_ranks, [0, 4, 2]);
    assert!(actors[1].state_ranks.is_empty());
}

#[test]
fn actor_attribute_ranks_keep_each_byte_and_a_missing_tail() {
    let ldb = make_ldb(&[(
        0x0B,
        section(&[element(1, &[subchunk(0x4A, &[2, 1, 4])]), element(2, &[])]),
    )]);
    let actors = parse_actors(&ldb).unwrap();
    assert_eq!(actors[0].attribute_ranks, [2, 1, 4]);
    assert!(actors[1].attribute_ranks.is_empty());
}

#[test]
fn actor_criticals_preserve_defaults_and_explicit_values() {
    let ldb = make_ldb(&[(
        0x0B,
        section(&[
            element(1, &[]),
            element(2, &[subchunk(9, &[0]), subchunk(10, &[20])]),
            element(3, &[subchunk(9, &[1]), subchunk(10, &[0])]),
        ]),
    )]);
    let actors = parse_actors(&ldb).unwrap();
    assert_eq!(
        (actors[0].critical_hit, actors[0].critical_hit_chance),
        (true, 30)
    );
    assert_eq!(
        (actors[1].critical_hit, actors[1].critical_hit_chance),
        (false, 20)
    );
    assert_eq!(
        (actors[2].critical_hit, actors[2].critical_hit_chance),
        (true, 0)
    );
}

#[test]
fn actor_graphics_and_skill_labels_keep_empty_overrides_and_explicit_zero() {
    let ldb = make_ldb(&[(
        0x0B,
        section(&[
            element(
                1,
                &[
                    subchunk(3, b"Chara4"),
                    subchunk(4, &[2]),
                    subchunk(0x42, &[1]),
                    subchunk(0x43, b"Penget\xe1nc"),
                    subchunk(0x38, &[0]),
                ],
            ),
            element(2, &[subchunk(0x42, &[1])]),
            element(3, &[subchunk(0x42, &[0]), subchunk(0x43, b"Unused")]),
        ]),
    )]);
    let actors = parse_actors(&ldb).unwrap();
    assert_eq!(actors[0].character_name, "Chara4");
    assert_eq!(actors[0].character_index, 2);
    assert_eq!(actors[0].skill_name, "Pengetánc");
    assert!(actors[0].rename_skill);
    assert_eq!(actors[0].unarmed_animation, 0);
    assert!(actors[1].rename_skill);
    assert!(actors[1].skill_name.is_empty());
    assert!(!actors[2].rename_skill);
    assert_eq!(actors[2].skill_name, "Unused");
}

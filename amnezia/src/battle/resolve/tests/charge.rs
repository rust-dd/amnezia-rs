use super::*;

#[test]
fn a_charged_double_attack_uses_the_same_bonus_for_both_hits() {
    let mut battle = build_1v2();
    battle.members[0].hp = 1000;
    battle.members[0].max_hp = 1000;
    wind_enemy_hits(&mut battle, &[0, 2]);
    let rng = battle.rng;
    let mut expected = 0;
    for _ in 0..2 {
        battle.enemies[0].charging = true;
        expected += battle.enemy_strike_member(0, 0).unwrap();
    }
    let expected_rng = battle.rng;
    battle.rng = rng;
    battle.members[0].hp = 1000;
    battle.enemies[0].charging = true;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::DoubleAttack { target: 0 },
        agility: 1,
    });
    assert_eq!(battle.members[0].hp, 1000 - expected);
    assert_eq!(battle.rng, expected_rng);
    assert!(!battle.enemies[0].charging);
}

#[test]
fn a_missed_first_hit_does_not_remove_the_second_hits_captured_charge() {
    let mut battle = build_1v2();
    battle.members[0].stats.agility = 10;
    battle.enemies[0].stats.agility = 10;
    loop {
        let mut probe = battle.rng;
        if rng_next(&mut probe) % 100 >= 90 && rng_next(&mut probe) % 100 < 90 {
            break;
        }
        rng_next(&mut battle.rng);
    }
    let rng = battle.rng;
    let hp = battle.members[0].hp;
    battle.enemies[0].charging = true;
    assert!(battle.enemy_strike_member(0, 0).is_none());
    battle.enemies[0].charging = true;
    let expected = battle.enemy_strike_member(0, 0).unwrap();
    let expected_rng = battle.rng;
    battle.rng = rng;
    battle.members[0].hp = hp;
    battle.enemies[0].charging = true;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::DoubleAttack { target: 0 },
        agility: 1,
    });
    assert_eq!(battle.members[0].hp, hp - expected);
    assert_eq!(battle.rng, expected_rng);
    assert!(!battle.enemies[0].charging);
}

#[test]
fn a_real_non_attack_consumes_charge_but_a_cancelled_action_does_not() {
    for kind in [
        Command::Defend,
        Command::Observe,
        logic::enemy_command(Some(&enemy_action_def(7)), 0),
        Command::Skill {
            skill_id: 1,
            target: 0,
        },
    ] {
        let mut battle = build_1v2();
        battle.skills = vec![damage_skill(1, 5, vec![], vec![])];
        battle.enemies[0].charging = true;
        battle.apply(Action {
            source: Source::Enemy(0),
            kind,
            agility: 1,
        });
        assert!(!battle.enemies[0].charging);
    }
    let mut battle = build_1v2();
    battle.enemies[0].charging = true;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Nothing,
        agility: 1,
    });
    assert!(battle.enemies[0].charging);
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Charge,
        agility: 1,
    });
    assert!(battle.enemies[0].charging);
    battle.skills = vec![damage_skill(1, 5, vec![], vec![])];
    battle.enemies[0].sp = 0;
    battle.queue = vec![Action {
        source: Source::Enemy(0),
        kind: Command::Skill {
            skill_id: 1,
            target: 0,
        },
        agility: 1,
    }];
    battle.resolve_next();
    assert!(battle.enemies[0].charging);
}

#[test]
fn original_enemy_charge_and_double_attack_combinations_remain_present() {
    let monsters = crate::assets::load_ron::<Vec<amnezia_data::MonsterDef>>(&format!(
        "{}/monsters.ron",
        crate::assets::asset_root()
    ));
    let charged = monsters
        .iter()
        .filter(|monster| {
            monster
                .actions
                .iter()
                .any(|action| action.kind == 0 && action.basic == 4)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        charged.iter().map(|monster| monster.id).collect::<Vec<_>>(),
        [6, 9, 14, 15, 16, 17, 20, 29, 38, 41]
    );
    assert_eq!(
        charged
            .iter()
            .filter(|monster| {
                monster
                    .actions
                    .iter()
                    .any(|action| action.kind == 0 && action.basic == 1)
            })
            .map(|monster| monster.id)
            .collect::<Vec<_>>(),
        [6, 9, 16, 17, 20, 38, 41]
    );
}

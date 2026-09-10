use super::*;

#[test]
fn skill_failure_sounds_only_play_for_the_physical_dodge_message_mode() {
    for failure in [0, 1, 2, 3, 9] {
        for scope in [0, 3] {
            for target in [Source::Party(0), Source::Enemy(0)] {
                let mut battle = build_1v2();
                battle.members[0].stats.agility = 10;
                battle.enemies[0].stats.agility = 10;
                let mut skill = damage_skill(1, 20, vec![], vec![]);
                skill.scope = scope;
                skill.hit = 0;
                skill.failure_message = failure;
                if scope == 0 {
                    battle.skill_hit_battler(Source::Party(0), target, &skill);
                } else {
                    battle.skill_heal_battler(Source::Party(0), target, &skill);
                }
                assert_eq!(
                    battle.pending_se.contains(&BattleSe::Dodge),
                    failure == 3,
                    "failure mode {failure}, scope {scope}"
                );
                assert_eq!(battle.pending_se.len(), usize::from(failure == 3));
            }
        }
    }
}

#[test]
fn hp_absorption_omits_normal_damage_sound_and_blink_but_keeps_lethal_feedback() {
    for target in [Source::Party(0), Source::Enemy(0)] {
        for absorb in [false, true] {
            for lethal in [false, true] {
                let mut battle = build_1v2();
                let source = match target {
                    Source::Party(_) => Source::Enemy(0),
                    Source::Enemy(_) => Source::Party(0),
                };
                let hp = if lethal { 1 } else { 30 };
                match target {
                    Source::Party(i) => battle.members[i].hp = hp,
                    Source::Enemy(i) => battle.enemies[i].hp = hp,
                }
                let mut skill = damage_skill(1, 5, vec![], vec![]);
                skill.absorb = absorb;
                skill.magical_rate = 0;
                skill.variance = 0;
                battle.skill_hit_battler(source, target, &skill);
                assert_eq!(battle.battler_hp(target), (hp - 5).max(0));
                assert_eq!(
                    battle
                        .pending_se
                        .iter()
                        .any(|se| matches!(se, BattleSe::ActorDamaged | BattleSe::EnemyDamaged)),
                    !absorb
                );
                let enemy = matches!(target, Source::Enemy(_));
                assert_eq!(battle.pending_blinks.len(), usize::from(enemy && !absorb));
                assert_eq!(
                    battle.pending_se.contains(&BattleSe::EnemyDefeated),
                    enemy && lethal
                );
                if enemy && lethal {
                    assert!(battle.enemies[0].dying.is_some());
                }
            }
        }
    }
}

#[test]
fn double_attack_stops_after_a_lethal_first_hit_without_retargeting_or_extra_rolls() {
    let mut battle = build_party2();
    battle.members[0].hp = 1;
    wind_enemy_hits(&mut battle, &[0]);
    let rng = battle.rng;
    let ally_hp = battle.members[1].hp;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 1,
    });
    assert_eq!(battle.members[0].hp, 0);
    let after_single = battle.rng;
    battle.members[0].hp = 1;
    battle.members[0].states.clear();
    battle.rng = rng;
    battle.hit_reports.clear();
    battle.pending_se.clear();
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::DoubleAttack { target: 0 },
        agility: 1,
    });
    assert_eq!(battle.members[0].hp, 0);
    assert_eq!(battle.members[1].hp, ally_hp);
    assert_eq!(battle.rng, after_single);
    assert_eq!(battle.hit_reports.len(), 1);
    assert_eq!(battle.pending_se.len(), 1);
}

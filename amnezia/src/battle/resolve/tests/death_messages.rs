use super::*;

fn original_battle() -> Battle {
    let mut battle = build_1v2();
    messages::original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.enemies[0].name = "Bandita".into();
    battle
}

#[test]
fn a_lethal_plain_or_critical_weapon_hit_reports_damage_before_collapse() {
    for crit in [false, true] {
        let mut battle = original_battle();
        battle.enemies[0].hp = 1;
        battle.resolve_strike_impact(0, 0, Strike::Hit { dmg: 5, crit });
        if crit {
            assert_eq!(battle.log[0], battle.text.enemy_critical);
        }
        assert_eq!(battle.enemies[0].hp, 0);
        assert_eq!(
            &battle.log[usize::from(crit)..],
            ["Bandita 5 HP-t sebződik", "Bandita összeesik!"]
        );
    }
}

#[test]
fn lethal_skill_damage_and_absorption_both_report_one_original_collapse() {
    for target in [Source::Party(0), Source::Enemy(0)] {
        for absorb in [false, true] {
            let mut battle = original_battle();
            let source = match target {
                Source::Party(i) => {
                    battle.members[i].hp = 1;
                    Source::Enemy(0)
                }
                Source::Enemy(i) => {
                    battle.enemies[i].hp = 1;
                    Source::Party(0)
                }
            };
            let mut skill = damage_skill(1, 5, vec![], vec![]);
            skill.absorb = absorb;
            skill.magical_rate = 0;
            skill.variance = 0;
            let lines = battle.skill_hit_battler(source, target, &skill);
            assert_eq!(lines.len(), 2);
            assert_eq!(
                lines[1],
                format!("{} összeesik!", battle.battler_name(target))
            );
            if absorb {
                assert_eq!(lines[0], format!("{} HP 1", battle.battler_name(target)));
            } else {
                assert!(lines[0].contains(" 5 HP-t "));
            }
        }
    }
}

#[test]
fn a_forced_death_state_retains_the_original_extra_message_wait() {
    let mut battle = original_battle();
    battle.enemies[0].state_ranks = vec![0];
    battle
        .states
        .iter_mut()
        .find(|state| state.id == 1)
        .unwrap()
        .rates[0] = 100;
    let mut skill = damage_skill(1, 0, vec![], vec![1]);
    skill.affect_hp = false;
    assert_eq!(
        battle.skill_hit_battler(Source::Party(0), Source::Enemy(0), &skill),
        ["Bandita összeesik!", "Bandita összeesik!"]
    );
    assert_eq!(battle.enemies[0].hp, 0);
}

#[test]
fn a_weapon_death_state_retains_the_original_extra_collapse_after_damage() {
    let mut battle = original_battle();
    battle.members[0].weapon_states = vec![(1, 100)];
    battle.enemies[0].state_ranks = vec![0];
    battle
        .states
        .iter_mut()
        .find(|state| state.id == 1)
        .unwrap()
        .rates[0] = 100;
    battle.resolve_strike_impact(
        0,
        0,
        Strike::Hit {
            dmg: 1,
            crit: false,
        },
    );
    assert_eq!(
        battle.log,
        [
            "Bandita 1 HP-t sebződik",
            "Bandita összeesik!",
            "Bandita összeesik!"
        ]
    );
    assert_eq!(battle.enemies[0].hp, 0);
}

#[test]
fn normal_double_and_confused_attacks_report_a_fallen_party_member_once() {
    for kind in [
        Command::Attack { target: 0 },
        Command::DoubleAttack { target: 0 },
    ] {
        let mut battle = original_battle();
        battle.members[0].hp = 1;
        wind_enemy_hits(&mut battle, &[0]);
        battle.apply(Action {
            source: Source::Enemy(0),
            kind,
            agility: 1,
        });
        let text = battle.log.join("\n");
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2], "Ron összeesik!");
    }
    let mut battle = original_battle();
    battle.members[0].hp = 1;
    battle.land_ally_strike(Source::Party(0), Source::Party(0), Some(5));
    assert_eq!(battle.log, ["Ron 5 HP-t veszít", "Ron összeesik!"]);
}

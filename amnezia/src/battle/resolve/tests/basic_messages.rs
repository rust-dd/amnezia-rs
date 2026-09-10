use super::*;

fn original_battle() -> Battle {
    let mut battle = build_1v2();
    messages::original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.enemies[0].name = "Bandita".into();
    battle
}

#[test]
fn original_enemy_basic_messages_distinguish_observing_from_an_absent_action() {
    for (basic, expected) in [
        (2, Some("Bandita védekezik")),
        (3, Some("Bandita csak figyel és várakozik")),
        (4, Some("Bandita erőt gyüjt")),
        (6, Some("Bandita elmenekül!")),
        (7, None),
    ] {
        let mut battle = original_battle();
        battle.apply(Action {
            source: Source::Enemy(0),
            kind: logic::enemy_command(Some(&enemy_action_def(basic)), 0),
            agility: 1,
        });
        assert_eq!(battle.log, expected.into_iter().collect::<Vec<_>>());
    }
    let mut battle = original_battle();
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Nothing,
        agility: 1,
    });
    assert!(battle.log.is_empty());
}

#[test]
fn original_normal_attack_announces_the_source_before_its_animation() {
    let mut battle = original_battle();
    battle.members[0].attack_animation = 3;
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 0;
    battle.members[0].base_critical_denominator = None;
    let hp = battle.enemies[0].hp;
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Attack { target: 0 },
        agility: 1,
    });
    assert_eq!(battle.log, ["Ron megtámadja az ellenséget"]);
    assert_eq!(battle.enemies[0].hp, hp);
    assert!(battle.anim_hold_active());
    battle.resolve_next();
    assert_eq!(
        battle.log,
        [
            "Ron megtámadja az ellenséget".to_string(),
            format!("Bandita {} HP-t sebződik", hp - battle.enemies[0].hp)
        ]
    );
}

#[test]
fn original_enemy_attacks_report_target_damage_without_invented_attack_summaries() {
    for kind in [
        Command::Attack { target: 0 },
        Command::DoubleAttack { target: 0 },
    ] {
        let mut battle = original_battle();
        wind_enemy_hits(&mut battle, &[0, 2]);
        let hp = battle.members[0].hp;
        battle.apply(Action {
            source: Source::Enemy(0),
            kind,
            agility: 1,
        });
        let lines = battle.log.join("\n");
        let lines = lines.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "Bandita megtámadja az ellenséget");
        let expected_hits = if matches!(kind, Command::DoubleAttack { .. }) {
            2
        } else {
            1
        };
        assert_eq!(lines.len(), expected_hits + 1);
        let dealt = lines[1..]
            .iter()
            .map(|line| {
                line.strip_prefix("Ron ")
                    .unwrap()
                    .strip_suffix(" HP-t veszít")
                    .unwrap()
                    .parse::<i32>()
                    .unwrap()
            })
            .sum::<i32>();
        assert_eq!(hp - battle.members[0].hp, dealt);
    }
}

#[test]
fn zero_damage_and_confused_strikes_use_the_original_target_terms() {
    let mut battle = original_battle();
    battle.resolve_strike_impact(
        0,
        0,
        Strike::Hit {
            dmg: 0,
            crit: false,
        },
    );
    assert_eq!(battle.log, ["Bandita kivédi a támadást"]);
    battle.log.clear();
    battle.land_ally_strike(Source::Party(0), Source::Party(0), Some(0));
    assert_eq!(battle.log, ["Ron félreugrik"]);
    battle.log.clear();
    battle.land_ally_strike(Source::Party(0), Source::Party(0), None);
    assert_eq!(battle.log, ["Ron kivédi a támadást"]);
}

#[test]
fn original_self_destruct_announces_then_reports_each_recipient() {
    let mut battle = build_party2();
    messages::original_text(&mut battle);
    battle.enemies[0].name = "Bandita".into();
    let hp = battle
        .members
        .iter()
        .map(|member| member.hp)
        .collect::<Vec<_>>();
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::SelfDestruct,
        agility: 1,
    });
    assert_eq!(battle.log[0], "Bandita előretör");
    for (index, member) in battle.members.iter().enumerate() {
        assert_eq!(
            battle.log[index + 1],
            format!("{} {} HP-t veszít", member.name, hp[index] - member.hp)
        );
    }
    assert_eq!(battle.enemies[0].hp, 0);
}

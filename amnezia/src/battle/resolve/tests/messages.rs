use super::*;

fn original_text(battle: &mut Battle) {
    let root = crate::assets::asset_root();
    battle
        .text
        .apply(&crate::assets::load_ron(&format!("{root}/terms.ron")));
    battle.states = crate::assets::load_ron(&format!("{root}/states.ron"));
    battle.log.clear();
}

#[test]
fn original_skill_messages_announce_usage_before_the_animation_and_only_once() {
    let mut battle = build_party2();
    original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.members[0].sp = 100;
    battle.members[0].max_sp = 100;
    battle.skills = crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()));
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Skill {
            skill_id: 2,
            target: 0,
        },
        agility: 1,
    });
    assert_eq!(battle.log, ["Ron ciklonként támad"]);
    assert_eq!(battle.members[0].sp, 65);
    assert!(battle.anim_hold_active());
    while battle.resolve_next() {}
    assert_eq!(
        battle
            .log
            .iter()
            .filter(|s| *s == "Ron ciklonként támad")
            .count(),
        1
    );
    assert!(!battle.log.iter().any(|s| s.contains("varázsol")));
}

#[test]
fn original_skill_messages_use_the_selected_failure_term_without_a_fallback() {
    let mut battle = build_1v2();
    original_text(&mut battle);
    battle.enemies[0].name = "Bandita".into();
    battle.members[0].stats.agility = 10;
    battle.enemies[0].stats.agility = 10;
    for (failure, expected) in [
        (0, "Bandita félreugrik"),
        (1, "Bandita védekezik"),
        (2, "Bandita kivédi a támadást"),
        (3, "Bandita kivédi a támadást"),
        (9, "Bandita"),
    ] {
        let mut skill = damage_skill(1, 20, vec![], vec![]);
        skill.hit = 0;
        skill.failure_message = failure;
        assert_eq!(
            battle.skill_hit_chance(Source::Party(0), Source::Enemy(0), &skill),
            0
        );
        assert_eq!(
            battle.skill_hit_battler(Source::Party(0), Source::Enemy(0), &skill),
            [expected]
        );
    }
}

#[test]
fn original_skill_messages_use_state_infliction_recovery_and_blank_already_suffixes() {
    let mut battle = build_1v2();
    original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.enemies[0].name = "Bandita".into();
    let mut skill = damage_skill(1, 0, vec![], vec![2]);
    for target in [Source::Party(0), Source::Enemy(0)] {
        for seed in 0..100 {
            battle.rng = seed;
            let effects = battle.skill_states(target, &skill, 100);
            if effects.success {
                assert_eq!(
                    effects.lines,
                    [match target {
                        Source::Party(_) => "Ron mérgezést kap",
                        Source::Enemy(_) => "Bandita megmérgeződik",
                    }]
                );
                break;
            }
        }
        assert!(logic::has_state(battle.battler_states(target), 2));
    }
    assert_eq!(
        battle.skill_states(Source::Party(0), &skill, 100).lines,
        ["Ron"]
    );
    skill.scope = 3;
    assert_eq!(
        battle.skill_states(Source::Party(0), &skill, 100).lines,
        ["Ron szervezetéből eltűnt a méreg"]
    );
}

#[test]
fn skill_usage_preserves_blank_first_suffixes_and_independent_second_lines() {
    let mut battle = build_1v2();
    original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.enemies[0].name = "Bandita".into();
    let mut skill = damage_skill(1, 20, vec![], vec![]);
    skill.using_message2 = "Második sor".into();
    battle.log_skill_use(Source::Party(0), &skill);
    skill.using_message1 = " támadásba lendül".into();
    battle.log_skill_use(Source::Enemy(0), &skill);
    assert_eq!(
        battle.log,
        [
            "Ron",
            "Második sor",
            "Bandita támadásba lendül",
            "Második sor"
        ]
    );
}

#[test]
fn original_recovery_messages_report_hp_and_sp_separately_with_clamped_values() {
    let mut battle = build_1v2();
    original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.members[0].hp = battle.members[0].max_hp - 3;
    battle.members[0].sp = battle.members[0].max_sp - 5;
    let mut skill = heal_skill(1, 40);
    skill.affect_sp = true;
    assert_eq!(
        battle.skill_heal_battler(Source::Party(0), Source::Party(0), &skill),
        ["Ron HP 3 visszatért", "Ron SP 5 visszatért"]
    );
    assert_eq!(
        battle.skill_heal_battler(Source::Party(0), Source::Party(0), &skill),
        ["Ron HP 0 visszatért"]
    );
}

#[test]
fn original_reviving_skill_reports_consciousness_without_an_extra_hp_message() {
    let mut battle = build_1v2();
    original_text(&mut battle);
    battle.members[0].name = "Ron".into();
    battle.members[0].hp = 0;
    battle.members[0].states = vec![(1, 0)];
    let mut skill = heal_skill(1, 40);
    skill.affected_states = vec![1];
    assert_eq!(
        battle.skill_heal_battler(Source::Party(0), Source::Party(0), &skill),
        ["Ron visszanyeri eszméletét!"]
    );
    assert!(battle.members[0].hp > 0);
    assert!(battle.members[0].states.is_empty());
}

#[test]
fn original_absorption_and_parameter_messages_keep_intentionally_empty_terms() {
    let mut battle = build_1v2();
    original_text(&mut battle);
    battle.enemies[0].name = "Bandita".into();
    battle.enemies[0].sp = 7;
    let mut skill = damage_skill(1, 40, vec![], vec![]);
    skill.affect_hp = false;
    skill.affect_sp = true;
    skill.absorb = true;
    assert_eq!(
        battle.skill_hit_battler(Source::Party(0), Source::Enemy(0), &skill),
        ["Bandita SP 7"]
    );
    assert_eq!(battle.enemies[0].sp, 0);
    assert_eq!(
        battle.skill_hit_battler(Source::Party(0), Source::Enemy(0), &skill),
        ["Bandita félreugrik"]
    );
    assert_eq!(
        battle.text.parameter_changed("Ron", &battle.text.attack, 8),
        "Ron Támadóerő 8 "
    );
    assert_eq!(
        battle
            .text
            .parameter_changed("Ron", &battle.text.defense, -8),
        "Ron Védőerő 8"
    );
    assert_eq!(battle.text.parameter_changed("Ron", "HP", 0), "");
    assert_eq!(battle.text.damaged("Ron", true, 0), "Ron félreugrik");
    assert_eq!(
        battle.text.damaged("Bandita", false, 0),
        "Bandita kivédi a támadást"
    );
}

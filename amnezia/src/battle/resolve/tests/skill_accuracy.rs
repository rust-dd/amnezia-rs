use super::*;

fn original_skills() -> Vec<SkillDef> {
    crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()))
}

#[test]
fn converted_skills_preserve_the_original_success_rates() {
    let skills = original_skills();
    assert_eq!(skills.iter().filter(|s| s.hit == 100).count(), 59);
    let zeroes = skills
        .iter()
        .filter(|s| s.hit == 0)
        .map(|s| s.id)
        .collect::<Vec<_>>();
    assert_eq!(zeroes, vec![67, 68, 69, 70]);
    assert_eq!(
        skills.iter().find(|s| s.id == 5).unwrap().failure_message,
        3
    );
    assert_eq!(
        skills.iter().find(|s| s.id == 13).unwrap().failure_message,
        2
    );
}

#[test]
fn physical_skill_accuracy_depends_on_the_failure_mode_not_damage_weights() {
    let mut skill = damage_skill(1, 30, vec![], vec![]);
    skill.hit = 80;
    skill.physical_rate = 10;
    assert_eq!(logic::skill_to_hit(&skill, 10, 40, true), 80);
    assert_eq!(logic::skill_to_hit(&skill, 10, 40, false), 80);
    skill.failure_message = 3;
    skill.physical_rate = 0;
    assert_eq!(logic::skill_to_hit(&skill, 10, 40, true), 50);
    assert_eq!(logic::skill_to_hit(&skill, 10, 40, false), 100);
    skill.scope = 3;
    assert_eq!(logic::skill_to_hit(&skill, 10, 40, true), 80);
}

#[test]
fn the_original_guaranteed_skill_hits_from_both_sides() {
    let mut battle = build_1v2();
    battle.skills = original_skills();
    let skill = battle.skills.iter().find(|s| s.id == 1).unwrap().clone();
    for seed in 1..=32 {
        battle.rng = seed;
        battle.enemies[0].hp = 1000;
        battle.skill_hit_enemy(0, 0, &skill);
        assert!(battle.enemies[0].hp < 1000);
        battle.rng = seed;
        battle.members[0].hp = 1000;
        battle.enemy_cast(0, 1, 0);
        assert!(battle.members[0].hp < 1000);
    }
}

#[test]
fn an_original_zero_chance_skill_misses_even_with_an_agility_advantage() {
    let mut battle = build_1v2();
    battle.skills = original_skills();
    let skill = battle.skills.iter().find(|s| s.id == 67).unwrap().clone();
    battle.members[0].stats.agility = 100;
    battle.enemies[0].stats.agility = 1;
    let hp = battle.enemies[0].hp;
    for seed in 1..=32 {
        battle.rng = seed;
        let lines = battle.skill_hit_enemy(0, 0, &skill);
        assert!(lines.iter().any(|line| line.contains("elkerülte")));
        assert_eq!(battle.enemies[0].hp, hp);
    }
    battle.members[0].stats.agility = 1;
    battle.enemies[0].stats.agility = 100;
    let hp = battle.members[0].hp;
    let line = battle.enemy_cast(0, 67, 0).unwrap();
    assert!(line.contains("elkerülte"));
    assert_eq!(battle.members[0].hp, hp);
}

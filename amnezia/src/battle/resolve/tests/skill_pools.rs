use super::*;

fn original_skills(battle: &mut Battle) {
    battle.skills = crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()));
}

#[test]
fn original_demonlord_drain_transfers_sp_without_damaging_hp() {
    let mut battle = build_1v2();
    original_skills(&mut battle);
    let skill = battle.skills.iter().find(|s| s.id == 65).unwrap();
    assert!(!skill.affect_hp && skill.affect_sp && skill.absorb);
    assert_eq!((skill.power, skill.hit, skill.sp_cost), (50, 85, 0));
    battle.rng = 0;
    battle.enemies[0].sp = 0;
    let hp = battle.members[0].hp;
    let sp = battle.members[0].sp;
    let line = battle.enemy_cast(0, 65, 0).unwrap();
    assert_eq!(battle.members[0].hp, hp);
    assert_eq!(battle.members[0].sp, 0);
    assert_eq!(battle.enemies[0].sp, sp);
    assert!(line.contains("SP"));
}

#[test]
fn original_phoenix_attacks_both_hp_and_sp_and_pays_its_cost() {
    let mut battle = build_1v2();
    original_skills(&mut battle);
    let skill = battle.skills.iter().find(|s| s.id == 6).unwrap();
    assert_eq!((skill.scope, skill.power, skill.sp_cost), (0, 999, 300));
    assert!(skill.affect_hp && skill.affect_sp && !skill.absorb);
    battle.members[0].sp = 350;
    battle.members[0].max_sp = 350;
    let enemy = &mut battle.enemies[0];
    enemy.hp = 2000;
    enemy.max_hp = 2000;
    enemy.sp = 2000;
    enemy.max_sp = 2000;
    battle.cast_skill(0, 6, 0).unwrap();
    assert_eq!(battle.enemies[0].hp, 1001);
    assert_eq!(battle.enemies[0].sp, 1001);
    assert_eq!(battle.members[0].sp, 50);
}

#[test]
fn both_sides_skip_unaffordable_skills_before_playing_the_animation() {
    for source in [Source::Party(0), Source::Enemy(0)] {
        let mut battle = build_1v2();
        let mut skill = damage_skill(1, 10, vec![], vec![]);
        skill.animation_id = 4;
        battle.skills = vec![skill];
        battle.members[0].sp = 2;
        battle.enemies[0].sp = 2;
        let hp = (battle.members[0].hp, battle.enemies[0].hp);
        battle.apply(Action {
            source,
            kind: Command::Skill {
                skill_id: 1,
                target: 0,
            },
            agility: 1,
        });
        assert!(battle.pending_anims.is_empty());
        assert!(!battle.action_in_progress());
        assert_eq!((battle.members[0].hp, battle.enemies[0].hp), hp);
        assert_eq!((battle.members[0].sp, battle.enemies[0].sp), (2, 2));
    }
}

#[test]
fn animated_casts_pay_once_before_the_effect_and_misses_still_cost_sp() {
    for source in [Source::Party(0), Source::Enemy(0)] {
        let mut battle = build_1v2();
        let mut skill = damage_skill(1, 10, vec![], vec![]);
        skill.animation_id = 4;
        skill.hit = 0;
        battle.skills = vec![skill];
        battle.members[0].sp = 6;
        battle.enemies[0].sp = 6;
        let hp = (battle.members[0].hp, battle.enemies[0].hp);
        battle.apply(Action {
            source,
            kind: Command::Skill {
                skill_id: 1,
                target: 0,
            },
            agility: 1,
        });
        let paid = (battle.members[0].sp, battle.enemies[0].sp);
        assert_eq!(
            paid,
            match source {
                Source::Party(_) => (3, 6),
                Source::Enemy(_) => (6, 3),
            }
        );
        assert_eq!(battle.pending_anims.len(), 1);
        battle.resolve_next();
        assert_eq!((battle.members[0].sp, battle.enemies[0].sp), paid);
        assert_eq!((battle.members[0].hp, battle.enemies[0].hp), hp);
    }
}

#[test]
fn sp_damage_is_not_halved_by_defending_and_absorption_is_capped_on_both_sides() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 50, vec![], vec![]);
    skill.variance = 0;
    skill.magical_rate = 0;
    skill.affect_hp = false;
    skill.affect_sp = true;
    skill.absorb = true;
    battle.members[0].sp = 36;
    battle.enemies[0].sp = 23;
    battle.enemies[0].defending = true;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].sp, 0);
    assert_eq!(battle.members[0].sp, 37);
    assert_eq!(battle.enemies[0].hp, 30);
}

#[test]
fn hp_absorption_cannot_steal_more_health_than_the_target_has() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 100, vec![], vec![]);
    skill.variance = 0;
    skill.magical_rate = 0;
    skill.absorb = true;
    battle.members[0].hp = 10;
    battle.enemies[0].hp = 7;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].hp, 0);
    assert_eq!(battle.members[0].hp, 17);
}

#[test]
fn enemy_self_recovery_changes_only_the_requested_pools_after_paying() {
    let mut battle = build_1v2();
    let mut skill = heal_skill(1, 20);
    skill.scope = 2;
    skill.variance = 0;
    skill.magical_rate = 0;
    skill.affect_hp = false;
    skill.affect_sp = true;
    battle.skills = vec![skill];
    battle.enemies[0].sp = 5;
    battle.enemies[0].max_sp = 15;
    battle.enemies[0].hp = 7;
    battle.enemy_cast(0, 1, 0).unwrap();
    assert_eq!(battle.enemies[0].sp, 15);
    assert_eq!(battle.enemies[0].hp, 7);
}

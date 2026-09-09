use super::*;
use crate::battle::model::Fighter;

fn original_actor(battle: &mut Battle, id: u32, slots: [u32; 5]) {
    let root = crate::assets::asset_root();
    let actors =
        crate::assets::load_ron::<Vec<amnezia_data::ActorDef>>(&format!("{root}/actors.ron"));
    battle.items = crate::assets::load_ron(&format!("{root}/items.ron"));
    battle.skills = crate::assets::load_ron(&format!("{root}/skills.ron"));
    battle.states = crate::assets::load_ron(&format!("{root}/states.ron"));
    let actor = actors.iter().find(|actor| actor.id == id).unwrap();
    battle.members[0] = Fighter::build(
        actor,
        slots,
        &battle.items,
        &crate::vitals::Vitals::default(),
        &crate::progression::Progression::default(),
    );
}

#[test]
fn original_state_tables_and_actor_ranks_determine_the_live_probability() {
    let mut battle = build_1v2();
    original_actor(&mut battle, 1, [0; 5]);
    assert_eq!(battle.battler_state_probability(Source::Party(0), 2), 50);
    assert_eq!(battle.battler_state_probability(Source::Party(0), 3), 30);
    assert_eq!(battle.battler_state_probability(Source::Party(0), 5), 20);
    assert_eq!(battle.battler_state_probability(Source::Enemy(0), 2), 70);
    assert_eq!(battle.battler_state_probability(Source::Enemy(0), 1), 60);
    original_actor(&mut battle, 5, [0; 5]);
    assert_eq!(battle.battler_state_probability(Source::Party(0), 4), 0);
    assert_eq!(battle.battler_state_probability(Source::Party(0), 2), 70);
    original_actor(&mut battle, 10, [0; 5]);
    for id in 1..=10 {
        assert_eq!(battle.battler_state_probability(Source::Party(0), id), 0);
    }
}

#[test]
fn only_the_strongest_armor_state_guard_applies_and_weapons_do_not_guard() {
    let mut battle = build_1v2();
    for slots in [[0, 58, 0, 0, 0], [0, 58, 77, 99, 0]] {
        original_actor(&mut battle, 1, slots);
        assert_eq!(battle.battler_state_probability(Source::Party(0), 1), 10);
    }
    original_actor(&mut battle, 1, [13, 0, 0, 0, 0]);
    assert_eq!(battle.battler_state_probability(Source::Party(0), 8), 40);
    original_actor(&mut battle, 1, [0, 59, 0, 0, 0]);
    for id in 2..=10 {
        assert_eq!(battle.battler_state_probability(Source::Party(0), id), 0);
    }
    assert_eq!(battle.battler_state_probability(Source::Party(0), 1), 40);
}

#[test]
fn original_enemy_poison_and_curse_apply_states_and_respect_immunity() {
    let mut battle = build_1v2();
    original_actor(&mut battle, 1, [0; 5]);
    for id in [50, 64] {
        let hp = battle.members[0].hp;
        for seed in 0..100 {
            battle.rng = seed;
            battle.enemies[0].sp = 999;
            battle.enemy_cast(0, id, 0).unwrap();
        }
        assert_eq!(battle.members[0].hp, hp);
    }
    assert!(logic::has_state(&battle.members[0].states, 2));
    assert!(logic::has_state(&battle.members[0].states, 3));
    original_actor(&mut battle, 1, [0, 59, 0, 0, 0]);
    for seed in 0..100 {
        battle.rng = seed;
        battle.enemies[0].sp = 999;
        battle.enemy_cast(0, 50, 0).unwrap();
        battle.enemy_cast(0, 64, 0).unwrap();
    }
    assert!(battle.members[0].states.is_empty());
}

#[test]
fn hp_sp_and_state_effects_use_independent_hit_checks() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 10, vec![], vec![2]);
    skill.hit = 50;
    skill.variance = 0;
    skill.magical_rate = 0;
    skill.affect_sp = true;
    battle.states = vec![poison_state(2)];
    battle.enemies[0].state_ranks = vec![0, 0];
    let mut hp_only = false;
    let mut sp_only = false;
    let mut state_miss = false;
    for seed in 0..200 {
        battle.rng = seed;
        battle.enemies[0].hp = 100;
        battle.enemies[0].sp = 100;
        battle.enemies[0].states.clear();
        battle.skill_hit_enemy(0, 0, &skill);
        let hp_hit = battle.enemies[0].hp < 100;
        let sp_hit = battle.enemies[0].sp < 100;
        hp_only |= hp_hit && !sp_hit;
        sp_only |= !hp_hit && sp_hit;
        state_miss |= hp_hit && sp_hit && battle.enemies[0].states.is_empty();
        if !hp_hit && !sp_hit {
            assert!(battle.enemies[0].states.is_empty());
        }
    }
    assert!(hp_only && sp_only && state_miss);
}

#[test]
fn a_lethal_hp_effect_does_not_also_drain_sp_or_apply_states() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 500, vec![], vec![2]);
    skill.variance = 0;
    skill.affect_sp = true;
    battle.states = vec![poison_state(2)];
    battle.enemies[0].state_ranks = vec![0, 0];
    let sp = battle.enemies[0].sp;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].hp, 0);
    assert_eq!(battle.enemies[0].sp, sp);
    assert_eq!(battle.enemies[0].states, [(1, 0)]);
}

#[test]
fn an_already_present_state_succeeds_without_restarting_its_duration() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 0, vec![], vec![2]);
    skill.affect_hp = false;
    skill.hit = 0;
    battle.enemies[0].states = vec![(2, 7)];
    let lines = battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].states, [(2, 7)]);
    assert!(!lines.iter().any(|line| line.contains("elkerülte")));
}

#[test]
fn healing_states_rolls_success_but_does_not_roll_resistance_on_either_side() {
    let mut battle = build_1v2();
    let mut skill = heal_skill(1, 0);
    skill.affect_hp = false;
    skill.affected_states = vec![2];
    for target in [Source::Party(0), Source::Enemy(0)] {
        *battle.battler_states_mut(target) = vec![(2, 3)];
        skill.hit = 0;
        battle.skill_heal_battler(target, target, &skill);
        assert!(logic::has_state(battle.battler_states(target), 2));
        skill.hit = 100;
        battle.skill_heal_battler(target, target, &skill);
        assert!(battle.battler_states(target).is_empty());
    }
}

#[test]
fn revival_requires_a_successful_death_cure_even_if_hp_recovery_was_requested() {
    let mut battle = build_1v2();
    let mut skill = heal_skill(1, 50);
    skill.affected_states = vec![1];
    skill.hit = 0;
    battle.members[0].hp = 0;
    battle.members[0].states = vec![(1, 0)];
    battle.skill_heal_battler(Source::Enemy(0), Source::Party(0), &skill);
    assert_eq!(battle.members[0].hp, 0);
    assert!(logic::has_state(&battle.members[0].states, 1));
}

#[test]
fn ordinary_skills_never_receive_weapon_critical_damage() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 10, vec![], vec![]);
    skill.magical_rate = 0;
    skill.variance = 0;
    battle.members[0].weapon_crit = 100;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].hp, 20);
}

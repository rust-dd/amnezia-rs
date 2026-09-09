use super::*;

#[test]
fn the_original_life_potion_revives_its_recipient_with_half_hp() {
    let mut battle = build_party2();
    battle.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    battle.members[0].hp = 7;
    battle.members[1].hp = 0;
    battle.members[1].states = vec![(1, 0)];
    battle.apply_item(0, 112, 1);
    assert_eq!(battle.members[0].hp, 7);
    assert_eq!(battle.members[1].hp, battle.members[1].max_hp / 2);
    assert!(battle.members[1].states.is_empty());
}

#[test]
fn the_original_life_potion_has_no_effect_on_a_living_recipient() {
    let mut battle = build_party2();
    battle.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    battle.members[1].hp = 7;
    battle.apply_item(0, 112, 1);
    assert_eq!(battle.members[1].hp, 7);
    assert!(battle.pending_numbers.is_empty());
}

#[test]
fn ordinary_medicine_does_not_revive_or_redirect_to_the_user() {
    let mut battle = build_party2();
    battle.items = vec![medicine(1, 50, 0, vec![])];
    battle.members[0].hp = 7;
    battle.members[1].hp = 0;
    battle.apply_item(0, 1, 1);
    assert_eq!(battle.members[0].hp, 7);
    assert_eq!(battle.members[1].hp, 0);
}

#[test]
fn party_medicine_restores_every_member_including_revivable_allies() {
    let mut battle = build_party2();
    let mut item = medicine(1, 20, 0, vec![1]);
    item.scope = 1;
    battle.items = vec![item];
    battle.members[0].hp = 1;
    battle.members[1].hp = 0;
    battle.apply_item(0, 1, 0);
    assert_eq!(battle.members[0].hp, 21);
    assert_eq!(battle.members[1].hp, 20);
}

#[test]
fn revival_without_hp_effect_uses_percentage_power_and_animates_the_fallen_ally() {
    let mut battle = build_party2();
    let mut skill = heal_skill(1, 50);
    skill.affect_hp = false;
    skill.magical_rate = 0;
    skill.variance = 0;
    skill.affected_states = vec![1];
    battle.members[1].hp = 0;
    battle.members[1].states = vec![(1, 0)];
    assert_eq!(battle.skill_anim_anchors(0, &skill, 1).len(), 1);
    battle.skills = vec![skill];
    battle.cast_skill(0, 1, 1);
    assert_eq!(battle.members[1].hp, battle.members[1].max_hp / 2);
    assert!(battle.members[1].states.is_empty());
}

#[test]
fn original_party_revival_includes_fallen_targets_in_deferred_steps() {
    let mut battle = build_party2();
    battle.skills = crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()));
    let cost = battle.skills.iter().find(|s| s.id == 35).unwrap().sp_cost as i32;
    battle.members[0].sp = cost;
    battle.members[0].max_sp = cost;
    battle.members[0].hp = 7;
    battle.members[1].hp = 0;
    battle.members[1].states = vec![(1, 0)];
    battle.cast_skill(0, 35, 0);
    assert_eq!(battle.members[0].hp, 7);
    assert_eq!(battle.members[1].hp, 0);
    assert!(battle.resolve_next());
    assert_eq!(battle.members[1].hp, battle.members[1].max_hp);
    assert!(battle.members[1].states.is_empty());
}

#[test]
fn a_cure_only_skill_does_not_restore_hp_to_a_living_ally() {
    let mut battle = build_party2();
    let mut skill = heal_skill(1, 50);
    skill.affect_hp = false;
    skill.affected_states = vec![3];
    battle.members[1].hp = 7;
    battle.members[1].states = vec![(3, 0)];
    battle.skill_heal_ally(0, 1, &skill);
    assert_eq!(battle.members[1].hp, 7);
    assert!(battle.members[1].states.is_empty());
}

#[test]
fn a_status_only_attack_does_not_deal_unconfigured_hp_damage() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 100, vec![], vec![3]);
    skill.affect_hp = false;
    battle.enemies[0].state_ranks = vec![0; 3];
    let hp = battle.enemies[0].hp;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].hp, hp);
    assert!(logic::has_state(&battle.enemies[0].states, 3));
    assert!(battle.pending_numbers.is_empty());
}

#[test]
fn an_instant_death_skill_fells_the_target_and_starts_its_death_effect() {
    let mut battle = build_1v2();
    let mut skill = damage_skill(1, 0, vec![], vec![1]);
    skill.affect_hp = false;
    battle.enemies[0].state_ranks = vec![0];
    battle.skill_hit_enemy(0, 0, &skill);
    assert!(!battle.enemies[0].alive());
    assert!(battle.enemies[0].dying.is_some());
    assert!(battle.pending_se.contains(&BattleSe::EnemyDefeated));
}

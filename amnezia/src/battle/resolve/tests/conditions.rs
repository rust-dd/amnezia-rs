use super::*;

#[test]
fn poison_drains_a_fighter_and_a_foe_by_val_plus_max_percent_at_turn_start() {
    let mut battle = build_1v2();
    // Méreg (state 2): type 0 lose, 5% of max HP + 1 flat, each turn.
    battle.states = vec![hp_change_state(2, 0, 5, 1)];
    battle.members[0].states = vec![(2, 0)];
    battle.enemies[0].states = vec![(2, 0)];
    let (m_max, f_max) = (battle.members[0].max_hp, battle.enemies[0].max_hp);
    let (m_before, f_before) = (battle.members[0].hp, battle.enemies[0].hp);
    let log = battle.log.clone();
    battle.tick_state_hp(Source::Party(0));
    battle.tick_state_hp(Source::Enemy(0));
    assert_eq!(m_before - battle.members[0].hp, 1 + m_max * 5 / 100);
    assert_eq!(f_before - battle.enemies[0].hp, 1 + f_max * 5 / 100);
    assert_eq!(battle.log, log);
}

#[test]
fn a_type_2_hp_change_state_drains_nothing() {
    let mut battle = build_1v2();
    // Type 2 = nothing, even with a large configured amount.
    battle.states = vec![hp_change_state(2, 2, 50, 10)];
    battle.members[0].states = vec![(2, 0)];
    let before = battle.members[0].hp;
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(battle.members[0].hp, before);
}

#[test]
fn poison_leaves_at_least_one_hp() {
    let mut battle = build_1v2();
    battle.states = vec![hp_change_state(2, 0, 100, 0)];
    battle.members[0].states = vec![(2, 0)];
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(battle.members[0].hp, 1);
    assert!(battle.members[0].alive());
}

#[test]
fn a_type_1_hp_change_state_regenerates_capped_at_max() {
    let mut battle = build_1v2();
    // Gain 5 HP plus 10% of maximum.
    battle.states = vec![hp_change_state(2, 1, 10, 5)];
    let max = battle.members[0].max_hp;
    battle.members[0].hp = 10;
    battle.members[0].states = vec![(2, 0)];
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(battle.members[0].hp, (10 + 5 + max * 10 / 100).min(max));
}

#[test]
fn a_damage_skill_inflicts_its_state_on_a_forced_hit_roll() {
    let mut battle = build_1v2();
    battle.states = vec![poison_state(3)];
    // Keep the target alive for status application; rank A guarantees state 3.
    battle.enemies[0].hp = 200;
    battle.enemies[0].state_ranks = vec![2, 2, 0];
    battle.skills = vec![damage_skill(1, 20, vec![], vec![3])];
    battle.cast_skill(0, 1, 0);
    assert!(logic::has_state(&battle.enemies[0].states, 3));
}

#[test]
fn being_hit_wears_off_a_damage_release_state_but_never_death() {
    let mut battle = build_1v2();
    // id 1 is the death state (exempt); id 2 shakes off on any hit.
    battle.states = vec![poison_state(1), damage_release_state(2)];
    battle.members[0].states = vec![(1, 0), (2, 0)];
    battle.hit_member(0, 8, 4, 100);
    assert!(logic::has_state(&battle.members[0].states, 1));
    assert!(!logic::has_state(&battle.members[0].states, 2));
}

#[test]
fn a_confused_member_turns_on_a_living_ally() {
    use crate::battle::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let ron = testkit::actor(1, 5, 80, 40);
    let tiff = testkit::actor(2, 5, 80, 40);
    let actors = vec![&ron, &tiff];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
    let mut battle = Battle::build(
        &troop,
        &monsters,
        &actors,
        &[testkit::slots(&ron), testkit::slots(&tiff)],
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        3,
    );
    battle.states = vec![confusion_state(9)];
    // State 9 forces the first member's confused action.
    battle.members[0].states = vec![(9, 0)];
    battle.members[0].weapon_hit = 100;
    battle.enemies[0].actions.clear();
    // The command flow auto-orders the confused member to strike an ally.
    battle.skip_restricted_choosers();
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Attack { .. })
    ));
    let hp = battle.members.iter().map(|member| member.hp).sum::<i32>();
    battle.commit(Command::Defend);
    while battle.resolve_next() {}
    assert!(battle.members.iter().map(|member| member.hp).sum::<i32>() < hp);
}

#[test]
fn a_cure_states_item_lifts_that_state_from_the_user() {
    let mut battle = build_1v2();
    battle.states = vec![poison_state(3)];
    battle.members[0].states = vec![(3, 0)];
    battle.items = vec![medicine(60, 0, 0, vec![3])];
    let line = battle.apply_item(0, 60, 0);
    assert!(!logic::has_state(&battle.members[0].states, 3));
    assert_eq!(
        line,
        format!(
            "{} Gyógyfű használata\n{}",
            battle.members[0].name, battle.members[0].name
        )
    );
}

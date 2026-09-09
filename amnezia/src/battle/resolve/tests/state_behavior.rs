use super::*;

fn original_states() -> Vec<amnezia_data::StateDef> {
    crate::assets::load_ron(&format!("{}/states.ron", crate::assets::asset_root()))
}

fn queue(battle: &mut Battle, source: Source, kind: Command) {
    battle.queue = vec![Action {
        source,
        kind,
        agility: 1,
    }];
    battle.queue_at = 0;
    battle.steps.clear();
}

#[test]
fn original_berserk_halves_defense_and_blindness_uses_the_lower_hit_ratio() {
    let mut battle = build_1v2();
    battle.states = original_states();
    battle.members[0].stats.defense = 101;
    battle.members[0].states = vec![(5, 0)];
    assert_eq!(battle.battler_stats(Source::Party(0)).defense, 50);
    assert_eq!(
        logic::state_hit_ratio(&battle.members[0].states, &battle.states),
        50
    );
    battle.members[0].states.push((3, 0));
    assert_eq!(battle.state_restriction(Source::Party(0)), 1);
    assert_eq!(
        logic::state_hit_ratio(&battle.members[0].states, &battle.states),
        20
    );
    let mut skill = damage_skill(1, 1, vec![], vec![]);
    skill.failure_message = 3;
    battle.members[0].stats.agility = 100;
    battle.enemies[0].stats.agility = 100;
    assert_eq!(
        battle.skill_hit_chance(Source::Party(0), Source::Enemy(0), &skill),
        20
    );
    skill.failure_message = 0;
    assert_eq!(
        battle.skill_hit_chance(Source::Party(0), Source::Enemy(0), &skill),
        100
    );
    skill.failure_message = 3;
    battle.enemies[0].states = vec![(7, 0)];
    assert_eq!(
        battle.skill_hit_chance(Source::Party(0), Source::Enemy(0), &skill),
        100
    );
}

#[test]
fn state_stat_scaling_happens_after_buffs_and_does_not_stack() {
    let mut battle = build_1v2();
    let mut half = poison_state(2);
    half.affect_stats = [true; 4];
    let mut other = half.clone();
    other.id = 3;
    battle.states = vec![half, other];
    battle.enemies[0].stats.defense = 101;
    battle.enemies[0].stat_modifiers[1] = 101;
    battle.enemies[0].states = vec![(2, 0), (3, 0)];
    assert_eq!(battle.battler_stats(Source::Enemy(0)).defense, 101);
    battle.states[1].affect_type = 1;
    assert_eq!(battle.battler_stats(Source::Enemy(0)).defense, 202);
    battle.enemies[0].states.remove(0);
    battle.enemies[0].stats.defense = 9999;
    assert_eq!(battle.battler_stats(Source::Enemy(0)).defense, 19998);
}

#[test]
fn skill_restrictions_use_inclusive_physical_and_magical_thresholds() {
    let mut battle = build_1v2();
    let mut silence = original_states().remove(3);
    silence.restriction = 0;
    battle.states = vec![silence];
    let mut skill = damage_skill(1, 10, vec![], vec![]);
    for target in [Source::Party(0), Source::Enemy(0)] {
        *battle.battler_states_mut(target) = vec![(4, 0)];
        skill.magical_rate = 1;
        assert!(!battle.skill_usable_by(target, &skill));
        skill.magical_rate = 0;
        assert!(battle.skill_usable_by(target, &skill));
    }
    battle.states[0].restrict_magic = false;
    battle.states[0].restrict_skill = true;
    battle.states[0].restrict_skill_level = 4;
    skill.physical_rate = 3;
    assert!(battle.skill_usable_by(Source::Party(0), &skill));
    skill.physical_rate = 4;
    assert!(!battle.skill_usable_by(Source::Party(0), &skill));
}

#[test]
fn original_priorities_suppress_shock_under_poison_and_clear_all_states_on_ko() {
    let mut battle = build_1v2();
    battle.states = original_states();
    let target = Source::Party(0);
    assert!(battle.inflict_battler_state(target, 9));
    assert!(battle.inflict_battler_state(target, 2));
    assert_eq!(battle.members[0].states, [(2, 0)]);
    assert!(!battle.inflict_battler_state(target, 9));
    assert!(battle.inflict_battler_state(target, 5));
    assert_eq!(battle.members[0].states, [(2, 0), (5, 0)]);
    battle.hit_member(0, 9999, 0, 100);
    assert_eq!(battle.members[0].states, [(1, 0)]);
    assert_eq!(battle.members[0].hp, 0);
}

#[test]
fn a_late_restriction_cancels_an_item_even_if_cured_before_the_action() {
    let mut battle = build_1v2();
    battle.states = original_states();
    queue(
        &mut battle,
        Source::Party(0),
        Command::Item {
            item_id: 1,
            target: 0,
        },
    );
    battle.inflict_battler_state(Source::Party(0), 8);
    battle.cure_battler_state(Source::Party(0), 8);
    let mut consumed = false;
    battle.resolve_next_with_items(|_| {
        consumed = true;
        true
    });
    assert!(!consumed);
    assert!(matches!(battle.queue[0].kind, Command::Nothing));
}

#[test]
fn silence_cancels_a_queued_enemy_skill_and_its_action_switch() {
    let mut battle = build_1v2();
    let mut silence = original_states().remove(3);
    silence.restriction = 0;
    battle.states = vec![silence];
    battle.skills = vec![damage_skill(1, 20, vec![], vec![])];
    queue(
        &mut battle,
        Source::Enemy(0),
        Command::Skill {
            skill_id: 1,
            target: 0,
        },
    );
    battle.enemies[0].switch_on_after_action = Some(545);
    let sp = battle.enemies[0].sp;
    let hp = battle.members[0].hp;
    battle.inflict_battler_state(Source::Enemy(0), 4);
    battle.cure_battler_state(Source::Enemy(0), 4);
    battle.resolve_next();
    assert_eq!((battle.enemies[0].sp, battle.members[0].hp), (sp, hp));
    assert!(battle.pending_switches.is_empty());
}

#[test]
fn recovery_runs_only_before_its_owner_action_and_respects_full_hold_turns() {
    let mut battle = build_1v2();
    let mut state = poison_state(2);
    state.hold_turn = 1;
    state.auto_release_prob = 100;
    battle.states = vec![state];
    battle.members[0].states = vec![(2, 0)];
    battle.enemies[0].states = vec![(2, 0)];
    battle.new_round();
    assert_eq!(battle.members[0].states, [(2, 0)]);
    queue(&mut battle, Source::Party(0), Command::Nothing);
    battle.resolve_next();
    assert_eq!(battle.members[0].states, [(2, 1)]);
    assert_eq!(battle.enemies[0].states, [(2, 0)]);
    queue(&mut battle, Source::Party(0), Command::Nothing);
    battle.resolve_next();
    assert!(battle.members[0].states.is_empty());
    assert_eq!(battle.enemies[0].states, [(2, 0)]);
}

#[test]
fn recovering_mindblow_is_removed_before_its_sp_effect() {
    let mut battle = build_1v2();
    battle.states = original_states();
    battle.members[0].states = vec![(10, 0)];
    let sp = battle.members[0].sp;
    queue(&mut battle, Source::Party(0), Command::Nothing);
    battle.resolve_next();
    assert!(battle.members[0].states.is_empty());
    assert_eq!(battle.members[0].sp, sp);
    battle.members[0].states = vec![(10, 0)];
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(
        battle.members[0].sp,
        sp - (battle.members[0].max_sp * 2 / 100 + 1)
    );
}

#[test]
fn a_start_of_action_sp_drain_prevents_an_unaffordable_cast() {
    let mut battle = build_1v2();
    let mut state = poison_state(2);
    state.sp_change_val = 2;
    battle.states = vec![state];
    battle.skills = vec![damage_skill(1, 20, vec![], vec![])];
    battle.enemies[0].states = vec![(2, 0)];
    battle.enemies[0].sp = 3;
    battle.enemies[0].switch_on_after_action = Some(545);
    let hp = battle.members[0].hp;
    queue(
        &mut battle,
        Source::Enemy(0),
        Command::Skill {
            skill_id: 1,
            target: 0,
        },
    );
    battle.resolve_next();
    assert_eq!((battle.enemies[0].sp, battle.members[0].hp), (1, hp));
    assert!(battle.pending_switches.is_empty());
}

#[test]
fn only_non_absorbing_physical_hp_effects_release_states_even_at_zero_damage() {
    let mut battle = build_1v2();
    battle.states = vec![damage_release_state(2)];
    let mut skill = damage_skill(1, 0, vec![], vec![]);
    skill.magical_rate = 0;
    skill.variance = 0;
    for (physical, absorb, released) in [(0, false, false), (10, true, false), (10, false, true)] {
        battle.enemies[0].states = vec![(2, 0)];
        battle.enemies[0].stats.defense = 9999;
        skill.physical_rate = physical;
        skill.absorb = absorb;
        battle.skill_hit_enemy(0, 0, &skill);
        assert_eq!(!logic::has_state(&battle.enemies[0].states, 2), released);
    }
    let defs = [damage_release_state(2)];
    let mut active = vec![(2, 0)];
    assert!(logic::release_on_damage(&mut active, &defs, 50, || 50).is_empty());
    assert_eq!(logic::release_on_damage(&mut active, &defs, 50, || 49), [2]);
}

#[test]
fn an_alone_confused_battler_can_target_itself_on_either_side() {
    let mut battle = build_1v2();
    battle.states = vec![confusion_state(6)];
    battle.enemies.truncate(1);
    battle.members[0].states = vec![(6, 0)];
    battle.begin_actor_commands();
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Attack { target: 0 })
    ));
    assert_eq!(battle.retarget_ally(0, 0), Some(0));
    battle.enemies[0].states = vec![(6, 0)];
    assert!(matches!(
        battle.enemy_action(0, &[true]).unwrap().kind,
        Command::Attack { target: 0 }
    ));
    assert_eq!(battle.retarget_other_enemy(0, 0), Some(0));
}

#[test]
fn a_confused_weapon_strike_uses_accuracy_elements_and_deferred_animation_without_critical() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 100;
    battle.members[0].base_critical_denominator = Some(1);
    battle.members[0].attack_animation = 5;
    battle.members[0].max_hp = 999;
    battle.members[0].hp = 999;
    battle.members[0].stats.attack = 200;
    battle.members[0].stats.defense = 4;
    let source = Source::Party(0);
    battle.confused_attack(source, source);
    assert_eq!(battle.members[0].hp, 999);
    assert_eq!(
        battle.pending_anims[0].targets,
        [battle.battler_pos(source)]
    );
    battle.resolve_next();
    assert!((80..=119).contains(&(999 - battle.members[0].hp)));
    battle.members[0].weapon_hit = 0;
    let hp = battle.members[0].hp;
    battle.confused_attack(source, source);
    battle.resolve_next();
    assert_eq!(battle.members[0].hp, hp);
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_attributes = vec![5];
    battle.members[0].attribute_ranks = vec![2, 2, 2, 2, 4];
    battle.attributes = vec![fire_attr()];
    battle.confused_attack(source, source);
    battle.resolve_next();
    assert_eq!(battle.members[0].hp, hp);
}

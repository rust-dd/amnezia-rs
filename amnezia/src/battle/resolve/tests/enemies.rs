use super::*;

#[test]
fn an_elemental_strike_amplifies_against_a_weak_foe() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 0;
    battle.members[0].weapon_attributes = vec![5];
    battle.attributes = vec![fire_attr()];
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 0];
    let base = logic::physical_damage(
        battle.members[0].stats.attack,
        battle.enemies[0].stats.defense,
    );
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    assert!(dmg > base);
}

#[test]
fn an_elemental_strike_is_nullified_by_an_immune_foe() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_attributes = vec![5];
    battle.attributes = vec![fire_attr()];
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 4];
    let before = battle.enemies[0].hp;
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    assert_eq!(dmg, 0);
    assert_eq!(battle.enemies[0].hp, before);
}

#[test]
fn killing_every_enemy_yields_victory_and_summed_reward() {
    let mut battle = build_1v2();
    for e in &mut battle.enemies {
        e.hp = 0;
    }
    assert!(matches!(battle.end_state(), Some(BattleOutcome::Victory)));
    assert_eq!(battle.victory_rewards(), (20, 60));
}

#[test]
fn retargeting_skips_a_dead_enemy() {
    let mut battle = build_1v2();
    battle.enemies[0].hp = 0;
    assert_eq!(battle.retarget_enemy(0), Some(1));
}

#[test]
fn a_fire_skill_amplifies_against_a_weak_foe() {
    let mut battle = build_1v2();
    battle.enemies[0].hp = 200;
    battle.enemies[0].max_hp = 200;
    battle.attributes = vec![fire_attr()];
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 0];
    battle.skills = vec![damage_skill(1, 50, vec![5], vec![])];
    let plain = logic::skill_effect(
        &battle.skills[0],
        &battle.members[0].stats,
        &battle.enemies[0].stats,
        true,
    );
    let before = battle.enemies[0].hp;
    battle.commit(Command::Skill {
        skill_id: 1,
        target: 0,
    });
    while battle.resolve_next() {}
    let dealt = before - battle.enemies[0].hp;
    assert!(
        dealt > plain,
        "fire against a weak (A) foe should beat the plain base ({dealt} <= {plain})"
    );
}

#[test]
fn an_enemy_skill_cast_wounds_the_targeted_member() {
    let mut battle = build_1v2();
    battle.skills = vec![damage_skill(1, 30, vec![], vec![])];
    let before = battle.members[0].hp;
    let line = battle.enemy_cast(0, 1, 0).unwrap();
    assert!(battle.members[0].hp < before);
    assert_eq!(
        line,
        format!(
            "{} {} HP-t veszít",
            battle.members[0].name,
            before - battle.members[0].hp
        )
    );
}

#[test]
fn an_enemy_ally_scope_skill_heals_the_caster_clamped_to_max() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)];
    battle.enemies[0].hp = battle.enemies[0].max_hp - 5;
    battle.enemy_cast(0, 2, 0);
    assert_eq!(battle.enemies[0].hp, battle.enemies[0].max_hp);
}

#[test]
fn a_defending_foe_takes_half_of_an_identical_strike() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 0;
    // Foe 0 (open) and foe 1 (defending) are identical bandits; strike each
    // from the same RNG state so only the Defend stance differs.
    let rng_save = battle.rng;
    let Strike::Hit { dmg: full, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    battle.rng = rng_save;
    battle.enemies[1].defending = true;
    let Strike::Hit { dmg: half, .. } = battle.strike_enemy(0, 1) else {
        panic!("a forced-hit strike missed");
    };
    assert!(half < full);
    // Plain halving, no floor (EasyRPG `AdjustDamageForDefend`).
    assert_eq!(half, full / 2);
}

#[test]
fn a_defending_enemy_guards_and_deals_no_damage_that_turn() {
    let mut battle = build_1v2();
    for e in &mut battle.enemies {
        e.actions = vec![enemy_action_def(2)];
    }
    let hp_before = battle.members[0].hp;
    battle.commit(Command::Defend);
    while battle.resolve_next() {}
    assert!(battle.enemies.iter().all(|e| e.defending));
    assert_eq!(battle.members[0].hp, hp_before);
    assert!(battle.log.iter().any(|l| l.contains("védekezik")));
}

#[test]
fn a_double_attack_strikes_the_target_twice() {
    let mut battle = build_1v2();
    let hp0 = battle.members[0].hp;
    // Land both to-hit rolls (draw 0 for the single blow, draws 0 and 2 for
    // the double) so the comparison reflects strike count, not a chance miss.
    wind_enemy_hits(&mut battle, &[0, 2]);
    let rng_save = battle.rng;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    let single = hp0 - battle.members[0].hp;
    assert!(single > 0);
    // Same RNG state, but a double-attack: the first blow matches `single`,
    // the second adds more, so the total clearly exceeds one strike.
    battle.rng = rng_save;
    battle.members[0].hp = hp0;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::DoubleAttack { target: 0 },
        agility: 0,
    });
    let double = hp0 - battle.members[0].hp;
    assert!(
        double > single,
        "double-attack ({double}) should exceed a single strike ({single})"
    );
}

#[test]
fn self_destruct_hits_every_member_and_hides_the_foe_without_killing_it() {
    let mut battle = build_party2();
    let hp = [battle.members[0].hp, battle.members[1].hp];
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::SelfDestruct,
        agility: 0,
    });
    assert!(battle.members[0].hp < hp[0]);
    assert!(battle.members[1].hp < hp[1]);
    assert_eq!(battle.enemies[0].hp, battle.enemies[0].max_hp);
    assert!(battle.enemies[0].fled);
    assert!(!battle.enemies[0].alive());
    assert!(battle.log.iter().any(|l| l.contains("előretör")));
}

#[test]
fn an_escaping_foe_leaves_battle_and_grants_no_reward() {
    let mut battle = build_1v2();
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Escape,
        agility: 0,
    });
    assert!(battle.enemies[0].fled);
    assert!(!battle.enemies[0].alive());
    assert!(!battle.living_enemies().contains(&0));
    // Defeat the remaining foe: victory pays only for the one truly beaten.
    battle.enemies[1].hp = 0;
    assert!(matches!(battle.end_state(), Some(BattleOutcome::Victory)));
    assert_eq!(battle.victory_rewards(), (10, 30));
    assert!(battle.log.iter().any(|l| l.contains("elmenekül")));
}

#[test]
fn a_charged_foe_doubles_its_next_strike_then_clears() {
    let mut battle = build_1v2();
    let hp0 = battle.members[0].hp;
    wind_enemy_hits(&mut battle, &[0]);
    let rng_save = battle.rng;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    let normal = hp0 - battle.members[0].hp;
    assert!(normal > 0);
    // Same RNG, but the foe has charged: the strike lands double, then clears.
    battle.rng = rng_save;
    battle.members[0].hp = hp0;
    battle.enemies[0].charging = true;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    let charged = hp0 - battle.members[0].hp;
    assert!(
        charged > normal,
        "charged strike ({charged}) should exceed a normal one ({normal})"
    );
    assert!(!battle.enemies[0].charging);
}

#[test]
fn equipped_element_defence_halves_a_matching_enemy_skill_only() {
    use crate::battle::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let mut ron = testkit::actor(1, 3, 200, 50);
    ron.armor = 2;
    let actors = vec![&ron];
    let mut armor = testkit::item(2, 0, 0, 0, 0, 5);
    armor.item_type = 3;
    let items = vec![armor];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
    let mut battle = Battle::build(
        &troop,
        &monsters,
        &actors,
        &[testkit::slots(&ron)],
        &items,
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        5,
    );
    assert!(battle.members[0].resist_attributes.contains(&5));
    battle.attributes = vec![fire_attr()];
    // With the skill's variance set to 0 the halving is exact: the guarded
    // cast deals base/2 and the unguarded cast deals the full base, so the two
    // compare cleanly without depending on the variance draw.
    let before = battle.members[0].hp;
    let mut guarded = damage_skill(1, 40, vec![5], vec![]);
    guarded.variance = 0;
    battle.skills = vec![guarded];
    battle.enemy_cast(0, 1, 0);
    let resisted = before - battle.members[0].hp;
    battle.members[0].hp = before;
    let mut unguarded = damage_skill(1, 40, vec![6], vec![]);
    unguarded.variance = 0;
    battle.skills = vec![unguarded];
    battle.enemy_cast(0, 1, 0);
    let full = before - battle.members[0].hp;
    assert!(resisted < full);
    assert_eq!(resisted, (full / 2).max(1));
}

#[test]
fn a_physical_skill_auto_hits_a_foe_that_cannot_act() {
    let mut battle = build_1v2();
    battle.states = vec![sleep_state(7)];
    battle.enemies[0].states = vec![(7, 0)];
    battle.enemies[0].hp = 500;
    let mut s = damage_skill(1, 30, vec![], vec![]);
    s.hit = 1;
    s.failure_message = 3;
    battle.skills = vec![s];
    let before = battle.enemies[0].hp;
    battle.cast_skill(0, 1, 0);
    assert!(
        battle.enemies[0].hp < before,
        "a cannot-act foe cannot dodge the cast"
    );
}

#[test]
fn self_destruct_deals_attack_minus_half_defence_per_member() {
    let mut battle = build_party2();
    battle.enemies[0].stats.attack = 40;
    battle.members[0].stats.defense = 40;
    let hp0 = battle.members[0].hp;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::SelfDestruct,
        agility: 0,
    });
    let taken = hp0 - battle.members[0].hp;
    // CalcSelfDestructEffect base 20, then the var=4 spread (±4); a flat-attack
    // (40) blow would land well outside this band.
    assert!((16..=24).contains(&taken), "self-destruct dealt {taken}");
    assert!(taken < 40, "it is attack - def/2, not the flat attack");
}

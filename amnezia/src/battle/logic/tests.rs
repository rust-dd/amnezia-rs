//! Unit tests for the pure battle formulas.

use super::*;

fn skill(id: u32, name: &str, sp_cost: u32, power: u32) -> SkillDef {
    SkillDef {
        affect_stats: [false; 4],
        ignore_defense: false,
        id,
        name: name.into(),
        description: String::new(),
        sp_cost,
        power,
        hit: 0,
        skill_type: 0,
        failure_message: 0,
        scope: 0,
        animation_id: 0,
        physical_rate: 0,
        magical_rate: 3,
        variance: 4,
        affect_hp: false,
        affect_sp: false,
        absorb: false,
        attributes: vec![],
        affected_states: vec![],
    }
}

fn gear(id: u32, atk: u32, def: u32, spi: u32, agi: u32) -> ItemDef {
    ItemDef {
        prevent_critical: false,
        raise_evasion: false,
        half_sp_cost: false,
        actor_set: Vec::new(),
        state_chance: 0,
        id,
        name: String::new(),
        description: String::new(),
        item_type: 1,
        price: 0,
        recover_hp: 0,
        recover_hp_rate: 0,
        recover_sp: 0,
        recover_sp_rate: 0,
        cure_states: vec![],
        scope: 0,
        only_field: false,
        ko_only: false,
        uses: 0,
        atk,
        def,
        spi,
        agi,
        attribute_defense: vec![],
        state_defense: vec![],
        two_handed: false,
        hit: 0,
        crit: 0,
        weapon_animation: 0,
    }
}

fn attr(id: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> AttributeDef {
    AttributeDef {
        id,
        name: String::new(),
        attribute_type: 0,
        a_rate: a,
        b_rate: b,
        c_rate: c,
        d_rate: d,
        e_rate: e,
    }
}

#[test]
fn physical_damage_is_half_attack_less_quarter_defense_floored_at_zero() {
    assert_eq!(physical_damage(40, 20), 15);
    assert_eq!(physical_damage(20, 8), 8);
    assert_eq!(physical_damage(4, 100), 0);
}

#[test]
fn effective_hit_defaults_only_an_empty_weapon_slot_to_ninety() {
    assert_eq!(effective_hit(None), 90);
    assert_eq!(effective_hit(Some(0)), 0);
    assert_eq!(effective_hit(Some(85)), 85);
}

#[test]
fn to_hit_adjusts_the_base_by_the_agility_gap() {
    assert_eq!(to_hit(90, 10, 10), 90);
    assert_eq!(to_hit(100, 10, 10), 100);
    assert!(to_hit(90, 10, 20) < 90);
    assert!(to_hit(90, 20, 10) > 90);
    assert_eq!(to_hit(90, 10, 20), 85);
    assert_eq!(to_hit(90, 20, 10), 92);
    assert_eq!(to_hit(100, 5, 50), 100);
    assert_eq!(to_hit(90, 0, 10), 45);
}

#[test]
fn critical_triples_the_base() {
    assert_eq!(critical_damage(12), 36);
    assert_eq!(critical_damage(0), 0);
}

#[test]
fn skill_effect_matches_the_easyrpg_formula() {
    let mut s = skill(1, "X", 10, 50);
    s.physical_rate = 2;
    let src = Stats {
        attack: 40,
        defense: 10,
        spirit: 20,
        agility: 8,
    };
    let tgt = Stats {
        attack: 30,
        defense: 20,
        spirit: 12,
        agility: 6,
    };
    assert_eq!(skill_effect(&s, &src, &tgt, true), 54);
    assert_eq!(skill_effect(&s, &src, &tgt, false), 55);
    let weak = skill(2, "w", 0, 1);
    let tanky = Stats {
        attack: 0,
        defense: 400,
        spirit: 400,
        agility: 0,
    };
    assert_eq!(skill_effect(&weak, &src, &tanky, true), 0);
}

#[test]
fn variance_adjust_matches_easyrpg_for_known_rolls() {
    assert_eq!(variance_adjust(100, 4, 0), 80);
    assert_eq!(variance_adjust(100, 4, 20), 100);
    assert_eq!(variance_adjust(100, 4, 40), 120);
    assert_eq!(variance_adjust(100, 4, 41), 80);
    assert_eq!(variance_adjust(100, 0, 999), 100);
    assert_eq!(variance_adjust(0, 4, 999), 0);
    assert_eq!(variance_adjust(1, 4, 0), 1);
    assert_eq!(variance_adjust(1, 4, 1), 2);
}

#[test]
fn defend_halves_rounding_down() {
    assert_eq!(defended(11), 5);
    assert_eq!(defended(0), 0);
}

#[test]
fn turn_order_is_fastest_first_and_stable_on_ties() {
    assert_eq!(turn_order(&[8, 12, 5, 12, 20]), vec![4, 1, 3, 0, 2]);
}

#[test]
fn escape_chance_uses_the_ratio_of_average_agilities_and_clamps() {
    assert_eq!(init_escape_chance(10, 10), 50);
    assert_eq!(init_escape_chance(20, 10), 100);
    assert_eq!(init_escape_chance(10, 7), 80);
    assert_eq!(init_escape_chance(10, 20), 0);
    assert_eq!(init_escape_chance(0, 10), 0);
    assert!(escape_succeeds(60, 59) && !escape_succeeds(60, 60));
}

#[test]
fn average_agility_is_the_integer_mean_or_one_when_empty() {
    assert_eq!(average_agility(&[8, 8, 8]), 8);
    assert_eq!(average_agility(&[10, 5]), 7);
    assert_eq!(average_agility(&[]), 1);
}

#[test]
fn to_hit_vs_forces_a_certain_hit_against_a_target_that_cannot_act() {
    assert_eq!(to_hit_vs(90, 10, 10, true), to_hit(90, 10, 10));
    assert_eq!(to_hit_vs(90, 10, 20, true), 85);
    assert_eq!(to_hit_vs(90, 10, 20, false), 100);
    assert_eq!(to_hit_vs(0, 5, 50, false), 100);
}

#[test]
fn select_target_wraps_over_only_the_living() {
    let alive = [false, true, false, true];
    assert_eq!(select_target(&alive, 0), Some(1));
    assert_eq!(select_target(&alive, 1), Some(3));
    assert_eq!(select_target(&alive, 2), Some(1));
    assert_eq!(select_target(&[false, false], 0), None);
}

#[test]
fn rewards_sum_over_the_troop() {
    assert_eq!(total_rewards(&[(10, 30), (10, 30), (100, 100)]), (120, 160));
    assert_eq!(total_rewards(&[]), (0, 0));
}

#[test]
fn actor_stats_grow_with_level() {
    assert_eq!(
        actor_stats(2),
        Stats {
            attack: 28,
            defense: 16,
            spirit: 14,
            agility: 12
        }
    );
    assert!(actor_stats(10).attack > actor_stats(2).attack);
}

#[test]
fn actor_stats_at_reads_the_curve_then_falls_back_when_empty() {
    let curves = ActorCurves {
        max_hp: vec![100, 150, 200],
        max_sp: vec![10, 20, 30],
        attack: vec![10, 20, 30],
        defense: vec![5, 10, 15],
        spirit: vec![4, 8, 12],
        agility: vec![3, 6, 9],
    };
    assert_eq!(
        actor_stats_at(&curves, 2),
        Stats {
            attack: 20,
            defense: 10,
            spirit: 8,
            agility: 6
        }
    );
    assert_eq!(actor_hp_sp_at(&curves, 3, 0, 0), (200, 30));
    let empty = ActorCurves::default();
    assert_eq!(actor_stats_at(&empty, 2), actor_stats(2));
    assert_eq!(actor_hp_sp_at(&empty, 2, 63, 37), (63, 37));
}

#[test]
fn equipment_bonus_sums_only_the_equipped_gear() {
    use super::super::model::testkit::actor;
    let items = vec![
        gear(1, 10, 5, 0, 2),
        gear(2, 0, 20, 4, 1),
        gear(9, 99, 99, 99, 99),
    ];
    let mut a = actor(1, 2, 60, 30);
    a.weapon = 1;
    a.armor = 2;
    assert_eq!(
        equipment_bonus(&a, &items),
        Stats {
            attack: 10,
            defense: 25,
            spirit: 4,
            agility: 3,
        }
    );
    a.weapon = 0;
    a.armor = 0;
    a.helmet = 777;
    assert_eq!(equipment_bonus(&a, &items), Stats::default());
}

#[test]
fn attribute_percent_maps_ranks_a_through_e() {
    let fire = attr(5, 200, 150, 100, 50, 0);
    assert_eq!(attribute_percent(&fire, 0), 200);
    assert_eq!(attribute_percent(&fire, 1), 150);
    assert_eq!(attribute_percent(&fire, 2), 100);
    assert_eq!(attribute_percent(&fire, 3), 50);
    assert_eq!(attribute_percent(&fire, 4), 0);
    assert_eq!(attribute_percent(&fire, 9), 0);
}

#[test]
fn elemental_damage_amplifies_weak_reduces_resist_and_passes_through() {
    let attrs = vec![
        attr(5, 200, 150, 100, 50, 0),
        attr(6, 200, 150, 100, 50, 50),
    ];
    let ranks = [0u8, 0, 0, 0, 0, 4];
    let rank = |id: u32| {
        ranks
            .get(id.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(2)
    };
    assert_eq!(attribute_damage(100, &[5], &attrs, rank), 200);
    assert_eq!(attribute_damage(100, &[6], &attrs, rank), 50);
    assert_eq!(attribute_damage(100, &[0], &attrs, rank), 100);
    assert_eq!(attribute_damage(100, &[42], &attrs, rank), 100);
    assert_eq!(attribute_damage(80, &[6], &attrs, |_| 2), 80);
}

#[test]
fn state_infliction_chance_maps_ranks_a_through_e() {
    let state = state(1, 0, 0, 0, 0);
    for (rank, chance) in [100, 80, 60, 30, 0].into_iter().enumerate() {
        assert_eq!(state_infliction_chance(&state, rank as u8), chance);
    }
    assert_eq!(state_infliction_chance(&state, 9), 0);
}

fn state(id: u32, restriction: u32, hold_turn: u32, auto: u32, by_damage: u32) -> StateDef {
    StateDef {
        affect_type: 0,
        affect_stats: [false; 4],
        reduce_hit_ratio: 100,
        restrict_skill: false,
        restrict_skill_level: 0,
        restrict_magic: false,
        restrict_magic_level: 0,
        sp_change_type: 0,
        sp_change_max: 0,
        sp_change_val: 0,
        rates: [100, 80, 60, 30, 0],
        persistence: 0,
        id,
        name: format!("S{id}"),
        restriction,
        priority: 0,
        hold_turn,
        auto_release_prob: auto,
        release_by_damage: by_damage,
        hp_change_type: 0,
        hp_change_max: 0,
        hp_change_val: 0,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

#[test]
fn inflict_is_idempotent_and_cure_removes() {
    let mut states = vec![];
    inflict(&mut states, 3);
    inflict(&mut states, 3);
    inflict(&mut states, 5);
    assert_eq!(states, vec![(3, 0), (5, 0)]);
    assert!(has_state(&states, 3) && !has_state(&states, 9));
    cure(&mut states, 3);
    assert_eq!(states, vec![(5, 0)]);
    cure(&mut states, 99);
    assert_eq!(states, vec![(5, 0)]);
}

#[test]
fn worst_restriction_prioritizes_cannot_act_then_enemy_then_ally() {
    let defs = vec![
        state(1, 1, 0, 0, 0),
        state(2, 3, 0, 0, 0),
        state(3, 2, 0, 0, 0),
    ];
    assert_eq!(worst_restriction(&[(1, 0), (3, 0)], &defs), 1);
    assert_eq!(worst_restriction(&[(1, 0), (2, 0), (3, 0)], &defs), 1);
    assert_eq!(worst_restriction(&[(2, 0), (3, 0)], &defs), 2);
    assert_eq!(worst_restriction(&[], &defs), 0);
    assert_eq!(worst_restriction(&[(99, 0)], &defs), 0);
}

#[test]
fn tick_recovery_wears_off_after_hold_and_spares_the_death_state() {
    let defs = vec![state(1, 0, 0, 100, 0), state(2, 0, 0, 100, 0)];
    let mut states = vec![(1, 0), (2, 0)];
    assert_eq!(tick_recovery(&mut states, &defs, || 0), vec![2]);
    assert_eq!(states, vec![(1, 0)]);

    let held = vec![state(5, 0, 2, 100, 0)];
    let mut s = vec![(5, 0)];
    assert!(tick_recovery(&mut s, &held, || 0).is_empty());
    assert_eq!(s, vec![(5, 1)]);
    assert!(tick_recovery(&mut s, &held, || 0).is_empty());
    assert_eq!(s, vec![(5, 2)]);
    assert_eq!(tick_recovery(&mut s, &held, || 0), vec![5]);
    assert!(s.is_empty());
}

#[test]
fn release_on_damage_lifts_by_chance_and_spares_the_death_state() {
    let defs = vec![
        state(1, 0, 0, 0, 100),
        state(4, 0, 0, 0, 100),
        state(5, 0, 0, 0, 0),
    ];
    let mut states = vec![(1, 0), (4, 0), (5, 0)];
    assert_eq!(release_on_damage(&mut states, &defs, 100, || 0), vec![4]);
    assert_eq!(states, vec![(1, 0), (5, 0)]);
    let mut s = vec![(4, 0)];
    assert!(release_on_damage(&mut s, &defs, 100, || 100).is_empty());
    assert_eq!(s, vec![(4, 0)]);
}

fn state_hp(id: u32, hp_change_type: u32, hp_change_max: u32, hp_change_val: u32) -> StateDef {
    StateDef {
        affect_type: 0,
        affect_stats: [false; 4],
        reduce_hit_ratio: 100,
        restrict_skill: false,
        restrict_skill_level: 0,
        restrict_magic: false,
        restrict_magic_level: 0,
        sp_change_type: 0,
        sp_change_max: 0,
        sp_change_val: 0,
        rates: [100, 80, 60, 30, 0],
        persistence: 0,
        id,
        name: format!("S{id}"),
        restriction: 0,
        priority: 0,
        hold_turn: 0,
        auto_release_prob: 0,
        release_by_damage: 0,
        hp_change_type,
        hp_change_max,
        hp_change_val,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

#[test]
fn state_hp_delta_signs_by_type_and_truncates_small_percentages() {
    assert_eq!(state_hp_delta(&state_hp(2, 0, 5, 1), 100), -6);
    assert_eq!(state_hp_delta(&state_hp(2, 1, 5, 1), 100), 6);
    assert_eq!(state_hp_delta(&state_hp(2, 2, 5, 1), 100), 0);
    assert_eq!(state_hp_delta(&state_hp(2, 0, 0, 0), 100), 0);
    assert_eq!(state_hp_delta(&state_hp(2, 0, 1, 0), 50), 0);
}

fn action(
    kind: u32,
    basic: u32,
    skill_id: u32,
    condition_type: u32,
    condition_min: u32,
    condition_max: u32,
    priority: u32,
) -> EnemyActionDef {
    EnemyActionDef {
        kind,
        basic,
        skill_id,
        enemy_id: 0,
        condition_type,
        condition_min,
        condition_max,
        priority,
        ..Default::default()
    }
}

#[test]
fn enemy_command_maps_each_action_family() {
    assert!(matches!(
        enemy_command(Some(&action(1, 0, 4, 0, 0, 0, 0)), 2),
        Command::Skill {
            skill_id: 4,
            target: 2
        }
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 2, 0, 0, 0, 0, 0)), 0),
        Command::Defend
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 3, 0, 0, 0, 0, 0)), 0),
        Command::Nothing
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 0, 0, 0, 0, 0, 0)), 1),
        Command::Attack { target: 1 }
    ));
    assert!(matches!(enemy_command(None, 3), Command::Nothing));
}

#[test]
fn enemy_command_maps_the_monster_only_basics() {
    assert!(matches!(
        enemy_command(Some(&action(0, 1, 0, 0, 0, 0, 0)), 2),
        Command::DoubleAttack { target: 2 }
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 4, 0, 0, 0, 0, 0)), 0),
        Command::Charge
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 5, 0, 0, 0, 0, 0)), 0),
        Command::SelfDestruct
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 6, 0, 0, 0, 0, 0)), 0),
        Command::Escape
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 7, 0, 0, 0, 0, 0)), 0),
        Command::Nothing
    ));
    assert!(matches!(
        enemy_command(Some(&action(0, 9, 0, 0, 0, 0, 0)), 4),
        Command::Attack { target: 4 }
    ));
}

#[test]
fn equipment_resist_unions_attribute_defense_and_dedups() {
    let mut weapon = gear(1, 0, 0, 0, 0);
    weapon.attribute_defense = vec![3];
    let mut armor = gear(2, 0, 0, 0, 0);
    armor.item_type = 3;
    armor.attribute_defense = vec![5, 3];
    let mut unequipped = gear(9, 0, 0, 0, 0);
    unequipped.attribute_defense = vec![7];
    let items = vec![weapon, armor, unequipped];
    let mut resist = equipment_resist_slots([1, 0, 2, 0, 0], &items);
    resist.sort();
    assert_eq!(resist, vec![3, 5]);
    assert!(equipment_resist_slots([0, 0, 0, 0, 0], &items).is_empty());
}

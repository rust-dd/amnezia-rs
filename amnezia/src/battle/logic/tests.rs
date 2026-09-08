//! Unit tests for the pure battle formulas.

use super::*;

fn skill(id: u32, name: &str, sp_cost: u32, power: u32) -> SkillDef {
    SkillDef {
        id,
        name: name.into(),
        description: String::new(),
        sp_cost,
        power,
        hit: 0,
        skill_type: 0,
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
    assert_eq!(physical_damage(40, 20), 15); // 20 - 5
    assert_eq!(physical_damage(20, 8), 8); // 10 - 2
    assert_eq!(physical_damage(4, 100), 0); // floored, never negative
}

#[test]
fn effective_hit_defaults_bare_hands_to_ninety() {
    assert_eq!(effective_hit(0), 90); // empty weapon slot -> RM2000 default
    assert_eq!(effective_hit(85), 85); // a real weapon keeps its own rate
}

#[test]
fn to_hit_adjusts_the_base_by_the_agility_gap() {
    // Equal agility leaves the base hit unchanged.
    assert_eq!(to_hit(90, 10, 10), 90);
    assert_eq!(to_hit(100, 10, 10), 100);
    // A faster target lowers the chance; a slower target raises it.
    assert!(to_hit(90, 10, 20) < 90); // 100 - 10*(1 + 0.5) = 85
    assert!(to_hit(90, 20, 10) > 90); // 100 - 10*(1 - 0.25) = 92
    assert_eq!(to_hit(90, 10, 20), 85);
    assert_eq!(to_hit(90, 20, 10), 92);
    // A perfect base always lands, whatever the agilities.
    assert_eq!(to_hit(100, 5, 50), 100);
    // Source agility is guarded against zero — no divide-by-zero, no panic.
    assert_eq!(to_hit(90, 0, 10), 45); // src clamps to 1: 100 - 10*(1 + 4.5)
}

#[test]
fn critical_triples_the_base() {
    assert_eq!(critical_damage(12), 36);
    assert_eq!(critical_damage(0), 0);
}

#[test]
fn skill_effect_matches_the_easyrpg_formula() {
    let mut s = skill(1, "X", 10, 50); // power 50, physical_rate 0, magical_rate 3
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
    // Enemy scope: 50 + 2*40/20 + 3*20/40 - 2*20/40 - 3*12/80
    //            = 50 + 4 + 1 - 1 - 0 = 54.
    assert_eq!(skill_effect(&s, &src, &tgt, true), 54);
    // Ally/heal scope: no defensive subtraction -> 50 + 4 + 1 = 55.
    assert_eq!(skill_effect(&s, &src, &tgt, false), 55);
    // Overwhelming defence floors the effect at 0.
    let weak = skill(2, "w", 0, 1); // power 1, magical_rate 3
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
    // var=4, base=100: adj = max(1, 400/10) = 40, window [base-20, base+20].
    assert_eq!(variance_adjust(100, 4, 0), 80); // 100 + (0 % 41 = 0) - 20
    assert_eq!(variance_adjust(100, 4, 20), 100); // centre: 100 + 20 - 20
    assert_eq!(variance_adjust(100, 4, 40), 120); // 100 + 40 - 20
    assert_eq!(variance_adjust(100, 4, 41), 80); // roll wraps: 41 % 41 = 0
    // var=0 leaves the base untouched.
    assert_eq!(variance_adjust(100, 0, 999), 100);
    // A non-positive base is returned unchanged (an immune hit stays 0).
    assert_eq!(variance_adjust(0, 4, 999), 0);
    // A tiny base still gets a floor-1 window: adj = max(1, 4/10 = 0) = 1.
    assert_eq!(variance_adjust(1, 4, 0), 1); // 1 + 0 - 0
    assert_eq!(variance_adjust(1, 4, 1), 2); // 1 + 1 - 0
}

#[test]
fn defend_halves_rounding_down() {
    assert_eq!(defended(11), 5);
    assert_eq!(defended(0), 0);
}

#[test]
fn turn_order_is_fastest_first_and_stable_on_ties() {
    // indices 0..4 with agilities: ties (12) keep input order 1 before 3.
    assert_eq!(turn_order(&[8, 12, 5, 12, 20]), vec![4, 1, 3, 0, 2]);
}

#[test]
fn escape_chance_uses_the_ratio_of_average_agilities_and_clamps() {
    // Equal average agility -> 150 - round(100) = 50%.
    assert_eq!(init_escape_chance(10, 10), 50);
    // A faster party (slower enemies) escapes more easily, clamped at 100.
    assert_eq!(init_escape_chance(20, 10), 100); // 150 - round(50) = 100
    // Rounding of the ratio: 100 * 7 / 10 = 70 -> 150 - 70 = 80.
    assert_eq!(init_escape_chance(10, 7), 80);
    // A slower party escapes less easily, clamped at 0.
    assert_eq!(init_escape_chance(10, 20), 0); // 150 - 200 = -50 -> 0
    // A zero party average is guarded against a divide-by-zero.
    assert_eq!(init_escape_chance(0, 10), 0);
    // PercentChance semantics: a roll strictly under the chance succeeds.
    assert!(escape_succeeds(60, 59) && !escape_succeeds(60, 60));
}

#[test]
fn average_agility_is_the_integer_mean_or_one_when_empty() {
    assert_eq!(average_agility(&[8, 8, 8]), 8);
    assert_eq!(average_agility(&[10, 5]), 7); // 15 / 2 = 7 (integer division)
    assert_eq!(average_agility(&[]), 1); // an empty side guards the division
}

#[test]
fn to_hit_vs_forces_a_certain_hit_against_a_target_that_cannot_act() {
    // A target that can act uses the ordinary agility-adjusted chance.
    assert_eq!(to_hit_vs(90, 10, 10, true), to_hit(90, 10, 10));
    assert_eq!(to_hit_vs(90, 10, 20, true), 85);
    // A target that cannot act (asleep/paralyzed) is struck with certainty,
    // whatever the base hit or the agility gap.
    assert_eq!(to_hit_vs(90, 10, 20, false), 100);
    assert_eq!(to_hit_vs(0, 5, 50, false), 100);
}

#[test]
fn select_target_wraps_over_only_the_living() {
    let alive = [false, true, false, true];
    assert_eq!(select_target(&alive, 0), Some(1));
    assert_eq!(select_target(&alive, 1), Some(3));
    assert_eq!(select_target(&alive, 2), Some(1)); // wraps
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
    // Empty curve -> fall back to the formula / provided fallbacks.
    let empty = ActorCurves::default();
    assert_eq!(actor_stats_at(&empty, 2), actor_stats(2));
    assert_eq!(actor_hp_sp_at(&empty, 2, 63, 37), (63, 37));
}

#[test]
fn usable_skills_keeps_affordable_offensive_rows_only() {
    let skills = vec![
        skill(1, "X-Csapás", 20, 50), // affordable, offensive
        skill(2, "Főnix", 300, 999),  // too expensive
        skill(3, "--------", 0, 0),   // divider row
        skill(4, "Lélekdal", 50, 0),  // zero power (non-damage)
    ];
    let usable = usable_skills(&skills, 40);
    assert_eq!(usable.len(), 1);
    assert_eq!(usable[0].id, 1);
}

#[test]
fn equipment_bonus_sums_only_the_equipped_gear() {
    use super::super::model::testkit::actor;
    let items = vec![
        gear(1, 10, 5, 0, 2),    // weapon
        gear(2, 0, 20, 4, 1),    // armor
        gear(9, 99, 99, 99, 99), // in the catalogue but not equipped
    ];
    let mut a = actor(1, 2, 60, 30);
    a.weapon = 1;
    a.armor = 2; // shield/helmet/accessory stay 0 (empty slots)
    assert_eq!(
        equipment_bonus(&a, &items),
        Stats {
            attack: 10,
            defense: 25,
            spirit: 4,
            agility: 3,
        }
    );
    // Empty and unknown ids add nothing.
    a.weapon = 0;
    a.armor = 0;
    a.helmet = 777;
    assert_eq!(equipment_bonus(&a, &items), Stats::default());
}

#[test]
fn attribute_percent_maps_ranks_a_through_e() {
    let fire = attr(5, 200, 150, 100, 50, 0);
    assert_eq!(attribute_percent(&fire, 0), 200); // A, most vulnerable
    assert_eq!(attribute_percent(&fire, 1), 150); // B
    assert_eq!(attribute_percent(&fire, 2), 100); // C, neutral
    assert_eq!(attribute_percent(&fire, 3), 50); // D
    assert_eq!(attribute_percent(&fire, 4), 0); // E, immune
    assert_eq!(attribute_percent(&fire, 9), 0); // past E clamps to E
}

#[test]
fn elemental_damage_amplifies_weak_reduces_resist_and_passes_through() {
    // attr 5 (fire): weak A doubles, resist E zeroes; attr 6 (ice): resist E halves.
    let attrs = vec![
        attr(5, 200, 150, 100, 50, 0),
        attr(6, 200, 150, 100, 50, 50),
    ];
    let ranks = [0u8, 0, 0, 0, 0, 4]; // fire -> A (weak), ice -> E (resist)
    assert_eq!(elemental_damage(100, 5, &ranks, &attrs), 200); // weak amplifies
    assert_eq!(elemental_damage(100, 6, &ranks, &attrs), 50); // resist reduces
    assert_eq!(elemental_damage(100, 0, &ranks, &attrs), 100); // non-elemental unchanged
    assert_eq!(elemental_damage(100, 42, &ranks, &attrs), 100); // unknown id -> unchanged
    // An id past the truncated rank vector reads neutral C (100%).
    assert_eq!(elemental_damage(80, 6, &[0], &attrs), 80);
}

#[test]
fn state_infliction_chance_maps_ranks_a_through_e() {
    assert_eq!(state_infliction_chance(0), 100); // A always lands
    assert_eq!(state_infliction_chance(1), 80);
    assert_eq!(state_infliction_chance(2), 60);
    assert_eq!(state_infliction_chance(3), 40);
    assert_eq!(state_infliction_chance(4), 0); // E never lands
    assert_eq!(state_infliction_chance(9), 0); // past E clamps to E
}

fn state(id: u32, restriction: u32, hold_turn: u32, auto: u32, by_damage: u32) -> StateDef {
    StateDef {
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
    inflict(&mut states, 3); // no duplicate
    inflict(&mut states, 5);
    assert_eq!(states, vec![(3, 0), (5, 0)]);
    assert!(has_state(&states, 3) && !has_state(&states, 9));
    cure(&mut states, 3);
    assert_eq!(states, vec![(5, 0)]);
    cure(&mut states, 99); // absent -> no-op
    assert_eq!(states, vec![(5, 0)]);
}

#[test]
fn worst_restriction_takes_the_max_across_active_states() {
    let defs = vec![
        state(1, 1, 0, 0, 0), // can't act
        state(2, 3, 0, 0, 0), // confusion
        state(3, 2, 0, 0, 0), // berserk
    ];
    assert_eq!(worst_restriction(&[(1, 0), (3, 0)], &defs), 2); // 1 and 2 -> 2
    assert_eq!(worst_restriction(&[(1, 0), (2, 0), (3, 0)], &defs), 3); // + confusion
    assert_eq!(worst_restriction(&[], &defs), 0); // none active
    assert_eq!(worst_restriction(&[(99, 0)], &defs), 0); // id absent from defs
}

#[test]
fn tick_recovery_wears_off_after_hold_and_spares_the_death_state() {
    // Death (id 1) never lifts; state 2 lifts at once (hold 0, 100%).
    let defs = vec![state(1, 0, 0, 100, 0), state(2, 0, 0, 100, 0)];
    let mut states = vec![(1, 0), (2, 0)];
    assert_eq!(tick_recovery(&mut states, &defs, || 0), vec![2]);
    assert_eq!(states, vec![(1, 0)]); // KO endures, its turn untouched

    // hold_turn gates the roll: a 2-turn hold survives round 1, lifts on round 2.
    let held = vec![state(5, 0, 2, 100, 0)];
    let mut s = vec![(5, 0)];
    assert!(tick_recovery(&mut s, &held, || 0).is_empty());
    assert_eq!(s, vec![(5, 1)]);
    assert_eq!(tick_recovery(&mut s, &held, || 0), vec![5]);
    assert!(s.is_empty());
}

#[test]
fn release_on_damage_lifts_by_chance_and_spares_the_death_state() {
    // state 4 shakes off on any hit (100%); death (1) never; state 5 (0%) never.
    let defs = vec![
        state(1, 0, 0, 0, 100),
        state(4, 0, 0, 0, 100),
        state(5, 0, 0, 0, 0),
    ];
    let mut states = vec![(1, 0), (4, 0), (5, 0)];
    assert_eq!(release_on_damage(&mut states, &defs, || 0), vec![4]);
    assert_eq!(states, vec![(1, 0), (5, 0)]);
    // A roll at or above the percent keeps the state (100 !< 100).
    let mut s = vec![(4, 0)];
    assert!(release_on_damage(&mut s, &defs, || 100).is_empty());
    assert_eq!(s, vec![(4, 0)]);
}

fn state_hp(id: u32, hp_change_type: u32, hp_change_max: u32, hp_change_val: u32) -> StateDef {
    StateDef {
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
fn state_hp_delta_signs_by_type_and_floors_a_configured_change_at_one() {
    // Poison (type 0): 5% of a 100-max battler plus 1 flat drains 6.
    assert_eq!(state_hp_delta(&state_hp(2, 0, 5, 1), 100), -6);
    // The same amounts as a type-1 regen gain 6.
    assert_eq!(state_hp_delta(&state_hp(2, 1, 5, 1), 100), 6);
    // Type 2 does nothing, whatever the amounts.
    assert_eq!(state_hp_delta(&state_hp(2, 2, 5, 1), 100), 0);
    // An unconfigured state never moves HP, even at the default type 0 — so a
    // plain restriction state (confusion, KO) does not bleed.
    assert_eq!(state_hp_delta(&state_hp(2, 0, 0, 0), 100), 0);
    // A configured drain that rounds to zero still bleeds the RM2000 minimum 1.
    assert_eq!(state_hp_delta(&state_hp(2, 0, 1, 0), 50), -1); // 50 * 1 / 100 = 0 -> 1
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
    }
}

#[test]
fn choose_enemy_action_gates_conditions_breaks_priority_and_empties_to_none() {
    // Empty and fully-gated lists yield None (caller falls back to attack).
    assert!(choose_enemy_action(&[], 100, 100, 1, 1, 0).is_none());

    // Turn (type 2, min 2, interval 2) fires on rounds 2, 4, ... not 1, 3.
    let turn = [action(0, 0, 0, 2, 2, 2, 10)];
    assert!(choose_enemy_action(&turn, 100, 100, 1, 1, 0).is_none());
    assert!(choose_enemy_action(&turn, 100, 100, 1, 2, 0).is_some());
    assert!(choose_enemy_action(&turn, 100, 100, 1, 3, 0).is_none());
    assert!(choose_enemy_action(&turn, 100, 100, 1, 4, 0).is_some());

    // Monster-HP% (type 3, [0, 30]) only fires while the enemy is low.
    let hp = [action(1, 0, 7, 3, 0, 30, 5)];
    assert!(choose_enemy_action(&hp, 100, 100, 1, 1, 0).is_none());
    let low = choose_enemy_action(&hp, 20, 100, 1, 1, 0).unwrap();
    assert_eq!((low.kind, low.skill_id), (1, 7));

    // Highest priority wins over an always-eligible basic attack.
    let mix = [action(0, 0, 0, 0, 0, 0, 1), action(1, 0, 9, 0, 0, 0, 8)];
    assert_eq!(
        choose_enemy_action(&mix, 100, 100, 1, 1, 0)
            .unwrap()
            .skill_id,
        9
    );
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
    // A plain basic attack, and the None fallback, both attack.
    assert!(matches!(
        enemy_command(Some(&action(0, 0, 0, 0, 0, 0, 0)), 1),
        Command::Attack { target: 1 }
    ));
    assert!(matches!(
        enemy_command(None, 3),
        Command::Attack { target: 3 }
    ));
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
    // Observe (3) and do-nothing (7) both no-op; an unknown basic attacks.
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
    armor.attribute_defense = vec![5, 3]; // overlaps the weapon's 3
    let mut unequipped = gear(9, 0, 0, 0, 0);
    unequipped.attribute_defense = vec![7];
    let items = vec![weapon, armor, unequipped];
    // Weapon (1) and armor (2) equipped; shield/helmet/accessory empty.
    let mut resist = equipment_resist_slots([1, 0, 2, 0, 0], &items);
    resist.sort();
    assert_eq!(resist, vec![3, 5]); // deduped union; unequipped 7 excluded
    // No gear -> nothing guarded.
    assert!(equipment_resist_slots([0, 0, 0, 0, 0], &items).is_empty());
}

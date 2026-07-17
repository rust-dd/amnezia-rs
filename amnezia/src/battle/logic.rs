//! Pure battle mathematics: the RM2000-flavoured damage, turn-order, flee,
//! target and reward formulas, plus the derived party stat curve and the skill
//! filter. Kept free of Bevy and of the live battle state so every rule is
//! unit-testable in isolation; the battle systems are thin wrappers over these.

use super::model::Command;
use amnezia_data::{
    ActorCurves, ActorDef, AttributeDef, EnemyActionDef, ItemDef, MonsterDef, SkillDef,
};

/// A combatant's four battle stats. Enemies read them straight from their
/// [`MonsterDef`]; party members, whose `ActorDef` carries only a level, get
/// them from [`actor_stats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub attack: u32,
    pub defense: u32,
    pub spirit: u32,
    pub agility: u32,
}

impl Stats {
    /// Monster `m`'s stats, read directly from its definition.
    pub fn from_monster(m: &MonsterDef) -> Self {
        Self {
            attack: m.attack,
            defense: m.defense,
            spirit: m.spirit,
            agility: m.agility,
        }
    }
}

/// Derive a party member's battle stats from their `level`. The converted
/// `ActorDef` carries no combat stats (only level and HP/SP), so v1 grows them on
/// a simple linear curve tuned so an early hero trades a handful of blows with the
/// early troops. A documented approximation, not RM2000 stat-curve parity.
pub fn actor_stats(level: u32) -> Stats {
    Stats {
        attack: 16 + level * 6,
        defense: 8 + level * 4,
        spirit: 8 + level * 3,
        agility: 8 + level * 2,
    }
}

/// A party member's battle stats at `level`, read from their stat curve (level L
/// at index L-1). Falls back to the level formula when the curve is empty (e.g. a
/// stale `actors.ron` predating the curve fields).
pub fn actor_stats_at(curves: &ActorCurves, level: u32) -> Stats {
    let i = (level.max(1) - 1) as usize;
    match (
        curves.attack.get(i),
        curves.defense.get(i),
        curves.spirit.get(i),
        curves.agility.get(i),
    ) {
        (Some(&attack), Some(&defense), Some(&spirit), Some(&agility)) => Stats {
            attack,
            defense,
            spirit,
            agility,
        },
        _ => actor_stats(level),
    }
}

/// A party member's max HP/SP at `level` from their curve, falling back to
/// `(fallback_hp, fallback_sp)` when the curve is empty.
pub fn actor_hp_sp_at(
    curves: &ActorCurves,
    level: u32,
    fallback_hp: u32,
    fallback_sp: u32,
) -> (u32, u32) {
    let i = (level.max(1) - 1) as usize;
    let hp = curves.max_hp.get(i).copied().unwrap_or(fallback_hp);
    let sp = curves.max_sp.get(i).copied().unwrap_or(fallback_sp);
    (hp, sp)
}

/// The stat bonus an actor's five equipment slots (weapon, shield, armor, helmet,
/// accessory) add on top of the curve-derived base: the summed `atk`/`def`/`spi`/
/// `agi` of each equipped item. Empty slots (id `0`) and ids absent from `items`
/// contribute nothing.
pub fn equipment_bonus(actor: &ActorDef, items: &[ItemDef]) -> Stats {
    let slots = [
        actor.weapon,
        actor.shield,
        actor.armor,
        actor.helmet,
        actor.accessory,
    ];
    let mut bonus = Stats::default();
    for id in slots {
        if id == 0 {
            continue;
        }
        if let Some(item) = items.iter().find(|i| i.id == id) {
            bonus.attack += item.atk;
            bonus.defense += item.def;
            bonus.spirit += item.spi;
            bonus.agility += item.agi;
        }
    }
    bonus
}

/// RM2000-style physical damage: half the attacker's attack, less a quarter of
/// the defender's defense, never below zero.
pub fn physical_damage(attack: u32, defense: u32) -> i32 {
    (attack as i32 / 2 - defense as i32 / 4).max(0)
}

/// The effective to-hit percentage for a weapon: an empty weapon slot (`hit == 0`,
/// i.e. bare hands) lands at the RM2000 90% default; a real weapon keeps its rate.
pub fn effective_hit(weapon_hit: u32) -> u32 {
    if weapon_hit == 0 { 90 } else { weapon_hit }
}

/// A critical hit's damage: RM2000 triples the blow.
pub fn critical_damage(base: i32) -> i32 {
    base * 3
}

/// Skill damage: the skill's base `power` plus half the caster's spirit, less a
/// quarter of the target's spirit. A powered skill always lands at least 1; a
/// zero-power skill does nothing (its effect type is not modelled in v1).
pub fn skill_damage(power: u32, spirit: u32, target_spirit: u32) -> i32 {
    if power == 0 {
        return 0;
    }
    (power as i32 + spirit as i32 / 2 - target_spirit as i32 / 4).max(1)
}

/// The RM2000 damage percent for `rank` (0=A … 4=E) from an attribute's own A–E
/// rate table. A weak rank yields >100%, a resist rank <100% (E often 0). A rank
/// past E clamps to the E rate.
pub fn attribute_percent(attr: &AttributeDef, rank: u8) -> u32 {
    match rank {
        0 => attr.a_rate,
        1 => attr.b_rate,
        2 => attr.c_rate,
        3 => attr.d_rate,
        _ => attr.e_rate,
    }
}

/// Scale `base` damage by the target's resistance to `attr_id` (100% = unchanged).
/// A non-elemental hit (`attr_id == 0`) or an unknown id leaves `base` untouched;
/// otherwise the target's A–E rank for that attribute (neutral C when the id falls
/// past the truncated `target_ranks` vector) picks the percentage. Consumed by the
/// weapon-strike resolution in [`super::resolve`].
pub fn elemental_damage(
    base: i32,
    attr_id: u32,
    target_ranks: &[u8],
    attributes: &[AttributeDef],
) -> i32 {
    if attr_id == 0 {
        return base;
    }
    let Some(attr) = attributes.iter().find(|a| a.id == attr_id) else {
        return base;
    };
    let rank = target_ranks
        .get((attr_id - 1) as usize)
        .copied()
        .unwrap_or(2);
    (base * attribute_percent(attr, rank) as i32 / 100).max(0)
}

/// Spread a base damage by ±10% from a `roll` in `0..=20` (10 = no change), so
/// the pure formula stays deterministic and the caller owns the randomness.
/// Positive damage stays at least 1; zero stays zero.
pub fn with_variance(base: i32, roll: u32) -> i32 {
    if base <= 0 {
        return base;
    }
    let pct = (roll % 21) as i32 - 10;
    (base + base * pct / 100).max(1)
}

/// Halve incoming damage while defending (RM2000 Defend), rounding down.
pub fn defended(damage: i32) -> i32 {
    damage / 2
}

/// Order combatant indices by `agility` descending — RM2000 acts fastest-first.
/// The stable sort keeps equal-agility combatants in their given order, so the
/// result is deterministic.
pub fn turn_order(agilities: &[u32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..agilities.len()).collect();
    order.sort_by(|&a, &b| agilities[b].cmp(&agilities[a]));
    order
}

/// The party's escape chance in percent (`0..=100`): a 50% base shifted by the
/// agility gap between the fleeing party and the enemies, then clamped to a
/// `25..=90` band so fleeing is always possible but never certain.
pub fn flee_chance(party_agility: u32, enemy_agility: u32) -> u32 {
    let base = 50 + party_agility as i32 - enemy_agility as i32;
    base.clamp(25, 90) as u32
}

/// Whether a `roll` in `0..=99` beats the escape `chance`.
pub fn flee_succeeds(chance: u32, roll: u32) -> bool {
    roll < chance
}

/// Pick the index of the `roll`-th living combatant among `alive`, wrapping. The
/// enemy AI uses it to choose a random living party target; `None` when every
/// flag is false.
pub fn select_target(alive: &[bool], roll: usize) -> Option<usize> {
    let living: Vec<usize> = alive
        .iter()
        .enumerate()
        .filter(|&(_, &a)| a)
        .map(|(i, _)| i)
        .collect();
    living.get(roll % living.len().max(1)).copied()
}

/// Sum the experience and gold from every defeated monster's `(exp, gold)`.
pub fn total_rewards(rewards: &[(u32, u32)]) -> (u32, u32) {
    rewards
        .iter()
        .fold((0, 0), |(exp, gold), &(e, g)| (exp + e, gold + g))
}

/// The offensive skills a caster with `sp` spirit-points can use this turn:
/// affordable and dealing damage (positive power), excluding the database's
/// divider rows (names starting with `-`). Per-actor skill ownership is deferred,
/// so any member may pick from the shared list.
pub fn usable_skills(skills: &[SkillDef], sp: i32) -> Vec<&SkillDef> {
    skills
        .iter()
        .filter(|s| s.power > 0 && s.sp_cost as i32 <= sp && !s.name.starts_with('-'))
        .collect()
}

/// The percent chance (`0..=100`) a status effect lands, from the target's A–E
/// affliction rank for that state (0=A … 4=E): rank A always lands, E never.
pub fn state_infliction_chance(rank: u8) -> u32 {
    match rank {
        0 => 100,
        1 => 80,
        2 => 60,
        3 => 40,
        _ => 0,
    }
}

/// Add `state_id` to an active-state list if it is not already present, so
/// infliction stays idempotent (RM2000 never stacks the same state twice).
pub fn inflict(states: &mut Vec<u32>, state_id: u32) {
    if !states.contains(&state_id) {
        states.push(state_id);
    }
}

/// Remove `state_id` from an active-state list — a cure or a wear-off.
pub fn cure(states: &mut Vec<u32>, state_id: u32) {
    states.retain(|&s| s != state_id);
}

/// The party level the enemy AI assumes: battle state tracks no per-member
/// level, so party-level conditions (`condition_type == 5`) test against this
/// conservative floor rather than a real average — a documented approximation.
pub const AI_PARTY_LEVEL: u32 = 1;

/// The integer percentage (`0..=100`, saturating) `current` is of `max`, used to
/// gate the HP-conditioned enemy-AI actions. A non-positive `max` yields `0`, and
/// a negative `current` (an overkilled combatant) clamps to `0`.
pub fn hp_percent(current: i32, max: i32) -> u32 {
    if max <= 0 {
        return 0;
    }
    (current.max(0) as i64 * 100 / max as i64) as u32
}

/// Pick an enemy's action this turn from its RM2000 AI list, honouring each
/// entry's condition gate. Eligibility by `condition_type`:
/// - `0` always — always eligible.
/// - `2` turn — from `condition_min` on, every `condition_max` rounds:
///   `round >= condition_min && (round - condition_min) % condition_max.max(1) == 0`.
/// - `3` monster-hp% — `enemy_hp_pct` within `[condition_min, condition_max]`.
/// - `4` party-hp% — `party_hp_pct` within `[condition_min, condition_max]`.
/// - `5` party level — `party_level >= condition_min`.
///
/// `1` switch and `6` party-exhausted are treated as never holding: there is no
/// in-battle switch access, and the pure inputs carry no per-member SP to detect
/// an exhausted (0-SP) member. Among the eligible actions the highest `priority`
/// wins; ties are broken by `roll`. Returns `None` when nothing is eligible (the
/// caller then falls back to a basic attack), so an empty or fully-gated list
/// still yields a fight.
pub fn choose_enemy_action(
    actions: &[EnemyActionDef],
    enemy_hp_pct: u32,
    party_hp_pct: u32,
    party_level: u32,
    round: u32,
    roll: u64,
) -> Option<EnemyActionDef> {
    let eligible: Vec<&EnemyActionDef> = actions
        .iter()
        .filter(|a| match a.condition_type {
            0 => true,
            2 => {
                round >= a.condition_min
                    && (round - a.condition_min).is_multiple_of(a.condition_max.max(1))
            }
            3 => (a.condition_min..=a.condition_max).contains(&enemy_hp_pct),
            4 => (a.condition_min..=a.condition_max).contains(&party_hp_pct),
            5 => party_level >= a.condition_min,
            _ => false,
        })
        .collect();
    let best = eligible.iter().map(|a| a.priority).max()?;
    let top: Vec<&EnemyActionDef> = eligible
        .into_iter()
        .filter(|a| a.priority == best)
        .collect();
    top.get(roll as usize % top.len()).copied().cloned()
}

/// Map a chosen enemy `action` to a battle [`Command`] against `target` (a living
/// party member). A skill action (`kind == 1`) casts its `skill_id`; a basic
/// action defends (`2`), does nothing (observe/charge/wait — `3`/`4`/`7`), or
/// attacks (`0`/`1`, plus any unmodelled action — self-destruct, escape, or a
/// transform — which falls back to a plain attack). `None` (nothing eligible)
/// also attacks, so an enemy always acts.
pub fn enemy_command(action: Option<&EnemyActionDef>, target: usize) -> Command {
    match action {
        Some(a) if a.kind == 1 => Command::Skill {
            skill_id: a.skill_id,
            target,
        },
        Some(a) if a.basic == 2 => Command::Defend,
        Some(a) if matches!(a.basic, 3 | 4 | 7) => Command::Nothing,
        _ => Command::Attack { target },
    }
}

#[cfg(test)]
mod tests {
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
            physical_rate: 0,
            magical_rate: 3,
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
    fn critical_triples_the_base() {
        assert_eq!(critical_damage(12), 36);
        assert_eq!(critical_damage(0), 0);
    }

    #[test]
    fn skill_damage_adds_spirit_and_floors_positive_at_one() {
        assert_eq!(skill_damage(50, 20, 8), 58); // 50 + 10 - 2
        assert_eq!(skill_damage(10, 0, 400), 1); // reduced below 1, floored to 1
        assert_eq!(skill_damage(0, 999, 0), 0); // zero-power skill does nothing
    }

    #[test]
    fn variance_spans_plus_minus_ten_percent_deterministically() {
        assert_eq!(with_variance(100, 0), 90); // -10%
        assert_eq!(with_variance(100, 10), 100); // centre
        assert_eq!(with_variance(100, 20), 110); // +10%
        assert_eq!(with_variance(0, 5), 0); // zero stays zero
        assert_eq!(with_variance(3, 0), 3); // small hit rounds to no reduction
        assert_eq!(with_variance(1, 0), 1); // positive damage never drops below 1
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
    fn flee_chance_shifts_with_agility_gap_and_clamps() {
        assert_eq!(flee_chance(20, 10), 60); // +10 gap
        assert_eq!(flee_chance(0, 100), 25); // clamped low
        assert_eq!(flee_chance(200, 0), 90); // clamped high
        assert!(flee_succeeds(60, 59) && !flee_succeeds(60, 60));
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

    #[test]
    fn inflict_is_idempotent_and_cure_removes() {
        let mut states = vec![];
        inflict(&mut states, 3);
        inflict(&mut states, 3); // no duplicate
        inflict(&mut states, 5);
        assert_eq!(states, vec![3, 5]);
        cure(&mut states, 3);
        assert_eq!(states, vec![5]);
        cure(&mut states, 99); // absent -> no-op
        assert_eq!(states, vec![5]);
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
}

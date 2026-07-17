//! Pure battle mathematics: the RM2000-flavoured damage, turn-order, flee,
//! target and reward formulas, plus the derived party stat curve and the skill
//! filter. Kept free of Bevy and of the live battle state so every rule is
//! unit-testable in isolation; the battle systems are thin wrappers over these.

use amnezia_data::{MonsterDef, SkillDef};

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

/// RM2000-style physical damage: half the attacker's attack, less a quarter of
/// the defender's defense, never below zero.
pub fn physical_damage(attack: u32, defense: u32) -> i32 {
    (attack as i32 / 2 - defense as i32 / 4).max(0)
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
        }
    }

    #[test]
    fn physical_damage_is_half_attack_less_quarter_defense_floored_at_zero() {
        assert_eq!(physical_damage(40, 20), 15); // 20 - 5
        assert_eq!(physical_damage(20, 8), 8); // 10 - 2
        assert_eq!(physical_damage(4, 100), 0); // floored, never negative
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
}

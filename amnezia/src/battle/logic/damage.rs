//! The RM2000 damage formulas: physical and skill magnitude, the elemental
//! multiplier, criticals, variance, and the defending halving, plus the offensive
//! skill filter the command menu offers.

use super::Stats;
use amnezia_data::{AttributeDef, SkillDef};

/// RM2000-style physical damage: half the attacker's attack, less a quarter of
/// the defender's defense, never below zero.
pub fn physical_damage(attack: u32, defense: u32) -> i32 {
    (attack as i32 / 2 - defense as i32 / 4).max(0)
}

/// A critical hit's damage: RM2000 triples the blow.
pub fn critical_damage(base: i32) -> i32 {
    base * 3
}

/// Skill effect magnitude (RM2000 / EasyRPG `Algo::CalcSkillEffect`, pre-variance):
/// the skill's `power`, plus its physical/magical rates weighting the caster's
/// attack and spirit — `physical_rate * atk / 20 + magical_rate * spi / 40` — and,
/// when the skill targets enemies, less the target's defence and spirit —
/// `physical_rate * def / 40 + magical_rate * spi / 80`. Floored at 0. The
/// attribute (element) multiplier, an optional critical, and variance are applied
/// on top by the caller, in that order. `targets_enemies` is true for the
/// offensive scopes (one or all enemies) and false for the ally/heal scopes, which
/// take no defensive subtraction (the RM2000 `ignore_defense` flag is not
/// modelled, i.e. assumed false).
pub fn skill_effect(
    skill: &SkillDef,
    source: &Stats,
    target: &Stats,
    targets_enemies: bool,
) -> i32 {
    let mut effect = skill.power as i32
        + skill.physical_rate as i32 * source.attack as i32 / 20
        + skill.magical_rate as i32 * source.spirit as i32 / 40;
    if targets_enemies {
        effect -= skill.physical_rate as i32 * target.defense as i32 / 40;
        effect -= skill.magical_rate as i32 * target.spirit as i32 / 80;
    }
    effect.max(0)
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
/// weapon-strike resolution in [`crate::battle::resolve`].
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

/// Apply RM2000 / EasyRPG damage variance (`Algo::VarianceAdjustEffect`): with a
/// non-zero `var` and a positive `base`, the spread window is
/// `adj = max(1, var * base / 10)` and the result is
/// `base + rand(0..=adj) - adj / 2`, i.e. up to ±(var·10)% around `base`. The
/// caller owns the randomness and passes a raw `roll`; we take `roll % (adj + 1)`
/// for the inclusive `0..=adj` draw, keeping it one draw per hit. A `var` of 0 or
/// a non-positive `base` returns `base` unchanged (an immune 0-damage hit stays
/// 0). For a normal attack `var` is 4; for a skill it is [`SkillDef::variance`].
pub fn variance_adjust(base: i32, var: i32, roll: u64) -> i32 {
    if var > 0 && base > 0 {
        let adj = (var * base / 10).max(1);
        base + (roll % (adj as u64 + 1)) as i32 - adj / 2
    } else {
        base
    }
}

/// Halve incoming damage while defending (RM2000 Defend), rounding down.
pub fn defended(damage: i32) -> i32 {
    damage / 2
}

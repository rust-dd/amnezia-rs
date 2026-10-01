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

/// EasyRPG `Algo::CalcSkillEffect`, before attributes and variance (in that order).
/// Ally scopes and `ignore_defense` skip defensive subtraction; results floor at zero.
pub fn skill_effect(
    skill: &SkillDef,
    source: &Stats,
    target: &Stats,
    targets_enemies: bool,
) -> i32 {
    let mut effect = skill.power as i32
        + skill.physical_rate as i32 * source.attack as i32 / 20
        + skill.magical_rate as i32 * source.spirit as i32 / 40;
    if targets_enemies && !skill.ignore_defense {
        effect -= skill.physical_rate as i32 * target.defense as i32 / 40;
        effect -= skill.magical_rate as i32 * target.spirit as i32 / 80;
    }
    effect.max(0)
}

/// Attribute damage percentage for rank 0=A … 4=E; out-of-range ranks clamp to E.
pub fn attribute_percent(attr: &AttributeDef, rank: u8) -> u32 {
    match rank {
        0 => attr.a_rate,
        1 => attr.b_rate,
        2 => attr.c_rate,
        3 => attr.d_rate,
        _ => attr.e_rate,
    }
}

/// EasyRPG `Algo::VarianceAdjustEffect`, consuming one caller-supplied roll per hit.
/// Zero variance or non-positive damage stays unchanged; normal attacks use variance 4.
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

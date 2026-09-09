//! The to-hit percentages: a weapon's base rate and the agility-gap adjustment,
//! with the certain hit against a target that cannot act.

use amnezia_data::SkillDef;

/// An empty weapon slot uses RM2000's 90% base chance. An equipped weapon keeps
/// its configured rate, including an explicit zero.
pub fn effective_hit(weapon_hit: Option<u32>) -> u32 {
    weapon_hit.unwrap_or(90)
}

/// RPG_RT keys physical skill accuracy off the miss-message mode, not the
/// attack/spirit damage weights. Other skills keep their configured chance.
pub fn skill_to_hit(
    skill: &SkillDef,
    source_agi: u32,
    target_agi: u32,
    target_can_act: bool,
) -> i32 {
    if skill.failure_message == 3 && skill.scope < 2 {
        to_hit_vs(skill.hit, source_agi, target_agi, target_can_act)
    } else {
        skill.hit as i32
    }
}

/// Adjust a `base_hit` percentage by the agility gap between attacker and target
/// (RM2000 / EasyRPG `CalcToHitAgiAdjustment`): a faster target lowers the chance,
/// a slower one raises it, computed as
/// `100 - (100 - base_hit) * (1 + (target_agi / source_agi - 1) / 2)`. EasyRPG
/// runs this in `float` and truncates the result to an integer, so we mirror the
/// `f32` arithmetic and `as i32` truncation exactly; `source_agi` is guarded to at
/// least 1 to avoid a divide-by-zero. The result can dip below 0 (a certain miss)
/// but never exceeds 100, and a `base_hit` of 100 always yields 100 whatever the
/// agilities.
pub fn to_hit(base_hit: u32, source_agi: u32, target_agi: u32) -> i32 {
    let src = source_agi.max(1) as f32;
    let tgt = target_agi as f32;
    (100.0 - (100 - base_hit as i32) as f32 * (1.0 + (tgt / src - 1.0) / 2.0)) as i32
}

/// [`to_hit`], except a target that cannot act is struck with certainty: RM2000 /
/// EasyRPG `CalcNormalAttackToHit` and physical-mode `CalcSkillToHit` return 100
/// against a do-nothing target (asleep, paralyzed) before any agility adjustment.
/// `target_can_act` is false for such a target — its worst restriction is
/// "can't act" ([`super::worst_restriction`] `== 1`).
pub fn to_hit_vs(base_hit: u32, source_agi: u32, target_agi: u32, target_can_act: bool) -> i32 {
    if target_can_act {
        to_hit(base_hit, source_agi, target_agi)
    } else {
        100
    }
}

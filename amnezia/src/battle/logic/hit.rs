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
    state_hit_ratio: u32,
) -> i32 {
    if skill.failure_message == 3 && skill.scope < 2 {
        to_hit_vs(
            skill.hit * state_hit_ratio / 100,
            source_agi,
            target_agi,
            target_can_act,
        )
    } else {
        skill.hit as i32
    }
}

/// EasyRPG `CalcToHitAgiAdjustment`, preserving f32 arithmetic and integer truncation.
/// The result may be negative; base hit 100 stays certain regardless of agility.
pub fn to_hit(base_hit: u32, source_agi: u32, target_agi: u32) -> i32 {
    let src = source_agi.max(1) as f32;
    let tgt = target_agi as f32;
    (100.0 - (100 - base_hit as i32) as f32 * (1.0 + (tgt / src - 1.0) / 2.0)) as i32
}

/// Normal attacks and physical-mode skills hit a cannot-act target with certainty,
/// bypassing agility adjustment (EasyRPG `CalcNormalAttackToHit`/`CalcSkillToHit`).
pub fn to_hit_vs(base_hit: u32, source_agi: u32, target_agi: u32, target_can_act: bool) -> i32 {
    if target_can_act {
        to_hit(base_hit, source_agi, target_agi)
    } else {
        100
    }
}

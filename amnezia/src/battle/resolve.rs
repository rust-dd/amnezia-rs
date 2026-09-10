//! Battle resolution: apply the agility-ordered turn queue to the live
//! [`Battle`], one action at a time — physical hits, skill casts, item heals,
//! defence, and enemy attacks — then check for the end, tally rewards, and roll a
//! flee. Extends [`Battle`] with further `impl` blocks so the turn-flow half stays
//! in [`super::model`]; the damage numbers themselves come from [`super::logic`].
//!
//! Grouped by responsibility into submodules that each add methods to [`Battle`]:
//! [`step`] (the resolution engine and action dispatch), [`hit`] (the damage
//! application and feedback queues), [`strike`] (physical strikes), [`skill`]
//! (skill casts, party and enemy), [`item`] (item use), [`enemy`] (the enemy-AI
//! action pick, command flow, and retargeting), [`death`] (foe death-outs),
//! [`state`] (per-turn status HP change and recovery), and [`end`] (end checks,
//! rewards, and flee).

mod battler;
mod confused_attack;
mod death;
mod end;
mod enemy;
mod enemy_ai;
mod enemy_skill;
mod equipment;
mod hit;
mod item;
mod messages;
mod skill;
mod skill_heal;
mod skill_hit;
mod skill_pools;
mod skill_states;
mod skill_stats;
mod state;
mod state_change;
mod step;
mod strike;

#[cfg(test)]
mod tests;

pub(in crate::battle::resolve) use super::BattleOutcome;
pub(in crate::battle::resolve) use super::logic;
pub(in crate::battle::resolve) use super::model::{
    Action, Battle, BattleSe, Command, Dying, HitKind, HitReport, PendingAnim, Phase, Source, Step,
    rng_next,
};
pub(in crate::battle::resolve) use amnezia_data::SkillDef;

/// Internal report anchors for undrawn party members; these are not sprite positions.
const PARTY_ANIM_Y: f32 = 80.0;

/// Keep each undrawn party member's diagnostic anchor distinct.
const PARTY_ANIM_SPREAD: f32 = 16.0;

/// Seconds a slain foe blinks and fades out before it is cleared (RM2000
/// `SetDeathTimer(36)` counted down at 60 fps).
const DEATH_SECS: f32 = 36.0 / 60.0;

/// Seconds a self-destructing foe zoom-fades before it is cleared (RM2000
/// `SetExplodeTimer(20)` at 60 fps) — a shorter, punchier burst than a plain death.
const EXPLODE_SECS: f32 = 20.0 / 60.0;

/// The outcome of a party member's weapon strike: a clean miss, or a landed hit
/// carrying the damage dealt and whether it critical'd (for the log line).
#[derive(Clone, Copy)]
enum Strike {
    Miss,
    Hit { dmg: i32, crit: bool },
}

/// Diagnostic hit value; zero damage remains distinct from an evaded blow.
fn damage_report(dmg: i32) -> (String, HitKind) {
    if dmg > 0 {
        (dmg.to_string(), HitKind::Damage)
    } else {
        ("0".to_string(), HitKind::Miss)
    }
}

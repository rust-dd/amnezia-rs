//! Frame-accurate battle action planning, presentation and application.

mod battler;
mod death;
mod end;
mod enemy;
mod enemy_ai;
mod equipment;
mod hit;
mod messages;
mod plan;
mod skill_hit;
mod skill_pools;
mod state;
mod state_change;
pub(in crate::battle) mod timeline;

#[cfg(test)]
mod tests;

pub(in crate::battle::resolve) use super::BattleOutcome;
pub(in crate::battle::resolve) use super::logic;
use super::model::Phase;
pub(in crate::battle::resolve) use super::model::{
    Action, Battle, BattleSe, Command, Dying, HitKind, HitReport, PendingAnim, Source, rng_next,
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
#[cfg(test)]
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

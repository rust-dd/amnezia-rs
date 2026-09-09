//! Pure battle mathematics: the RM2000-flavoured damage, turn-order, flee,
//! target and reward formulas, plus the derived party stat curve and the skill
//! filter. Kept free of Bevy and of the live battle state so every rule is
//! unit-testable in isolation; the battle systems are thin wrappers over these.
//!
//! The rules are grouped by responsibility into submodules and re-exported here,
//! so callers keep using `logic::<name>` unchanged: [`stats`] (the party stat
//! curve and equipment bonuses), [`damage`] (the damage formulas and skill
//! filter), [`hit`] (to-hit), [`state`] (status effects), [`enemy`] (the enemy
//! AI), and [`escape`] (turn order, flee, and rewards).

mod damage;
mod enemy;
mod escape;
mod hit;
mod state;
mod stats;

#[cfg(test)]
mod tests;

pub(crate) use damage::{
    critical_damage, defended, elemental_damage, physical_damage, skill_effect, variance_adjust,
};
pub(crate) use enemy::{
    EnemyAiContext, check_turn, choose_enemy_action, enemy_command, hp_percent, select_target,
};
pub(crate) use escape::{
    average_agility, escape_succeeds, init_escape_chance, total_rewards, turn_order,
};
pub(crate) use hit::{effective_hit, skill_to_hit, to_hit_vs};
pub(crate) use state::{
    cure, has_state, inflict, release_on_damage, state_hp_delta, state_infliction_chance,
    tick_recovery, worst_restriction,
};
#[cfg(test)]
pub(crate) use stats::equipment_bonus;
pub(crate) use stats::{
    Stats, actor_hp_sp_at, actor_stats_at, equipment_bonus_slots, equipment_resist_slots,
    equipment_state_guards,
};

pub(super) use super::model::Command;

#[cfg(test)]
pub(crate) use amnezia_data::{
    ActorCurves, AttributeDef, EnemyActionDef, ItemDef, SkillDef, StateDef,
};
#[cfg(test)]
pub(crate) use damage::attribute_percent;
#[cfg(test)]
pub(crate) use hit::to_hit;
#[cfg(test)]
pub(crate) use stats::actor_stats;

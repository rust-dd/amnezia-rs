//! RM2000 battle formulas, independent of Bevy and live battle state.

mod attribute;
mod critical;
mod damage;
mod enemy;
mod escape;
mod hit;
mod skill_requirements;
mod state;
mod state_effects;
mod stats;

#[cfg(test)]
mod tests;

pub(crate) use attribute::attribute_damage;
pub(crate) use critical::critical_chance;
pub(crate) use damage::{
    critical_damage, defended, physical_damage, skill_effect, variance_adjust,
};
pub(crate) use enemy::{
    EnemyAiContext, check_turn, choose_enemy_action, enemy_command, hp_percent, select_target,
};
pub(crate) use escape::{
    average_agility, escape_succeeds, init_escape_chance, total_rewards, turn_order,
};
pub(crate) use hit::{effective_hit, skill_to_hit, to_hit_vs};
pub(crate) use skill_requirements::weapon_allows_skill;
pub(crate) use state::{
    cure, has_state, inflict, release_on_damage, state_hp_delta, state_infliction_chance,
    state_sp_delta, tick_recovery, worst_restriction,
};
pub(crate) use state_effects::{
    inflict_with_priority, state_hit_ratio, state_stats, states_allow_skill,
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

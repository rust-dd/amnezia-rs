//! Status-effect rules: infliction chance, the active-state list helpers, the
//! worst restriction, the turn- and damage-based wear-off, and per-turn HP change.

use amnezia_data::StateDef;

pub fn state_infliction_chance(state: &StateDef, rank: u8) -> u32 {
    state.rates.get(rank as usize).copied().unwrap_or(0)
}

/// The status id (RM2000 state 1) that marks a KO'd combatant. It is exempt from
/// turn- and damage-based recovery: whether a combatant is down is governed by HP
/// and [`crate::battle::model::Fighter::alive`], not by a wear-off roll.
pub const DEATH_STATE: u32 = 1;

/// Whether `state_id` is active in `states` (a `(state_id, turns_held)` list).
pub fn has_state(states: &[(u32, u32)], state_id: u32) -> bool {
    states.iter().any(|&(id, _)| id == state_id)
}

/// Add `state_id` to an active-state list (held for `0` turns) if it is not
/// already present, so infliction stays idempotent (RM2000 never stacks a state).
pub fn inflict(states: &mut Vec<(u32, u32)>, state_id: u32) {
    if !has_state(states, state_id) {
        states.push((state_id, 0));
    }
}

/// Remove `state_id` from an active-state list — a cure or a wear-off.
pub fn cure(states: &mut Vec<(u32, u32)>, state_id: u32) {
    states.retain(|&(id, _)| id != state_id);
}

/// Cannot act takes precedence over attack-enemy, then attack-ally restrictions.
pub fn worst_restriction(states: &[(u32, u32)], defs: &[StateDef]) -> u32 {
    states
        .iter()
        .filter_map(|&(id, _)| defs.iter().find(|d| d.id == id).map(|d| d.restriction))
        .filter(|restriction| matches!(restriction, 1..=3))
        .min()
        .unwrap_or(0)
}

/// Recover immediately before this battler's action, after its full hold count.
/// Stored counts start at zero (the reference's active state counter starts at one).
pub fn tick_recovery(
    states: &mut Vec<(u32, u32)>,
    defs: &[StateDef],
    mut roll: impl FnMut() -> u32,
) -> Vec<u32> {
    let mut lifted = Vec::new();
    states.retain_mut(|(id, turns)| {
        if *id == DEATH_STATE {
            return true;
        }
        let Some(def) = defs.iter().find(|d| d.id == *id) else {
            return true;
        };
        if *turns >= def.hold_turn && def.auto_release_prob > 0 && roll() < def.auto_release_prob {
            lifted.push(*id);
            false
        } else {
            *turns = turns.saturating_add(1);
            true
        }
    });
    lifted
}

/// Damage recovery chance is scaled by the attack's physical percentage.
/// Zero chance and the death state do not consume a recovery roll.
pub fn release_on_damage(
    states: &mut Vec<(u32, u32)>,
    defs: &[StateDef],
    physical_rate: u32,
    mut roll: impl FnMut() -> u32,
) -> Vec<u32> {
    let mut lifted = Vec::new();
    states.retain_mut(|(id, _)| {
        if *id == DEATH_STATE {
            return true;
        }
        let Some(def) = defs.iter().find(|d| d.id == *id) else {
            return true;
        };
        let chance = def.release_by_damage.saturating_mul(physical_rate) / 100;
        if chance > 0 && roll() < chance {
            lifted.push(*id);
            false
        } else {
            true
        }
    });
    lifted
}

/// Flat change plus the floored percentage of the maximum pool.
pub fn state_hp_delta(def: &StateDef, max_hp: i32) -> i32 {
    state_pool_delta(
        def.hp_change_type,
        def.hp_change_val,
        def.hp_change_max,
        max_hp,
    )
}

pub fn state_sp_delta(def: &StateDef, max_sp: i32) -> i32 {
    state_pool_delta(
        def.sp_change_type,
        def.sp_change_val,
        def.sp_change_max,
        max_sp,
    )
}

fn state_pool_delta(kind: u32, flat: u32, percent: u32, max: i32) -> i32 {
    let amount = (i64::from(flat) + i64::from(max) * i64::from(percent) / 100)
        .clamp(0, i64::from(i32::MAX)) as i32;
    match kind {
        0 => -amount,
        1 => amount,
        _ => 0,
    }
}

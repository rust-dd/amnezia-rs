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

/// The highest `restriction` among the combatant's active `states` — `0` none,
/// `1` can't act, `2` attack-enemy (berserk), `3` attack-ally (confusion) — or `0`
/// when it bears no restricting state. The worst restriction governs how the actor
/// is forced to behave this round; ids absent from `defs` contribute nothing.
pub fn worst_restriction(states: &[(u32, u32)], defs: &[StateDef]) -> u32 {
    states
        .iter()
        .filter_map(|&(id, _)| defs.iter().find(|d| d.id == id).map(|d| d.restriction))
        .max()
        .unwrap_or(0)
}

/// Advance every active state's held-turn count and roll its automatic wear-off:
/// once a state has been held at least `hold_turn` rounds it lifts on a `roll() <
/// auto_release_prob` (percent) draw. The death state ([`DEATH_STATE`]) never wears
/// off. `roll` yields a fresh `0..100` value, consulted only for a state eligible
/// to lift. Returns the ids that lifted, for the caller to log. Run once per
/// combatant at the top of each round.
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
        *turns = turns.saturating_add(1);
        let Some(def) = defs.iter().find(|d| d.id == *id) else {
            return true;
        };
        if *turns >= def.hold_turn && roll() < def.auto_release_prob {
            lifted.push(*id);
            false
        } else {
            true
        }
    });
    lifted
}

/// Roll each active state's damage wear-off after its bearer is struck: a state
/// lifts on a `roll() < release_by_damage` (percent) draw. The death state
/// ([`DEATH_STATE`]) never wears off, and a state that cannot be shaken by damage
/// (`release_by_damage == 0`) is left untouched with no roll spent. `roll` yields a
/// fresh `0..100` value per eligible state. Returns the ids that lifted, to log.
pub fn release_on_damage(
    states: &mut Vec<(u32, u32)>,
    defs: &[StateDef],
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
        if def.release_by_damage > 0 && roll() < def.release_by_damage {
            lifted.push(*id);
            false
        } else {
            true
        }
    });
    lifted
}

/// A state's per-turn HP change (RM2000 `hp_change`), returned already signed by
/// its `hp_change_type`: a negative drain for type `0`, a positive regen for type
/// `1`, and `0` for type `2` (nothing) or an unconfigured state. The magnitude is
/// `hp_change_val + max_hp * hp_change_max / 100`; a state that is *configured* to
/// change HP (either amount non-zero) always moves at least one point (RM2000
/// floors an afflicted battler's loss/gain at 1), while a state with both amounts
/// zero — every non-poison state, whose `hp_change_type` defaults to `0` — is left
/// untouched. The map-only fields (`hp_change_map_*`) are out of scope here: they
/// drain on the overworld, not per battle turn. Applied at the start of a
/// battler's turn by [`crate::battle::resolve`].
pub fn state_hp_delta(def: &StateDef, max_hp: i32) -> i32 {
    if def.hp_change_val == 0 && def.hp_change_max == 0 {
        return 0;
    }
    let magnitude = (def.hp_change_val as i32 + max_hp * def.hp_change_max as i32 / 100).max(1);
    match def.hp_change_type {
        0 => -magnitude,
        1 => magnitude,
        _ => 0,
    }
}

//! The enemy battle AI: HP-percent gating, the condition-scored action pick, the
//! mapping to a battle [`Command`], and the random living-target selector.

use super::Command;
use amnezia_data::EnemyActionDef;

/// Pick the index of the `roll`-th living combatant among `alive`, wrapping. The
/// enemy AI uses it to choose a random living party target; `None` when every
/// flag is false.
pub fn select_target(alive: &[bool], roll: usize) -> Option<usize> {
    let living: Vec<usize> = alive
        .iter()
        .enumerate()
        .filter(|&(_, &a)| a)
        .map(|(i, _)| i)
        .collect();
    living.get(roll % living.len().max(1)).copied()
}

/// The party level the enemy AI assumes: battle state tracks no per-member
/// level, so party-level conditions (`condition_type == 5`) test against this
/// conservative floor rather than a real average — a documented approximation.
pub const AI_PARTY_LEVEL: u32 = 1;

/// The integer percentage (`0..=100`, saturating) `current` is of `max`, used to
/// gate the HP-conditioned enemy-AI actions. A non-positive `max` yields `0`, and
/// a negative `current` (an overkilled combatant) clamps to `0`.
pub fn hp_percent(current: i32, max: i32) -> u32 {
    if max <= 0 {
        return 0;
    }
    (current.max(0) as i64 * 100 / max as i64) as u32
}

/// Pick an enemy's action this turn from its RM2000 AI list, honouring each
/// entry's condition gate. Eligibility by `condition_type`:
/// - `0` always — always eligible.
/// - `2` turn — from `condition_min` on, every `condition_max` rounds:
///   `round >= condition_min && (round - condition_min) % condition_max.max(1) == 0`.
/// - `3` monster-hp% — `enemy_hp_pct` within `[condition_min, condition_max]`.
/// - `4` party-hp% — `party_hp_pct` within `[condition_min, condition_max]`.
/// - `5` party level — `party_level >= condition_min`.
///
/// `1` switch and `6` party-exhausted are treated as never holding: there is no
/// in-battle switch access, and the pure inputs carry no per-member SP to detect
/// an exhausted (0-SP) member. Among the eligible actions the highest `priority`
/// wins; ties are broken by `roll`. Returns `None` when nothing is eligible (the
/// caller then falls back to a basic attack), so an empty or fully-gated list
/// still yields a fight.
pub fn choose_enemy_action(
    actions: &[EnemyActionDef],
    enemy_hp_pct: u32,
    party_hp_pct: u32,
    party_level: u32,
    round: u32,
    roll: u64,
) -> Option<EnemyActionDef> {
    let eligible: Vec<&EnemyActionDef> = actions
        .iter()
        .filter(|a| match a.condition_type {
            0 => true,
            2 => {
                round >= a.condition_min
                    && (round - a.condition_min).is_multiple_of(a.condition_max.max(1))
            }
            3 => (a.condition_min..=a.condition_max).contains(&enemy_hp_pct),
            4 => (a.condition_min..=a.condition_max).contains(&party_hp_pct),
            5 => party_level >= a.condition_min,
            _ => false,
        })
        .collect();
    let best = eligible.iter().map(|a| a.priority).max()?;
    let top: Vec<&EnemyActionDef> = eligible
        .into_iter()
        .filter(|a| a.priority == best)
        .collect();
    top.get(roll as usize % top.len()).copied().cloned()
}

/// Map a chosen enemy `action` to a battle [`Command`] against `target` (a living
/// party member). A skill action (`kind == 1`) casts its `skill_id`; a basic
/// action maps by its RM2000 `basic` code: `0` attack, `1` double-attack, `2`
/// defend, `3`/`7` observe/do-nothing (a no-op [`Command::Nothing`]), `4`
/// charge-up, `5` self-destruct, `6` escape. An unknown basic — and `None`
/// (nothing eligible) — falls back to a plain attack, so an enemy always acts.
pub fn enemy_command(action: Option<&EnemyActionDef>, target: usize) -> Command {
    match action {
        Some(a) if a.kind == 1 => Command::Skill {
            skill_id: a.skill_id,
            target,
        },
        Some(a) => match a.basic {
            1 => Command::DoubleAttack { target },
            2 => Command::Defend,
            3 | 7 => Command::Nothing,
            4 => Command::Charge,
            5 => Command::SelfDestruct,
            6 => Command::Escape,
            _ => Command::Attack { target },
        },
        None => Command::Attack { target },
    }
}

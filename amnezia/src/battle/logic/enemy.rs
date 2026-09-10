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

/// Live quantities used by the original enemy condition codes.
#[derive(Default, Clone, Copy)]
pub struct EnemyAiContext {
    pub turn: u32,
    pub enemies: u32,
    pub hp: u32,
    pub sp: u32,
    pub level: u32,
    pub fatigue: u32,
}

pub fn hp_percent(current: i32, max: i32) -> u32 {
    if max <= 0 {
        return 0;
    }
    (i64::from(current.max(0)) * 100 / i64::from(max)) as u32
}

pub fn check_turn(turn: u32, base: u32, multiple: u32) -> bool {
    turn >= base
        && if multiple == 0 {
            turn == base
        } else {
            (turn - base).is_multiple_of(multiple)
        }
}

fn condition(
    action: &EnemyActionDef,
    context: &EnemyAiContext,
    switch: impl Fn(u32) -> bool,
) -> bool {
    let range = action.condition_min..=action.condition_max;
    match action.condition_type {
        0 => true,
        1 => switch(action.switch_id),
        2 => check_turn(context.turn, action.condition_max, action.condition_min),
        3 => range.contains(&context.enemies),
        4 => range.contains(&context.hp),
        5 => range.contains(&context.sp),
        6 => range.contains(&context.level),
        7 => range.contains(&context.fatigue),
        _ => true,
    }
}

/// Target availability is filtered after the rating cutoff, as in RPG_RT.
pub fn choose_enemy_action(
    actions: &[EnemyActionDef],
    context: &EnemyAiContext,
    usable: impl Fn(&EnemyActionDef) -> bool,
    effective: impl Fn(&EnemyActionDef) -> bool,
    switch: impl Fn(u32) -> bool,
    roll: u64,
) -> Option<EnemyActionDef> {
    let ratings = actions
        .iter()
        .map(|action| {
            if usable(action) && condition(action, context, &switch) {
                action.priority
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let highest = u64::from(*ratings.iter().max()?);
    let weights = actions
        .iter()
        .zip(ratings)
        .map(|(action, rating)| {
            if rating == 0 || !effective(action) {
                0
            } else {
                (u64::from(rating) + 10).saturating_sub(highest)
            }
        })
        .collect::<Vec<_>>();
    let total = weights.iter().sum::<u64>();
    if total == 0 {
        return None;
    }
    let mut choice = roll % total;
    for (action, weight) in actions.iter().zip(weights) {
        if choice < weight {
            return Some(action.clone());
        }
        choice -= weight;
    }
    None
}

/// Map RM2000 basic and skill actions; an empty selection does nothing.
pub fn enemy_command(action: Option<&EnemyActionDef>, target: usize) -> Command {
    match action {
        Some(a) if a.kind == 1 => Command::Skill {
            skill_id: a.skill_id,
            target,
        },
        Some(a) => match a.basic {
            1 => Command::DoubleAttack { target },
            2 => Command::Defend,
            3 => Command::Observe,
            7 => Command::Nothing,
            4 => Command::Charge,
            5 => Command::SelfDestruct,
            6 => Command::Escape,
            _ => Command::Attack { target },
        },
        None => Command::Nothing,
    }
}

#[cfg(test)]
mod tests;

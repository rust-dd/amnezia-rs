use super::{Stats, has_state, inflict};
use amnezia_data::{SkillDef, StateDef};

pub fn state_stats(stats: Stats, active: &[(u32, u32)], defs: &[StateDef]) -> Stats {
    let mut half = [false; 4];
    let mut double = [false; 4];
    for state in defs.iter().filter(|state| has_state(active, state.id)) {
        for index in 0..4 {
            if state.affect_stats[index] {
                half[index] |= state.affect_type == 0;
                double[index] |= state.affect_type == 1;
            }
        }
    }
    let values = [stats.attack, stats.defense, stats.spirit, stats.agility];
    let [attack, defense, spirit, agility] =
        std::array::from_fn(|index| match (half[index], double[index]) {
            (true, false) => (values[index] / 2).max(1),
            (false, true) => values[index] * 2,
            _ => values[index],
        });
    Stats {
        attack,
        defense,
        spirit,
        agility,
    }
}

pub fn state_hit_ratio(active: &[(u32, u32)], defs: &[StateDef]) -> u32 {
    defs.iter()
        .filter(|state| has_state(active, state.id))
        .map(|state| state.reduce_hit_ratio)
        .min()
        .unwrap_or(100)
        .min(100)
}

pub fn states_allow_skill(active: &[(u32, u32)], defs: &[StateDef], skill: &SkillDef) -> bool {
    !defs
        .iter()
        .filter(|state| has_state(active, state.id))
        .any(|state| {
            (state.restrict_skill && skill.physical_rate >= state.restrict_skill_level)
                || (state.restrict_magic && skill.magical_rate >= state.restrict_magic_level)
        })
}

pub fn inflict_with_priority(active: &mut Vec<(u32, u32)>, id: u32, defs: &[StateDef]) -> bool {
    if !defs.iter().any(|state| state.id == id) {
        return false;
    }
    inflict(active, id);
    let highest = defs
        .iter()
        .filter(|state| has_state(active, state.id))
        .map(|state| state.priority)
        .max()
        .unwrap_or(0);
    active.retain(|(id, _)| {
        defs.iter()
            .find(|state| state.id == *id)
            .is_some_and(|state| state.priority.saturating_add(10) > highest)
    });
    has_state(active, id)
}

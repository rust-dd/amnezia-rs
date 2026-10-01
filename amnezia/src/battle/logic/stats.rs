//! Party-member battle stats: the level/curve-derived base and the equipment
//! bonuses layered on top, plus the enemy stats read straight from a monster.

#[cfg(test)]
use amnezia_data::ActorDef;
use amnezia_data::{ActorCurves, ItemDef, MonsterDef};

/// Combat stats from actor curves or enemy definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub attack: u32,
    pub defense: u32,
    pub spirit: u32,
    pub agility: u32,
}

impl Stats {
    /// Monster `m`'s stats, read directly from its definition.
    pub fn from_monster(m: &MonsterDef) -> Self {
        Self {
            attack: m.attack,
            defense: m.defense,
            spirit: m.spirit,
            agility: m.agility,
        }
    }
}

/// Approximate fallback for legacy assets without stat curves; not RM2000 curve parity.
pub fn actor_stats(level: u32) -> Stats {
    Stats {
        attack: 16 + level * 6,
        defense: 8 + level * 4,
        spirit: 8 + level * 3,
        agility: 8 + level * 2,
    }
}

/// Read level L at curve index L-1; empty legacy curves use [`actor_stats`].
pub fn actor_stats_at(curves: &ActorCurves, level: u32) -> Stats {
    let i = (level.max(1) - 1) as usize;
    match (
        curves.attack.get(i),
        curves.defense.get(i),
        curves.spirit.get(i),
        curves.agility.get(i),
    ) {
        (Some(&attack), Some(&defense), Some(&spirit), Some(&agility)) => Stats {
            attack,
            defense,
            spirit,
            agility,
        },
        _ => actor_stats(level),
    }
}

/// A party member's max HP/SP at `level` from their curve, falling back to
/// `(fallback_hp, fallback_sp)` when the curve is empty.
pub fn actor_hp_sp_at(
    curves: &ActorCurves,
    level: u32,
    fallback_hp: u32,
    fallback_sp: u32,
) -> (u32, u32) {
    let i = (level.max(1) - 1) as usize;
    let hp = curves.max_hp.get(i).copied().unwrap_or(fallback_hp);
    let sp = curves.max_sp.get(i).copied().unwrap_or(fallback_sp);
    (hp, sp)
}

/// Starting-equipment fixture; runtime battles use [`equipment_bonus_slots`].
#[cfg(test)]
pub fn equipment_bonus(actor: &ActorDef, items: &[ItemDef]) -> Stats {
    equipment_bonus_slots(actor_slots(actor), items)
}

/// Sum bonuses in weapon/shield/armor/helmet/accessory order; empty or missing IDs add nothing.
pub fn equipment_bonus_slots(slots: [u32; 5], items: &[ItemDef]) -> Stats {
    let mut bonus = Stats::default();
    for id in slots {
        if id == 0 {
            continue;
        }
        if let Some(item) = items.iter().find(|i| i.id == id) {
            bonus.attack += item.atk;
            bonus.defense += item.def;
            bonus.spirit += item.spi;
            bonus.agility += item.agi;
        }
    }
    bonus
}

/// Only defensive gear grants an attribute rank boost; multiple pieces do not stack.
pub fn equipment_resist_slots(slots: [u32; 5], items: &[ItemDef]) -> Vec<u32> {
    let mut resist = Vec::<u32>::new();
    for id in slots {
        if id == 0 {
            continue;
        }
        if let Some(item) = items.iter().find(|i| i.id == id) {
            if !matches!(item.item_type, 2..=5) {
                continue;
            }
            for &attr in &item.attribute_defense {
                if !resist.contains(&attr) {
                    resist.push(attr);
                }
            }
        }
    }
    resist
}

/// Only the strongest armor resistance applies; weapon states are offensive.
pub fn equipment_state_guards(slots: [u32; 5], items: &[ItemDef]) -> Vec<(u32, u32)> {
    let mut guards = std::collections::BTreeMap::<u32, u32>::new();
    for item in slots
        .into_iter()
        .filter_map(|id| items.iter().find(|item| id != 0 && item.id == id))
    {
        if !matches!(item.item_type, 2..=5) {
            continue;
        }
        for &state in &item.state_defense {
            let multiplier = 100_u32.saturating_sub(item.state_chance);
            guards
                .entry(state)
                .and_modify(|rate| *rate = (*rate).min(multiplier))
                .or_insert(multiplier);
        }
    }
    guards.into_iter().collect()
}

/// An actor's starting five-slot loadout in slot order (weapon, shield, armor,
/// helmet, accessory), the fallback when no runtime loadout is supplied.
#[cfg(test)]
fn actor_slots(actor: &ActorDef) -> [u32; 5] {
    [
        actor.weapon,
        actor.shield,
        actor.armor,
        actor.helmet,
        actor.accessory,
    ]
}

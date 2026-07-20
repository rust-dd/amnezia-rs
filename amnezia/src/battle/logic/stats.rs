//! Party-member battle stats: the level/curve-derived base and the equipment
//! bonuses layered on top, plus the enemy stats read straight from a monster.

use amnezia_data::{ActorCurves, ActorDef, ItemDef, MonsterDef};

/// A combatant's four battle stats. Enemies read them straight from their
/// [`MonsterDef`]; party members, whose `ActorDef` carries only a level, get
/// them from [`actor_stats`].
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

/// Derive a party member's battle stats from their `level`. The converted
/// `ActorDef` carries no combat stats (only level and HP/SP), so v1 grows them on
/// a simple linear curve tuned so an early hero trades a handful of blows with the
/// early troops. A documented approximation, not RM2000 stat-curve parity.
pub fn actor_stats(level: u32) -> Stats {
    Stats {
        attack: 16 + level * 6,
        defense: 8 + level * 4,
        spirit: 8 + level * 3,
        agility: 8 + level * 2,
    }
}

/// A party member's battle stats at `level`, read from their stat curve (level L
/// at index L-1). Falls back to the level formula when the curve is empty (e.g. a
/// stale `actors.ron` predating the curve fields).
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

/// The stat bonus an actor's five equipment slots (weapon, shield, armor, helmet,
/// accessory) add on top of the curve-derived base: the summed `atk`/`def`/`spi`/
/// `agi` of each equipped item. Reads the actor's *starting* gear; the battle
/// builds from the runtime loadout via [`equipment_bonus_slots`].
pub fn equipment_bonus(actor: &ActorDef, items: &[ItemDef]) -> Stats {
    equipment_bonus_slots(actor_slots(actor), items)
}

/// The stat bonus of an explicit five-slot loadout (weapon, shield, armor,
/// helmet, accessory): the summed `atk`/`def`/`spi`/`agi` of each equipped item.
/// Empty slots (id `0`) and ids absent from `items` contribute nothing. This is
/// the runtime-equipment entry point [`crate::battle::model::Battle::build`] uses.
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

/// The 1-based attribute (element) ids an explicit five-slot loadout guards
/// against: the de-duplicated union of each equipped item's `attribute_defense`.
/// Empty slots (id `0`) and ids absent from `items` contribute nothing. Consumed
/// by [`crate::battle::resolve`] to halve a matching enemy skill's damage against
/// the wearer.
pub fn equipment_resist_slots(slots: [u32; 5], items: &[ItemDef]) -> Vec<u32> {
    let mut resist: Vec<u32> = Vec::new();
    for id in slots {
        if id == 0 {
            continue;
        }
        if let Some(item) = items.iter().find(|i| i.id == id) {
            for &attr in &item.attribute_defense {
                if !resist.contains(&attr) {
                    resist.push(attr);
                }
            }
        }
    }
    resist
}

/// An actor's starting five-slot loadout in slot order (weapon, shield, armor,
/// helmet, accessory), the fallback when no runtime loadout is supplied.
fn actor_slots(actor: &ActorDef) -> [u32; 5] {
    [
        actor.weapon,
        actor.shield,
        actor.armor,
        actor.helmet,
        actor.accessory,
    ]
}

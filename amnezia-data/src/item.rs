//! Item definitions shared by the converter, field menus, and battle system.

use serde::{Deserialize, Serialize};

/// Item effects and equipment bonuses. Recovery combines a fixed amount with
/// a `_rate` percentage of the maximum; attribute/state IDs are 1-based.
/// `hit` and `crit` are percentages; `ko_only` limits effects to fallen actors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDef {
    #[serde(default)]
    pub prevent_critical: bool,
    #[serde(default)]
    pub raise_evasion: bool,
    #[serde(default)]
    pub half_sp_cost: bool,
    #[serde(default)]
    pub actor_set: Vec<bool>,
    #[serde(default)]
    pub state_chance: u32,
    pub id: u32,
    pub name: String,
    pub description: String,
    /// 0 = normal, 1 = weapon, 2 = shield, 3 = armor, 4 = helmet, 5 = accessory,
    /// 6 = medicine, 7 = book, 8 = material, 9 = special, 10 = switch.
    pub item_type: u32,
    pub price: u32,
    #[serde(default)]
    pub recover_hp: u32,
    #[serde(default)]
    pub recover_hp_rate: u32,
    #[serde(default)]
    pub recover_sp: u32,
    #[serde(default)]
    pub recover_sp_rate: u32,
    #[serde(default)]
    pub cure_states: Vec<u32>,
    /// 0 = one ally, 1 = whole party.
    #[serde(default)]
    pub scope: u32,
    #[serde(default)]
    pub only_field: bool,
    #[serde(default)]
    pub ko_only: bool,
    /// Uses before consumption; 0 means unlimited.
    #[serde(default = "default_one")]
    pub uses: u32,
    #[serde(default)]
    pub atk: u32,
    #[serde(default)]
    pub def: u32,
    #[serde(default)]
    pub spi: u32,
    #[serde(default)]
    pub agi: u32,
    #[serde(default)]
    pub attribute_defense: Vec<u32>,
    #[serde(default)]
    pub state_defense: Vec<u32>,
    #[serde(default)]
    pub two_handed: bool,
    #[serde(default = "default_hit")]
    pub hit: u32,
    #[serde(default)]
    pub crit: u32,
    #[serde(default = "default_one")]
    pub weapon_animation: u32,
}

fn default_one() -> u32 {
    1
}

impl ItemDef {
    /// Omitted trailing actor flags mean allowed, unlike attribute/state sets.
    pub fn usable_by_actor(&self, actor_id: u32) -> bool {
        actor_id
            .checked_sub(1)
            .is_some_and(|index| self.actor_set.get(index as usize).copied().unwrap_or(true))
    }
}

fn default_hit() -> u32 {
    90
}

//! Item definitions shared by the converter, field menus, and battle system.

use serde::{Deserialize, Serialize};

/// An item's definition, read by the shop, item, and equip menus and the
/// use-item systems: its 1-based id, name, description, category (`item_type`:
/// 0 normal, 1 weapon, 2 shield, 3 armor, 4 helmet, 5 accessory, 6 medicine,
/// 7 book, 8 material, 9 special, 10 switch), and buy price.
///
/// The use-effect fields apply when the item is consumed (a medicine, or a
/// normal item used from the menu): `recover_hp`/`recover_sp` restore a fixed
/// amount, `recover_hp_rate`/`recover_sp_rate` a percentage of the maximum,
/// `cure_states` lists the 1-based state ids it lifts, `scope` targets one ally
/// (`0`) or the whole party (`1`), `only_field` marks it usable only from the
/// map menu, and `ko_only` restricts effects to fallen actors. `uses` is the
/// number of uses before consumption (default `1`, explicitly `0` = unlimited).
///
/// The equipment fields apply to gear (types 1–5): `atk`/`def`/`spi`/`agi` are
/// the stat bonuses, `attribute_defense`/`state_defense` the 1-based attribute
/// and state ids the armor guards against (or the weapon inflicts), `two_handed` marks a
/// two-handed weapon, `hit`/`crit` its hit and critical rates (percent), and
/// `weapon_animation` its attack animation id.
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
    #[serde(default)]
    pub scope: u32,
    #[serde(default)]
    pub only_field: bool,
    #[serde(default)]
    pub ko_only: bool,
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

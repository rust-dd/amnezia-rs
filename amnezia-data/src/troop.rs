use crate::EventCommand;
use serde::{Deserialize, Serialize};

/// One enemy's database ID and its position in the 320×240 battle backdrop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TroopMemberDef {
    pub enemy_id: u32,
    pub x: u32,
    pub y: u32,
}

/// A battle encounter and its event pages, in original evaluation order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TroopDef {
    pub id: u32,
    pub name: String,
    pub members: Vec<TroopMemberDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<TroopPageDef>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TroopPageDef {
    pub condition: TroopPageConditionDef,
    pub commands: Vec<EventCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TroopPageConditionDef {
    /// Bits 0–6 enable switch A, switch B, variable, turn, fatigue, enemy HP and actor HP.
    pub flags: u32,
    pub switch_a_id: u32,
    pub switch_b_id: u32,
    pub variable_id: u32,
    pub variable_value: i32,
    pub turn_a: u32,
    pub turn_b: u32,
    pub fatigue_min: u32,
    pub fatigue_max: u32,
    /// Zero-based slot within this troop, not a monster database ID.
    pub enemy_index: u32,
    pub enemy_hp_min: u32,
    pub enemy_hp_max: u32,
    pub actor_id: u32,
    pub actor_hp_min: u32,
    pub actor_hp_max: u32,
}

impl Default for TroopPageConditionDef {
    fn default() -> Self {
        Self {
            flags: 0,
            switch_a_id: 1,
            switch_b_id: 1,
            variable_id: 1,
            variable_value: 0,
            turn_a: 0,
            turn_b: 0,
            fatigue_min: 0,
            fatigue_max: 100,
            enemy_index: 0,
            enemy_hp_min: 0,
            enemy_hp_max: 100,
            actor_id: 1,
            actor_hp_min: 0,
            actor_hp_max: 100,
        }
    }
}

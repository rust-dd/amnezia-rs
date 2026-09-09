use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerrainDef {
    pub id: u32,
    pub name: String,
    pub damage: i32,
    pub encounter_rate: u32,
    pub background_name: String,
    pub boat_pass: bool,
    pub ship_pass: bool,
    pub airship_pass: bool,
    pub airship_land: bool,
    pub bush_depth: u32,
}

impl Default for TerrainDef {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            damage: 0,
            encounter_rate: 100,
            background_name: String::new(),
            boat_pass: false,
            ship_pass: false,
            airship_pass: true,
            airship_land: true,
            bush_depth: 0,
        }
    }
}

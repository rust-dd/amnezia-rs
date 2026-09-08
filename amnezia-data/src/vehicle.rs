use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleDef {
    pub charset: String,
    pub index: u32,
    pub map_id: u32,
    pub x: u32,
    pub y: u32,
}

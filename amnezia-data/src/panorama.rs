use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanoramaDef {
    pub name: String,
    pub loop_x: bool,
    pub loop_y: bool,
    pub auto_x: bool,
    pub auto_y: bool,
    pub speed_x: i32,
    pub speed_y: i32,
}

impl PanoramaDef {
    pub fn from_command(name: String, params: &[i32]) -> Self {
        let at = |i| params.get(i).copied().unwrap_or(0);
        Self {
            name,
            loop_x: at(0) != 0,
            loop_y: at(1) != 0,
            auto_x: at(2) != 0,
            speed_x: at(3),
            auto_y: at(4) != 0,
            speed_y: at(5),
        }
    }
}

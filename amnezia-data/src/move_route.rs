//! Engine-agnostic RM2000 routes, shared by custom-movement pages and `MoveEvent`.

use serde::{Deserialize, Serialize};

/// One move-route command: its RM2000 `code`, the code-specific integer
/// parameters, and the file-name `string` a change-graphic (34) or
/// play-sound-effect (35) command carries (empty otherwise).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveCommandDef {
    pub code: u32,
    pub params: Vec<i32>,
    pub string: String,
}

/// A move route: the ordered commands, whether it loops (`repeat`), and whether a
/// blocked step is skipped rather than waited on (`skippable`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRouteDef {
    pub commands: Vec<MoveCommandDef>,
    pub repeat: bool,
    pub skippable: bool,
}

impl MoveRouteDef {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

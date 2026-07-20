//! The clean, engine-agnostic form of an RM2000 move route (see `lcf::MoveRoute`):
//! a `repeat`/`skippable` pair and an ordered list of [`MoveCommandDef`]s. An
//! event page follows its route when its `move_type` is 6 (custom route), and the
//! `MoveEvent` command carries an inline route of the same shape; the game's route
//! stepper interprets the command codes.

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
    /// Whether the route carries no commands (the default for every page that is
    /// not a custom route, and for a graphic-less door page).
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

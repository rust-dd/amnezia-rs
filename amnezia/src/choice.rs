//! Dialogue choices return an option index to the interpreter. Cancel follows
//! RM2000's configured cancel branch, or is refused when cancellation is disabled.

use bevy::prelude::*;

mod input;
pub(crate) mod smoke;
use input::update as choice_input;

/// Choice state; `indent` identifies the interpreter branch receiving the result.
#[derive(Resource, Default)]
pub struct Choice {
    pub options: Vec<String>,
    pub(crate) disabled: Vec<usize>,
    pub cursor: usize,
    pub indent: u32,
    /// RM2000 `ShowChoices` cancel type (`parameters[0]`): `0` disallows cancel,
    /// otherwise the cancel key picks option `cancel_type - 1` (the special cancel
    /// branch when it exceeds the listed options).
    pub cancel_type: i32,
    pub active: bool,
    pub(crate) generation: u64,
    pub result: Option<i32>,
}

impl Choice {
    /// Show `options` for the choice at event `indent` with the given RM2000
    /// `cancel_type`. The interpreter pauses until the player confirms or cancels
    /// (`active` clears and `result` is set).
    pub fn open(&mut self, options: Vec<String>, indent: u32, cancel_type: i32) {
        self.options = options;
        self.disabled.clear();
        self.cursor = 0;
        self.indent = indent;
        self.cancel_type = cancel_type;
        self.active = true;
        self.result = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Whether the menu is showing (the interpreter and movement pause).
    pub fn active(&self) -> bool {
        self.active
    }
}

pub struct ChoicePlugin;

impl Plugin for ChoicePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Choice>().add_systems(
            Update,
            choice_input
                .in_set(crate::dialogue::PromptInput)
                .after(crate::menu::MenuInput),
        );
    }
}

#[cfg(test)]
mod tests;

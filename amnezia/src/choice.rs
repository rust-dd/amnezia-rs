//! Dialogue choices: the windowskin menu the interpreter opens for a
//! `ShowChoice` command. It renders at the message box's position with the
//! options listed under a cursor the player moves with the arrow keys and
//! confirms with the action key; the cancel key selects the choice's configured
//! cancel option (or is refused when the choice disallows cancelling). The
//! interpreter reads the chosen index back and runs the matching branch.

use bevy::prelude::*;

mod input;
pub(crate) mod smoke;
use input::update as choice_input;

/// The active choice menu: the option labels, the cursor row, which event
/// `indent` this choice belongs to, its RM2000 cancel type, whether it's showing,
/// and — once the player confirms or cancels — the chosen option index the
/// interpreter consumes.
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

//! Numeric entry: the windowskin box the interpreter opens for an `InputNumber`
//! command (RM2000 opcode `10150`). It shows a row of digit slots the player
//! edits with the arrow keys and confirms with the action key; the interpreter
//! reads the assembled number back and stores it in the target variable.

use bevy::prelude::*;

mod input;
pub(crate) mod smoke;
use input::update as input_number_input;

/// The active numeric entry box: how many digit slots it has, which variable the
/// result is destined for, the per-slot digits and the cursor slot, the running
/// assembled `value`, whether it's showing, and — once the player confirms — the
/// entered number the interpreter consumes.
#[derive(Resource, Default)]
pub struct InputNumber {
    /// Slot count the box opened with; kept for parity with the RM2000 command
    /// though the UI derives its width from `slots`.
    #[allow(dead_code)]
    pub digits: u32,
    pub var_id: u32,
    pub value: i64,
    pub active: bool,
    pub(crate) generation: u64,
    pub result: Option<i64>,
    slots: Vec<u8>,
    cursor: usize,
}

impl InputNumber {
    /// Show a numeric entry box of `digits` slots writing into variable `var_id`.
    /// The interpreter pauses until the player confirms (`active` clears and
    /// `result` is set to the assembled number).
    pub fn open(&mut self, digits: u32, var_id: u32) {
        self.digits = digits;
        self.var_id = var_id;
        self.slots = vec![0u8; digits as usize];
        self.cursor = self.slots.len().saturating_sub(1);
        self.value = 0;
        self.active = true;
        self.result = None;
        self.generation = self.generation.wrapping_add(1);
    }

    pub(crate) fn slots(&self) -> &[u8] {
        &self.slots
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether the box is showing (the interpreter and movement pause).
    pub fn active(&self) -> bool {
        self.active
    }

    /// Nudge the cursor slot's digit by `delta`, wrapping within `0..=9`, then
    /// recompute the assembled value.
    fn adjust(&mut self, delta: i8) {
        let cursor = self.cursor;
        let Some(slot) = self.slots.get_mut(cursor) else {
            return;
        };
        *slot = (*slot as i8 + delta).rem_euclid(10) as u8;
        self.recompute();
    }

    /// Reassemble `value` from the digit slots (most significant slot first).
    fn recompute(&mut self) {
        self.value = self.slots.iter().fold(0i64, |acc, &d| acc * 10 + d as i64);
    }
}

pub struct InputNumberPlugin;

impl Plugin for InputNumberPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputNumber>().add_systems(
            Update,
            input_number_input
                .in_set(crate::dialogue::PromptInput)
                .after(crate::menu::MenuInput),
        );
    }
}

#[cfg(test)]
mod tests;

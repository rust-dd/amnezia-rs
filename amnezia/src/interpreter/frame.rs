//! Per-run execution state, shared by the foreground event and each parallel script.

use super::commands::KeyAccept;
use crate::battle::BattleOutcome;
use crate::interpreter::flow::{skip_battle_handler, skip_shop_handler};
use amnezia_data::EventCommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-update command budget, shared by foreground continuations and separate
/// for each parallel interpreter.
pub(super) const MAX_STEPS_PER_FRAME: usize = 10_000;

/// The maximum nested `CallEvent` depth, guarding a page that calls itself (or a
/// cycle of pages) from growing the call stack without bound.
pub(super) const MAX_CALL_DEPTH: usize = 64;

/// Suspended CallEvent caller; finishing the callee restores its list, IP and event scope.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct CallFrame {
    pub(super) commands: Vec<EventCommand>,
    pub(super) ip: usize,
    pub(super) event_id: u32,
    #[serde(default)]
    pub(super) decision: bool,
    #[serde(default)]
    pub(super) choices: HashMap<u32, i32>,
    #[serde(default)]
    pub(super) battle_outcome: Option<BattleOutcome>,
    #[serde(default)]
    pub(super) shop_transacted: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Frame {
    /// Runtime-only ownership for cancelling a replaced scene request.
    #[serde(skip)]
    pub(super) scene_request: Option<u64>,
    pub(super) commands: Vec<EventCommand>,
    pub(super) ip: usize,
    pub(super) active: bool,
    pub(super) parallel: bool,
    pub(super) wait: f32,
    /// Set by `ProceedWithMovement` (11340): hold the frame until every forced
    /// move route on the map has finished. `MoveEvent` itself is fire-and-forget;
    /// only this opcode blocks, mirroring RM2000's "wait until movement complete".
    pub(super) wait_movement: bool,
    pub(super) event_id: u32,
    #[serde(default)]
    pub(super) decision: bool,
    pub(super) message_pending: bool,
    pub(super) choice_pending: bool,
    pub(super) choices: HashMap<u32, i32>,
    /// Suspended caller frames from `CallEvent`; a finished callee pops back to the
    /// top frame, and the run ends only when the stack is empty.
    pub(super) call_stack: Vec<CallFrame>,
    /// Set while a waiting `KeyInputProc` (11610) holds the event: each frame's
    /// poll writes the pressed key's RM2000 code into `key_var` and resumes.
    pub(super) key_pending: bool,
    pub(super) key_var: u32,
    pub(super) key_accept: KeyAccept,
    /// Set while a `BattleRequest` is in flight: holds the event paused across the
    /// fight until [`crate::battle::BattleResult`] is published, then consumed into
    /// `battle_outcome`.
    pub(super) battle_pending: bool,
    /// The finished fight's outcome while an `EnemyEncounter` block runs its
    /// handlers; the matching Victory/Escape/Defeat body executes, `EndBattle` clears it.
    pub(super) battle_outcome: Option<BattleOutcome>,
    /// Set while a shop/inn screen is open: holds the event paused until
    /// [`crate::shop::ShopOpen`] clears, then the block is skipped to its terminator.
    pub(super) shop_pending: bool,
    /// Set while an `InputNumber` box is open: holds the event paused until the
    /// player confirms, then the entered value is written to the target variable.
    pub(super) input_pending: bool,
    /// The merchant screen's result while a shop/inn block runs its handlers.
    pub(super) shop_transacted: Option<bool>,
}

impl Frame {
    pub(super) fn active(&self) -> bool {
        self.active
    }

    pub(super) fn wait_for(&mut self, seconds: f32) {
        self.wait = if seconds == 0.0 { 1.0 / 60.0 } else { seconds };
    }

    pub(super) fn consume_wait(&mut self) -> bool {
        if self.wait <= 0.0 {
            return false;
        }
        // Saves retain seconds; round each logical tick to avoid accumulating float drift.
        let frames = (f64::from(self.wait) * 60.0).round();
        self.wait = ((frames - 1.0).max(0.0) / 60.0) as f32;
        true
    }

    /// Begin running `commands` from the top. Ignored if a run is already live,
    /// so one event can't interrupt another mid-sequence.
    pub(super) fn start(&mut self, event_id: u32, commands: Vec<EventCommand>) {
        if self.active {
            return;
        }
        self.reset();
        self.commands = commands;
        self.event_id = event_id;
        self.active = true;
    }

    /// Reset to the idle state, dropping the command list and every suspension.
    pub(super) fn stop(&mut self) {
        self.reset();
    }

    pub(super) fn call(&mut self, commands: Vec<EventCommand>, event_id: u32) -> bool {
        self.push(commands, event_id, self.ip + 1)
    }

    pub(super) fn push_foreground(&mut self, commands: Vec<EventCommand>, event_id: u32) {
        assert!(self.push(commands, event_id, self.ip));
    }

    pub(super) fn base_event_id(&self) -> u32 {
        self.call_stack
            .first()
            .map_or(self.event_id, |frame| frame.event_id)
    }

    fn push(&mut self, commands: Vec<EventCommand>, event_id: u32, return_ip: usize) -> bool {
        if self.call_stack.len() >= MAX_CALL_DEPTH {
            return false;
        }
        self.call_stack.push(CallFrame {
            commands: std::mem::take(&mut self.commands),
            ip: return_ip,
            event_id: self.event_id,
            decision: self.decision,
            choices: std::mem::take(&mut self.choices),
            battle_outcome: self.battle_outcome.take(),
            shop_transacted: self.shop_transacted.take(),
        });
        self.commands = commands;
        self.ip = 0;
        self.event_id = event_id;
        self.decision = false;
        true
    }

    pub(super) fn return_to_caller(&mut self) -> bool {
        let Some(caller) = self.call_stack.pop() else {
            return false;
        };
        self.commands = caller.commands;
        self.ip = caller.ip;
        self.event_id = caller.event_id;
        self.decision = caller.decision;
        self.choices = caller.choices;
        self.battle_outcome = caller.battle_outcome;
        self.shop_transacted = caller.shop_transacted;
        true
    }

    fn reset(&mut self) {
        self.scene_request = None;
        self.active = false;
        self.parallel = false;
        self.commands.clear();
        self.ip = 0;
        self.wait = 0.0;
        self.wait_movement = false;
        self.event_id = 0;
        self.decision = false;
        self.message_pending = false;
        self.choice_pending = false;
        self.choices.clear();
        self.call_stack.clear();
        self.key_pending = false;
        self.key_var = 0;
        self.key_accept = KeyAccept::default();
        self.battle_pending = false;
        self.battle_outcome = None;
        self.shop_pending = false;
        self.input_pending = false;
        self.shop_transacted = None;
    }

    /// Enter the matching battle outcome branch; skip other handlers like choice options.
    pub(super) fn select_battle_handler(&mut self, indent: u32, want: BattleOutcome) {
        if self.battle_outcome == Some(want) {
            self.ip += 1;
        } else {
            self.ip = skip_battle_handler(&self.commands, self.ip, indent);
        }
    }

    /// Enter the matching merchant branch: true = transaction/stay, false = no transaction/cancel.
    pub(super) fn select_shop_handler(&mut self, indent: u32, want: bool) {
        if self.shop_transacted == Some(want) {
            self.ip += 1;
        } else {
            self.ip = skip_shop_handler(&self.commands, self.ip, indent);
        }
    }

    /// The encounter's defeat mode is independent of any later battle's handlers.
    pub(super) fn defeat_is_unhandled(&self) -> bool {
        self.encounter_option(4) == 0
    }

    pub(super) fn escape_ends_event(&self) -> bool {
        self.encounter_option(3) == 1
    }

    fn encounter_option(&self, index: usize) -> i32 {
        self.commands
            .get(self.ip.saturating_sub(1))
            .and_then(|c| c.params.get(index))
            .copied()
            .unwrap_or(0)
    }
}

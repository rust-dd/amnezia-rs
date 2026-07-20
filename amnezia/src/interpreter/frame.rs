//! The interpreter execution frame: the per-run state a single event-command
//! list carries as it steps. One [`Frame`] backs the foreground
//! [`super::RunningEvent`]; the parallel pool ([`super::parallel`]) owns one more
//! per concurrently-running common event and parallel-process map page. The
//! opcode dispatch in [`super::exec`] operates on a `Frame`, so foreground and
//! background execution share identical command semantics.

use super::commands::KeyAccept;
use crate::battle::BattleOutcome;
use crate::interpreter::flow::{has_defeat_handler, skip_battle_handler};
use amnezia_data::EventCommand;
use std::collections::HashMap;

/// A frame-local cap on executed commands, so a malformed list (e.g. a branch
/// that never advances) can't lock up the frame. Well-formed pages never
/// approach it — every command strictly advances the instruction pointer.
pub(super) const MAX_STEPS_PER_FRAME: usize = 10_000;

/// The maximum nested `CallEvent` depth, guarding a page that calls itself (or a
/// cycle of pages) from growing the call stack without bound.
pub(super) const MAX_CALL_DEPTH: usize = 64;

/// The character a `MoveEvent` set walking, so the frame that issued it resumes
/// only once *that* character's queue drains — not when every queue on the map
/// happens to be idle. A global "all idle" test would let one parallel mover
/// stall behind another's ambient pacing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveWait {
    Hero,
    Event(u32),
}

/// A caller frame suspended by `CallEvent` (12330): the interrupted command list,
/// the instruction pointer to resume at, and the event id in scope. The callee
/// runs in place; reaching its end pops the frame and resumes the caller.
pub(super) struct CallFrame {
    pub(super) commands: Vec<EventCommand>,
    pub(super) ip: usize,
    pub(super) event_id: u32,
}

/// The per-run interpreter state: the command list, the instruction pointer,
/// whether a run is live, the pending-wait timers, and every subsystem-block
/// suspension flag. Shared verbatim by the foreground and parallel interpreters.
#[derive(Default)]
pub(super) struct Frame {
    pub(super) commands: Vec<EventCommand>,
    pub(super) ip: usize,
    pub(super) active: bool,
    pub(super) wait: f32,
    /// The character a `MoveEvent` is waiting on, if any.
    pub(super) wait_move: Option<MoveWait>,
    pub(super) event_id: u32,
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
    /// Whether this frame is currently executing.
    pub(super) fn active(&self) -> bool {
        self.active
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

    fn reset(&mut self) {
        self.active = false;
        self.commands.clear();
        self.ip = 0;
        self.wait = 0.0;
        self.wait_move = None;
        self.event_id = 0;
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

    /// Self-select an `EnemyEncounter` outcome handler: run its body (advance into
    /// it) when the finished battle's outcome is `want`, otherwise skip to the next
    /// handler or the block terminator. Mirrors the `ShowChoice` option arms.
    pub(super) fn select_battle_handler(&mut self, indent: u32, want: BattleOutcome) {
        if self.battle_outcome == Some(want) {
            self.ip += 1;
        } else {
            self.ip = skip_battle_handler(&self.commands, self.ip, indent);
        }
    }

    /// Self-select a shop/inn outcome handler: run its body when the merchant
    /// result matches `want` (Transaction/Stay = `true`, NoTransaction/Cancel =
    /// `false`), else skip to the next handler or the block terminator.
    pub(super) fn select_shop_handler(&mut self, indent: u32, want: bool) {
        if self.shop_transacted == Some(want) {
            self.ip += 1;
        } else {
            self.ip = skip_battle_handler(&self.commands, self.ip, indent);
        }
    }

    /// Whether a party wipe reaching the `EnemyEncounter` at the cursor has no
    /// `DefeatHandler` — an unrecoverable loss the caller routes to Game Over.
    pub(super) fn defeat_is_unhandled(&self) -> bool {
        let indent = self.commands.get(self.ip).map_or(0, |c| c.indent);
        !has_defeat_handler(&self.commands, self.ip, indent)
    }
}

//! The live [`Battle`] resource: its coarse phase and menu state, the whole
//! battle struct, and the per-round command/turn-flow. Building a fresh encounter
//! lives in [`super::build`]; the resolution mathematics in [`crate::battle::resolve`].

use super::{
    Action, BattleOutcome, BattleSe, Command, Fighter, Foe, HitReport, PendingAnim, Source, logic,
};
use amnezia_data::{AttributeDef, ItemDef, SkillDef, StateDef};
use bevy::prelude::*;

/// Allow renderer spawn latency, but bound waits for missing animation IDs.
const ANIM_HOLD_GRACE_TICKS: u32 = 12;

#[cfg(test)]
pub const LOG_TAIL: usize = 5;

/// The battle's coarse phase, which gates the input/resolve/outcome systems.
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum Phase {
    #[default]
    Inactive,
    Encounter,
    Escape,
    /// Fight/Auto/Escape selection before per-actor commands.
    PartyCommand,
    Command,
    Resolve,
    Outcome,
}

/// The command menu's current level while a member is choosing.
#[derive(Default, PartialEq, Clone, Copy)]
pub enum MenuLevel {
    #[default]
    Command,
    Skill,
    Item,
    Target,
    AllyTarget,
}

/// The whole live battle, held as a Bevy resource and reset to `default()` (the
/// `Inactive` phase) between fights.
#[derive(Resource, Default)]
pub struct Battle {
    pub(in crate::battle) timeline: crate::battle::resolve::timeline::Timeline,
    pub(in crate::battle) events: crate::battle::events::BattleEvents,
    pub(in crate::battle) ai_switches: std::collections::BTreeSet<u32>,
    pub(in crate::battle) pending_switches: Vec<(u32, bool)>,
    pub phase: Phase,
    pub(in crate::battle) messages: crate::battle::message::Messages,
    pub background: String,
    pub allow_escape: bool,
    pub members: Vec<Fighter>,
    pub enemies: Vec<Foe>,
    pub(in crate::battle) attributes: Vec<AttributeDef>,
    pub(in crate::battle) states: Vec<StateDef>,
    pub(in crate::battle) skills: Vec<SkillDef>,
    pub(in crate::battle) items: Vec<ItemDef>,
    /// The current battle round, counting from `1`, gating turn-numbered AI.
    pub round: u32,
    /// Initialized from average agilities at battle start; failed escapes add 10 points.
    pub(in crate::battle) escape_chance: u32,
    /// Whether the party opened with a first strike (RM2000 preemptive attack): it
    /// grants a guaranteed escape and the `+9999` turn-order bonus.
    pub(in crate::battle) first_strike: bool,
    pub turn: usize,
    pub menu: MenuLevel,
    pub cursor: usize,
    pub(in crate::battle) menu_cursors: [usize; 5],
    pub(in crate::battle) skill_cursors: [usize; 4],
    /// The chosen skill's id while its target is being picked.
    pub pending_skill: Option<u32>,
    /// The chosen item's id while its ally target is being picked.
    pub pending_item: Option<u32>,
    pub queue: Vec<Action>,
    pub queue_at: usize,
    pub log: Vec<String>,
    pub outcome: Option<BattleOutcome>,
    pub reward_exp: u32,
    pub reward_gold: u32,
    pub(in crate::battle) reward_items: Vec<u32>,
    pub(in crate::battle) outcome_log_start: usize,
    pub(in crate::battle) outcome_message: crate::battle::outcome_text::Script,
    /// A unique-per-fight stamp (the build seed) the UI watches to rebuild the
    /// enemy battler nodes exactly once when a new encounter begins.
    pub generation: u64,
    /// Queued casts, drained into `PlayAnimation` messages by the battle systems.
    pub(in crate::battle) pending_anims: Vec<PendingAnim>,
    /// Per-tick hit reports drained into diagnostic traces, not drawn on screen.
    pub(in crate::battle) hit_reports: Vec<HitReport>,
    /// Foe positions owed a visibility blink for non-absorbing HP hits.
    /// Animation flashes are independent; absorption does not trigger this blink.
    pub(in crate::battle) pending_blinks: Vec<(f32, f32)>,
    pub(in crate::battle) pending_action_flashes: Vec<Source>,
    pub(in crate::battle) pending_shake: bool,
    /// Queued sound roles, resolved against `SystemDef` by the battle systems.
    pub(in crate::battle) pending_se: Vec<BattleSe>,
    /// Troop-event animation wait; action animations use the frame timeline.
    pub(in crate::battle) anim_hold: bool,
    /// The renderer acknowledged the request or reported a live animation.
    /// Acknowledgement also covers effects that finish within one low-FPS update.
    pub(in crate::battle) anim_seen: bool,
    /// Unacknowledged ticks, bounded by [`ANIM_HOLD_GRACE_TICKS`].
    pub(in crate::battle) anim_hold_ticks: u32,
    /// Prevents paying rewards again while the outcome screen waits for input.
    pub(in crate::battle) rewarded: bool,
    /// Vocabulary snapshot captured at battle start, preserving intentional blanks.
    pub(in crate::battle) text: crate::battle::log_terms::BattleText,
    pub(in crate::battle) rng: u64,
}

/// Advance a little PCG-style LCG and return its next output. Self-contained so
/// the crate takes on no `rand` dependency for battle variance and AI rolls.
pub(in crate::battle) fn rng_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) ^ *state
}

impl Battle {
    /// The index of the next living member still owing a command this round.
    pub fn next_chooser(&self) -> Option<usize> {
        self.members
            .iter()
            .position(|f| f.alive() && f.command.is_none())
    }

    /// Living enemy indices, in placement order (the target-menu order).
    pub fn living_enemies(&self) -> Vec<usize> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive())
            .map(|(i, _)| i)
            .collect()
    }

    /// Living party member indices, in party order (the ally-target-menu order).
    pub fn living_members(&self) -> Vec<usize> {
        self.members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .map(|(i, _)| i)
            .collect()
    }

    /// Commit `command` for the member currently choosing, then move on: to the
    /// next chooser, or into resolution once every living member has an order.
    pub fn commit(&mut self, command: Command) {
        if let Some(f) = self.members.get_mut(self.turn) {
            f.command = Some(command);
            f.defending = matches!(command, Command::Defend);
        }
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.pending_item = None;
        self.skip_restricted_choosers();
    }

    /// Undo the last freely chosen order, skipping forced actions. With no earlier
    /// chooser, return to party options (RM2000 `SelectPreviousActor`).
    pub fn undo_choice(&mut self) {
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.pending_item = None;
        let target = self.members[..self.turn]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, f)| {
                f.alive()
                    && f.command.is_some()
                    && logic::worst_restriction(&f.states, &self.states) == 0
            })
            .map(|(i, _)| i);
        match target {
            Some(i) => {
                self.members[i].command = None;
                self.turn = i;
            }
            None => self.phase = Phase::PartyCommand,
        }
    }

    /// Enter Fight commands, auto-ordering restricted members before the first free chooser.
    pub fn begin_actor_commands(&mut self) {
        self.phase = Phase::Command;
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.skip_restricted_choosers();
    }

    /// Auto-order living members while preserving forced actions, then resolve.
    pub fn auto_battle(&mut self) {
        self.auto_battle_commands();
    }

    /// Build the agility-ordered turn queue from every member's committed command
    /// plus each living enemy's AI-chosen action (`resolve::enemy_action`), and
    /// start resolving.
    pub(in crate::battle) fn begin_resolve(&mut self) {
        self.timeline = default();
        self.events.next_turn();
        let alive: Vec<bool> = self.members.iter().map(|f| f.alive()).collect();
        let mut actions: Vec<Action> = Vec::new();
        for (i, f) in self.members.iter().enumerate() {
            if let Some(kind) = f.command {
                actions.push(Action {
                    source: Source::Party(i),
                    kind,
                    agility: self.battler_stats(Source::Party(i)).agility,
                });
            }
        }
        for i in 0..self.enemies.len() {
            if let Some(action) = self.enemy_action(i, &alive) {
                actions.push(action);
            }
        }
        // RM2000 re-rolls agility jitter each round; first strike adds 9999 to party keys.
        let keys: Vec<u32> = actions
            .iter()
            .map(|a| {
                let jitter = (rng_next(&mut self.rng) % (a.agility as u64 / 4 + 4)) as u32;
                let key = a.agility.saturating_add(jitter);
                match a.source {
                    Source::Party(_) if self.first_strike => key.saturating_add(9999),
                    _ => key,
                }
            })
            .collect();
        self.queue = logic::turn_order(&keys)
            .into_iter()
            .map(|i| actions[i])
            .collect();
        self.queue_at = 0;
        self.clear_anim_hold();
        self.phase = Phase::Resolve;
    }

    /// Wait for a troop-event animation, including renderer acknowledgement.
    pub(in crate::battle) fn begin_anim_hold(&mut self) {
        self.anim_hold = true;
        self.anim_seen = false;
        self.anim_hold_ticks = 0;
    }

    #[cfg(test)]
    pub(in crate::battle) fn anim_hold_active(&self) -> bool {
        self.anim_hold
    }

    fn clear_anim_hold(&mut self) {
        self.anim_hold = false;
        self.anim_seen = false;
        self.anim_hold_ticks = 0;
    }

    /// Wait until an observed animation ends, or the unacknowledged-request grace expires.
    pub(in crate::battle) fn tick_anim_hold(&mut self, anim_live: bool) -> bool {
        if !self.anim_hold {
            return false;
        }
        if anim_live {
            self.anim_seen = true;
            return true;
        }
        if self.anim_seen {
            self.anim_hold = false;
            return false;
        }
        self.anim_hold_ticks += 1;
        if self.anim_hold_ticks >= ANIM_HOLD_GRACE_TICKS {
            self.anim_hold = false;
            return false;
        }
        true
    }

    /// Reopen party commands; state recovery waits for each battler's action.
    pub fn new_round(&mut self) {
        self.timeline = default();
        self.first_strike = false;
        self.round += 1;
        for f in &mut self.members {
            f.command = None;
        }
        self.queue.clear();
        self.queue_at = 0;
        self.clear_anim_hold();
        self.pending_anims.clear();
        self.hit_reports.clear();
        self.pending_blinks.clear();
        self.pending_action_flashes.clear();
        self.pending_shake = false;
        self.pending_se.clear();
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.pending_item = None;
        // Reopen the party-option window; per-actor entry (and the restricted-member
        // auto-ordering) resumes only once the player picks Fight.
        self.phase = Phase::PartyCommand;
    }

    #[cfg(test)]
    pub fn log_tail(&self) -> String {
        let start = self.log.len().saturating_sub(LOG_TAIL);
        self.log[start..].join("\n")
    }
}

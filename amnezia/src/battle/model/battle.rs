//! The live [`Battle`] resource: its coarse phase and menu state, the whole
//! battle struct, and the per-round command/turn-flow. Building a fresh encounter
//! lives in [`super::build`]; the resolution mathematics in [`crate::battle::resolve`].

use super::{
    Action, BattleOutcome, BattleSe, Command, Fighter, Foe, PendingAnim, PendingNumber, Source,
    Step, logic,
};
use amnezia_data::{AttributeDef, ItemDef, SkillDef, StateDef};
use bevy::prelude::*;

/// Seconds between two resolved actions, so the log and damage read at a human
/// pace rather than flashing past in one frame.
pub const RESOLVE_STEP_SECS: f32 = 0.7;

/// Frames [`Battle::tick_anim_hold`] waits for a just-queued battle animation to
/// appear before giving up and applying its impact anyway. Comfortably longer
/// than the one or two frames the overlay needs to spawn the `LiveAnimation`, so
/// it only ever fires for an unknown animation id that spawns nothing (rather
/// than wedging resolution forever).
const ANIM_HOLD_GRACE_TICKS: u32 = 12;

/// How many trailing log lines the battle keeps for display.
pub const LOG_TAIL: usize = 5;

/// The battle's coarse phase, which gates the input/resolve/outcome systems.
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum Phase {
    #[default]
    Inactive,
    /// The RM2000 party-level option window at the top of each round: Fight (drop
    /// to per-actor [`Command`] entry), Auto (auto-battle the whole party), or
    /// Escape (attempt to flee). Shown before any member picks an order.
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
    pub(in crate::battle) events: crate::battle::events::BattleEvents,
    pub phase: Phase,
    pub background: String,
    pub allow_escape: bool,
    pub members: Vec<Fighter>,
    pub enemies: Vec<Foe>,
    /// The attribute (element) table, consulted by the elemental damage step.
    pub(in crate::battle) attributes: Vec<AttributeDef>,
    /// The state (status) table, consulted to name an inflicted or cured state.
    pub(in crate::battle) states: Vec<StateDef>,
    /// The skill table, looked up by id on a cast for its element, inflicted
    /// states, and heal-vs-damage scope.
    pub(in crate::battle) skills: Vec<SkillDef>,
    /// The item table, looked up by id when a member uses a medicine so its real
    /// HP/SP recovery and status cures apply, rather than a flat placeholder heal.
    pub(in crate::battle) items: Vec<ItemDef>,
    /// The current battle round, counting from `1`, gating turn-numbered AI.
    pub round: u32,
    /// The party's current escape chance in percent (RM2000 / EasyRPG
    /// `escape_chance`): set once at [`Battle::build`] from the two sides' average
    /// agilities ([`logic::init_escape_chance`]) and raised by 10 on each failed
    /// escape (see [`Battle::attempt_escape`]).
    pub(in crate::battle) escape_chance: u32,
    /// Whether the party opened with a first strike (RM2000 preemptive attack): it
    /// grants a guaranteed escape and the `+9999` turn-order bonus.
    pub(in crate::battle) first_strike: bool,
    pub turn: usize,
    pub menu: MenuLevel,
    pub cursor: usize,
    pub(in crate::battle) menu_cursors: [usize; 5],
    /// The chosen skill's id while its target is being picked.
    pub pending_skill: Option<u32>,
    /// The chosen item's id while its ally target is being picked.
    pub pending_item: Option<u32>,
    pub queue: Vec<Action>,
    pub queue_at: usize,
    pub timer: Timer,
    pub log: Vec<String>,
    pub outcome: Option<BattleOutcome>,
    pub reward_exp: u32,
    pub reward_gold: u32,
    /// A unique-per-fight stamp (the build seed) the UI watches to rebuild the
    /// enemy battler nodes exactly once when a new encounter begins.
    pub generation: u64,
    /// Attack animations queued as the current tick's actions resolve; drained
    /// each frame by `battle.rs` into `PlayAnimation` overlays and cleared by
    /// [`Battle::new_round`] (a fresh [`Battle::build`] starts it empty).
    pub(in crate::battle) pending_anims: Vec<PendingAnim>,
    /// Floating damage/heal numbers queued as the current tick's actions resolve,
    /// drained each frame by `battle::floaters` into rising overlay text and
    /// cleared by [`Battle::new_round`] (a fresh [`Battle::build`] starts empty).
    pub(in crate::battle) pending_numbers: Vec<PendingNumber>,
    /// Foe screen positions owed a guaranteed per-hit whitening blink, drained by
    /// `battle::scene` into a blink on each struck sprite. Every landed blow
    /// enqueues one, independent of the played animation's own flash timings.
    pub(in crate::battle) pending_blinks: Vec<(f32, f32)>,
    /// Battle sound effects owed as the current tick's actions resolve (a hit
    /// landed, a foe felled, an attack evaded), drained each frame by `battle.rs`
    /// into `AudioRequest`s named from the loaded `SystemDef` and cleared by
    /// [`Battle::new_round`] (a fresh [`Battle::build`] starts empty).
    pub(in crate::battle) pending_se: Vec<BattleSe>,
    /// Sub-steps the action currently resolving still owes, drained one per
    /// resolve tick (see [`Step`]) so a multi-target cast staggers its numbers
    /// and a critical announces on its own beat. Cleared by [`Battle::new_round`]
    /// and [`Battle::begin_resolve`].
    pub(in crate::battle) steps: std::collections::VecDeque<Step>,
    /// While a queued battle animation plays, resolution pauses and the pending
    /// strike/cast impact is held so its damage number lands only once the
    /// animation has finished (RM2000 sequences the animation, its `SetWait`,
    /// then the damage). Set when the animation is queued; `battle::resolve_tick`
    /// drives it down through [`Battle::tick_anim_hold`].
    pub(in crate::battle) anim_hold: bool,
    /// Whether the held animation has been observed live at least once, so the
    /// hold releases on its disappearance rather than the one-tick lag between
    /// queuing the animation and its `LiveAnimation` overlay appearing.
    pub(in crate::battle) anim_seen: bool,
    /// Frames the current hold has waited without the animation ever appearing,
    /// bounding the wait for an unknown/absent animation id (see
    /// [`ANIM_HOLD_GRACE_TICKS`]) so resolution can never wedge.
    pub(in crate::battle) anim_hold_ticks: u32,
    /// Set while a deferred skill/enemy cast re-runs after its animation, so the
    /// shared cast helper skips re-queuing the already-played animation.
    pub(in crate::battle) suppress_anim: bool,
    /// Set once the victory reward (gold, experience, and any level-ups) has been
    /// paid on entering the outcome, so `battle::apply_victory_rewards` pays out
    /// exactly once while the outcome screen waits for the player.
    pub(in crate::battle) rewarded: bool,
    /// The real RM2000 battle-end message terms (victory / defeat / escape and the
    /// reward lines), captured from the loaded vocabulary at battle start; the
    /// invented Hungarian defaults stand in until then (see
    /// [`crate::battle::log_terms`]).
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
        }
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.pending_item = None;
        self.skip_restricted_choosers();
    }

    /// Step back to the previous living chooser, clearing its order (RM2000 back).
    /// Auto-committed restricted members (asleep/berserk/confused) can't be
    /// re-ordered, so the step skips over them to the last freely-chosen member.
    /// With nothing earlier to undo — cancel on the first freely-choosing member —
    /// it backs all the way out to the party-option window (Fight / Auto / Escape),
    /// matching RM2000 `SelectPreviousActor` returning to `State_SelectOption` when
    /// the active actor is the first ally.
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

    /// Drop from the party-option window into per-actor command entry (RM2000
    /// Fight): enter the command phase and hand the round to the first member who
    /// may freely choose, auto-ordering and skipping any restricted members — or
    /// resolving at once if none can act.
    pub fn begin_actor_commands(&mut self) {
        self.phase = Phase::Command;
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.skip_restricted_choosers();
    }

    /// Auto-battle the whole party (RM2000 Auto): order every living member a basic
    /// attack on a random living enemy (a restricted member keeps its forced
    /// action), then resolve the round. Reuses the same target/AI helpers as the
    /// per-actor flow, so the enemies still act.
    pub fn auto_battle(&mut self) {
        self.auto_battle_commands();
    }

    /// Build the agility-ordered turn queue from every member's committed command
    /// plus each living enemy's AI-chosen action (`resolve::enemy_action`), and
    /// start resolving.
    pub(in crate::battle) fn begin_resolve(&mut self) {
        self.events.next_turn();
        let alive: Vec<bool> = self.members.iter().map(|f| f.alive()).collect();
        let mut actions: Vec<Action> = Vec::new();
        for (i, f) in self.members.iter().enumerate() {
            if let Some(kind) = f.command {
                actions.push(Action {
                    source: Source::Party(i),
                    kind,
                    agility: f.stats.agility,
                });
            }
        }
        for i in 0..self.enemies.len() {
            if let Some(action) = self.enemy_action(i, &alive) {
                actions.push(action);
            }
        }
        // RM2000 `CreateExecutionOrder`: each battler's sort key is its agility
        // plus a fresh jitter of `Rand::GetRandomNumber(0, agi/4 + 3)`, re-rolled
        // every round, sorted fastest-first. The stable `turn_order` keeps the
        // given order on equal keys. On a first strike (RM2000 preemptive) every
        // party battler's key gains the +9999 bonus, launching the whole party
        // ahead of the foes; `first_strike` is dormant until an encounter path
        // raises it.
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
        self.steps.clear();
        self.clear_anim_hold();
        self.timer.reset();
        self.phase = Phase::Resolve;
    }

    /// Begin holding resolution until the just-queued battle animation has played
    /// out, so the deferred strike/cast impact lands only once the swing/cast is
    /// seen (see [`Battle::anim_hold`]).
    pub(in crate::battle) fn begin_anim_hold(&mut self) {
        self.anim_hold = true;
        self.anim_seen = false;
        self.anim_hold_ticks = 0;
    }

    /// Whether resolution is currently paused waiting on a battle animation.
    pub(in crate::battle) fn anim_hold_active(&self) -> bool {
        self.anim_hold
    }

    /// Clear any in-progress animation hold and the cast-suppression flag.
    fn clear_anim_hold(&mut self) {
        self.anim_hold = false;
        self.anim_seen = false;
        self.anim_hold_ticks = 0;
        self.suppress_anim = false;
    }

    /// Advance the animation hold given whether any battle animation is live this
    /// frame, returning `true` while resolution must keep waiting. The hold clears
    /// once an observed animation has ended; as a safety net for an animation id
    /// that spawns nothing, it also clears once the grace window
    /// ([`ANIM_HOLD_GRACE_TICKS`]) elapses without one ever appearing.
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

    /// Open a fresh command round: bump the round, clear each order and defence,
    /// wear off timed states, then hand the round to the first member who may
    /// freely choose (auto-ordering and skipping any restricted members).
    pub fn new_round(&mut self) {
        self.round += 1;
        for f in &mut self.members {
            f.command = None;
            f.defending = false;
        }
        // A foe's Defend lasts until its next turn; clearing it here (one round
        // later) is the RM2000 approximation, mirroring the members above.
        for e in &mut self.enemies {
            e.defending = false;
        }
        self.run_recovery();
        self.queue.clear();
        self.queue_at = 0;
        self.steps.clear();
        self.clear_anim_hold();
        self.pending_anims.clear();
        self.pending_numbers.clear();
        self.pending_blinks.clear();
        self.pending_se.clear();
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.pending_item = None;
        // Reopen the party-option window; per-actor entry (and the restricted-member
        // auto-ordering) resumes only once the player picks Fight.
        self.phase = Phase::PartyCommand;
    }

    /// The last [`LOG_TAIL`] log lines, for the log window.
    pub fn log_tail(&self) -> String {
        let start = self.log.len().saturating_sub(LOG_TAIL);
        self.log[start..].join("\n")
    }
}

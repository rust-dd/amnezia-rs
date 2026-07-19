//! The live battle state and its turn-flow: the party fighters, the enemy foes,
//! the per-round command bookkeeping, and the agility-ordered turn queue. Building
//! the encounter and marshalling commands live here; the resolution mathematics
//! (applying actions, end checks, rewards, flee) live in the sibling [`resolve`]
//! module. Both are impure-but-Bevy-free and unit-tested directly, so the Bevy
//! systems in `battle.rs`/`input.rs` stay thin drivers.
//!
//! [`resolve`]: super::resolve

use super::BattleOutcome;
use super::logic::{self, Stats};
use crate::progression::Progression;
use crate::vitals::Vitals;
use amnezia_data::{
    ActorDef, AttributeDef, EnemyActionDef, ItemDef, MonsterDef, SkillDef, StateDef, TroopDef,
};
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
#[derive(Default, PartialEq, Clone, Copy)]
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

/// A party member in the fight: live HP/SP, derived stats, and the command it has
/// chosen this round (if any).
pub struct Fighter {
    pub actor_id: u32,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub sp: i32,
    pub max_sp: i32,
    pub stats: Stats,
    pub defending: bool,
    pub command: Option<Command>,
    /// The equipped weapon's hit and crit rates (percent) and its element id,
    /// captured at build time and consumed by the to-hit / critical / elemental
    /// resolution in [`super::resolve`]. Empty-handed leaves them `0` / `0` /
    /// `None`; a `0` hit reads as the RM2000 bare-hands 90% default.
    pub weapon_hit: u32,
    pub weapon_crit: u32,
    pub weapon_element: Option<u32>,
    /// The animation this member's normal attack plays on its target: the
    /// equipped weapon's `weapon_animation`, or the actor's `unarmed_animation`
    /// when it has no weapon. `0` means "no animation" and plays nothing.
    pub(super) attack_animation: u32,
    /// This fighter's active status effects as `(state_id, turns_held)` pairs; the
    /// turn count drives [`logic::tick_recovery`]'s hold-then-wear-off schedule.
    pub states: Vec<(u32, u32)>,
    /// The skill ids this member knows at its current level (its actor `learnings`
    /// at or below the level), captured at build time. The battle skill command
    /// offers only these, not the whole database.
    pub(super) known_skills: Vec<u32>,
    /// The 1-based attribute (element) ids this member's equipped gear guards
    /// against, unioned across its five slots at build time. A matching enemy
    /// skill's damage is halved once in [`super::resolve`].
    pub(super) resist_attributes: Vec<u32>,
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// A death (`SetDeathTimer`) or self-destruct explosion (`SetExplodeTimer`)
/// playing out on a foe's sprite before it leaves the field: `elapsed` advances in
/// real time toward `secs`, and `explode` drives the RM2000 zoom-and-fade rather
/// than the plain fade. While any foe's [`Dying`] runs, `resolve_tick` holds so the
/// beat is seen; `battle::scene` reads it to drive the sprite's alpha and zoom.
pub(super) struct Dying {
    pub elapsed: f32,
    pub secs: f32,
    pub explode: bool,
}

/// A live enemy in the fight: current HP, stats, reward, and its position on the
/// battle backdrop (RM2000 320×240 pixel space).
pub struct Foe {
    pub name: String,
    pub battler: String,
    pub hp: i32,
    /// This foe's starting (maximum) HP, kept so HP-percent AI conditions resolve.
    pub max_hp: i32,
    pub stats: Stats,
    pub exp: u32,
    pub gold: u32,
    pub x: u32,
    pub y: u32,
    /// This foe's per-attribute damage ranks (0=A … 4=E), copied from its
    /// `MonsterDef`. The vector is truncated, so ids past its end read neutral C.
    pub attribute_ranks: Vec<u8>,
    /// This foe's per-state affliction ranks, copied from its `MonsterDef`, for
    /// the status-infliction chance.
    pub state_ranks: Vec<u8>,
    /// This foe's active status effects as `(state_id, turns_held)` pairs; the
    /// turn count drives [`logic::tick_recovery`]'s hold-then-wear-off schedule.
    pub states: Vec<(u32, u32)>,
    /// Whether this foe took the RM2000 Defend stance on its last turn; it halves
    /// incoming damage in [`super::resolve`] until [`Battle::new_round`] clears it.
    pub(super) defending: bool,
    /// Whether this foe fled the battle (RM2000 monster Escape). It then counts as
    /// gone (see [`Foe::alive`]) but, unlike a defeated foe, grants no reward.
    pub(super) fled: bool,
    /// Whether this foe gathered power (RM2000 Charge): its next physical strike
    /// deals double, and the flag is consumed on that strike.
    pub(super) charging: bool,
    /// This foe's RM2000 battle-AI action list, consulted each round to choose
    /// its command (cast a skill, defend, or attack on turn/HP conditions).
    pub actions: Vec<EnemyActionDef>,
    /// A death or self-destruct fade playing out on this foe's sprite before it
    /// is cleared from view (see [`Dying`]); `None` until the foe is slain.
    pub(super) dying: Option<Dying>,
}

impl Foe {
    /// Whether this foe is still in the fight: living HP and not fled. A fled foe
    /// (RM2000 Escape) counts as gone, so it drops out of targeting and the
    /// living-enemy list and grants no reward.
    pub fn alive(&self) -> bool {
        self.hp > 0 && !self.fled
    }

    /// This foe's damage rank (0=A … 4=E) against attribute `attr_id`. Ids past
    /// the truncated `attribute_ranks` vector — and the non-elemental id `0` —
    /// read neutral C (`2`).
    #[allow(dead_code)]
    pub fn attribute_rank(&self, attr_id: u32) -> u8 {
        attr_id
            .checked_sub(1)
            .and_then(|i| self.attribute_ranks.get(i as usize).copied())
            .unwrap_or(2)
    }
}

/// A chosen action, from either side, awaiting resolution. Party members choose
/// `Attack`, `Skill`, `Item`, `Defend`, or `Nothing`; the enemy AI reuses `Attack`,
/// `Skill`, `Defend`, and `Nothing`, and adds the RM2000 monster-only basics
/// `DoubleAttack`, `SelfDestruct`, `Escape`, and `Charge`.
#[derive(Clone, Copy)]
pub enum Command {
    Attack {
        target: usize,
    },
    Skill {
        skill_id: u32,
        target: usize,
    },
    Item {
        item_id: u32,
        target: usize,
    },
    Defend,
    Nothing,
    /// Enemy-only: strike the target twice (two independent hit/damage rolls).
    DoubleAttack {
        target: usize,
    },
    /// Enemy-only: damage every living party member, then the foe dies.
    SelfDestruct,
    /// Enemy-only: flee the fight — the foe leaves without granting a reward.
    Escape,
    /// Enemy-only: gather power so the foe's next physical strike deals double.
    Charge,
}

/// Which side (and index) an action originates from.
#[derive(Clone, Copy)]
pub enum Source {
    Party(usize),
    Enemy(usize),
}

/// One entry in the agility-ordered turn queue.
#[derive(Clone, Copy)]
pub struct Action {
    pub source: Source,
    pub kind: Command,
    pub agility: u32,
}

/// A deferred sub-step of the action currently resolving, drained one per
/// resolve tick so a multi-target cast staggers its per-target beats and a
/// critical shows its announcement on its own beat before the damage lands.
/// Mirrors RM2000's `ProcessBattleAction` walking its substates each behind its
/// own `SetWait`, without the full substate machine.
#[derive(Clone, Copy)]
pub(super) enum Step {
    /// Land caster `pi`'s multi-target skill on one more enemy `ti`.
    HitEnemy { pi: usize, ti: usize, skill_id: u32 },
    /// Apply caster `pi`'s multi-target heal to one more ally `ti`.
    HealAlly { pi: usize, ti: usize, skill_id: u32 },
    /// The damage beat after a "Kritikus!" announcement: land the precomputed
    /// `dmg` of member `pi`'s critical strike on enemy `ti`.
    CritDamage { pi: usize, ti: usize, dmg: i32 },
    /// Apply member `pi`'s planned normal-strike outcome on foe `ti` once its
    /// attack animation has played out: pop the dodge when `miss`, else land
    /// `dmg` — taking the critical announcement beat first when `crit`. RM2000
    /// sequences the swing animation, its wait, then the damage; this is the
    /// deferred damage half (see `resolve::resolve_strike_impact`).
    StrikeImpact {
        pi: usize,
        ti: usize,
        dmg: i32,
        crit: bool,
        miss: bool,
    },
    /// Apply member `pi`'s skill `skill_id` at `target` once its cast animation
    /// has played. The animation was queued up front; this re-runs the cast with
    /// [`Battle::suppress_anim`] set so its effect (and RNG draws) resolve now
    /// without queuing the animation a second time.
    CastSkill {
        pi: usize,
        skill_id: u32,
        target: usize,
    },
    /// Apply enemy `ei`'s skill `skill_id` at member `target` once its cast
    /// animation has played — the enemy-side counterpart of [`Step::CastSkill`].
    EnemyCast {
        ei: usize,
        skill_id: u32,
        target: usize,
    },
}

/// One queued battle animation, produced as an action resolves and drained by
/// `battle.rs`'s `drain_pending_anims` into a single `PlayAnimation` overlay
/// message. `anim_id` is the effect id; `targets` are the RM2000 screen offsets
/// from the screen centre (y downward) of every battler the cast hits — one entry
/// for a single-target strike, several for a multi-target skill — so the effect's
/// sound plays once for the whole cast while its cells and flashes land on each
/// target.
#[derive(Clone)]
pub(super) struct PendingAnim {
    pub anim_id: u32,
    pub targets: Vec<(f32, f32)>,
}

/// What a floating battle number represents, which tints it: white HP `Damage`,
/// green `Heal` (an HP or SP restore), and a pale `Miss` for a dodge or a blocked
/// (0-damage) blow. Mirrors RM2000's damage-pop colouring.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum NumberKind {
    Damage,
    Heal,
    Miss,
}

/// A floating number queued as an action resolves — the RM2000 damage/heal pop —
/// drained by `battle::floaters` into rising, fading overlay text. `pos` is the
/// target's RM2000 screen offset from centre (y downward): a foe's anim pos or a
/// member's party slot. `text` is the digits (or "Miss") and `kind` its colour.
#[derive(Clone)]
pub(super) struct PendingNumber {
    pub pos: (f32, f32),
    pub text: String,
    pub kind: NumberKind,
}

/// A System-defined battle sound effect queued as an action resolves, drained by
/// `battle.rs`'s `drain_pending_se` into an `AudioRequest` whose asset name comes
/// from the loaded `SystemDef`. The resolution stays Bevy- and data-free by
/// naming only the effect's *role* here; the drain maps it to the configured
/// sound. Mirrors the EasyRPG `SePlay(GetSystemSE(...))` sites.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BattleSe {
    /// A blow landed on a foe (RM2000 `SFX_EnemyDamage`).
    EnemyDamaged,
    /// A blow landed on a party member (RM2000 `SFX_AllyDamage`).
    ActorDamaged,
    /// An attack was evaded (RM2000 `SFX_Evasion`).
    Dodge,
    /// A foe was felled (RM2000 `SFX_EnemyKill`).
    EnemyDefeated,
    /// The party attempted to flee (RM2000 `SFX_Escape`).
    Escape,
}

/// The whole live battle, held as a Bevy resource and reset to `default()` (the
/// `Inactive` phase) between fights.
#[derive(Resource, Default)]
pub struct Battle {
    pub phase: Phase,
    pub background: String,
    pub members: Vec<Fighter>,
    pub enemies: Vec<Foe>,
    /// The attribute (element) table, consulted by the elemental damage step.
    pub(super) attributes: Vec<AttributeDef>,
    /// The state (status) table, consulted to name an inflicted or cured state.
    pub(super) states: Vec<StateDef>,
    /// The skill table, looked up by id on a cast for its element, inflicted
    /// states, and heal-vs-damage scope.
    pub(super) skills: Vec<SkillDef>,
    /// The item table, looked up by id when a member uses a medicine so its real
    /// HP/SP recovery and status cures apply, rather than a flat placeholder heal.
    pub(super) items: Vec<ItemDef>,
    /// The current battle round, counting from `1`, gating turn-numbered AI.
    pub round: u32,
    /// The party's current escape chance in percent (RM2000 / EasyRPG
    /// `escape_chance`): set once at [`Battle::build`] from the two sides' average
    /// agilities ([`logic::init_escape_chance`]) and raised by 10 on each failed
    /// escape (see [`Battle::attempt_escape`]).
    pub(super) escape_chance: u32,
    /// Whether the party opened with a first strike (RM2000 preemptive attack): it
    /// grants a guaranteed escape and the `+9999` turn-order bonus. No encounter
    /// path sets it yet (there is no ambush/initiative plumbing), so it stays
    /// `false`; the mechanism is honoured wherever the flag is raised.
    pub(super) first_strike: bool,
    pub turn: usize,
    pub menu: MenuLevel,
    pub cursor: usize,
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
    pub(super) pending_anims: Vec<PendingAnim>,
    /// Floating damage/heal numbers queued as the current tick's actions resolve,
    /// drained each frame by `battle::floaters` into rising overlay text and
    /// cleared by [`Battle::new_round`] (a fresh [`Battle::build`] starts empty).
    pub(super) pending_numbers: Vec<PendingNumber>,
    /// Foe screen positions owed a guaranteed per-hit whitening blink, drained by
    /// `battle::scene` into a blink on each struck sprite. Every landed blow
    /// enqueues one, independent of the played animation's own flash timings.
    pub(super) pending_blinks: Vec<(f32, f32)>,
    /// Battle sound effects owed as the current tick's actions resolve (a hit
    /// landed, a foe felled, an attack evaded), drained each frame by `battle.rs`
    /// into `AudioRequest`s named from the loaded `SystemDef` and cleared by
    /// [`Battle::new_round`] (a fresh [`Battle::build`] starts empty).
    pub(super) pending_se: Vec<BattleSe>,
    /// Sub-steps the action currently resolving still owes, drained one per
    /// resolve tick (see [`Step`]) so a multi-target cast staggers its numbers
    /// and a critical announces on its own beat. Cleared by [`Battle::new_round`]
    /// and [`Battle::begin_resolve`].
    pub(super) steps: std::collections::VecDeque<Step>,
    /// While a queued battle animation plays, resolution pauses and the pending
    /// strike/cast impact is held so its damage number lands only once the
    /// animation has finished (RM2000 sequences the animation, its `SetWait`,
    /// then the damage). Set when the animation is queued; `battle::resolve_tick`
    /// drives it down through [`Battle::tick_anim_hold`].
    pub(super) anim_hold: bool,
    /// Whether the held animation has been observed live at least once, so the
    /// hold releases on its disappearance rather than the one-tick lag between
    /// queuing the animation and its `LiveAnimation` overlay appearing.
    pub(super) anim_seen: bool,
    /// Frames the current hold has waited without the animation ever appearing,
    /// bounding the wait for an unknown/absent animation id (see
    /// [`ANIM_HOLD_GRACE_TICKS`]) so resolution can never wedge.
    pub(super) anim_hold_ticks: u32,
    /// Set while a deferred skill/enemy cast re-runs after its animation, so the
    /// shared cast helper skips re-queuing the already-played animation.
    pub(super) suppress_anim: bool,
    /// Set once the victory reward (gold, experience, and any level-ups) has been
    /// paid on entering the outcome, so `battle::apply_victory_rewards` pays out
    /// exactly once while the outcome screen waits for the player.
    pub(super) rewarded: bool,
    /// The real RM2000 battle-end message terms (victory / defeat / escape and the
    /// reward lines), captured from the loaded vocabulary at battle start; the
    /// invented Hungarian defaults stand in until then (see [`super::log_terms`]).
    pub(super) text: super::log_terms::BattleText,
    pub(super) rng: u64,
}

/// Advance a little PCG-style LCG and return its next output. Self-contained so
/// the crate takes on no `rand` dependency for battle variance and AI rolls.
pub(super) fn rng_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) ^ *state
}

impl Battle {
    /// Assemble a fresh encounter: instantiate each troop member as a live
    /// [`Foe`], each party actor as a live [`Fighter`] (HP/SP from `vitals`, or
    /// full on a first fight), and enter the command phase.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        troop: &TroopDef,
        monsters: &[MonsterDef],
        actors: &[&ActorDef],
        items: &[ItemDef],
        attributes: &[AttributeDef],
        states: &[StateDef],
        skills: &[SkillDef],
        vitals: &Vitals,
        progression: &Progression,
        background: String,
        seed: u64,
    ) -> Self {
        let enemies: Vec<Foe> = troop
            .members
            .iter()
            .filter_map(|m| {
                monsters.iter().find(|d| d.id == m.enemy_id).map(|d| Foe {
                    name: d.name.clone(),
                    battler: d.battler.clone(),
                    hp: d.max_hp as i32,
                    max_hp: d.max_hp as i32,
                    stats: Stats::from_monster(d),
                    exp: d.exp,
                    gold: d.gold,
                    x: m.x,
                    y: m.y,
                    attribute_ranks: d.attribute_ranks.clone(),
                    state_ranks: d.state_ranks.clone(),
                    states: Vec::new(),
                    defending: false,
                    fled: false,
                    charging: false,
                    actions: d.actions.clone(),
                    dying: None,
                })
            })
            .collect();
        let members: Vec<Fighter> = actors
            .iter()
            .map(|a| {
                let level = progression.level(a);
                let (max_hp, max_sp) = logic::actor_hp_sp_at(&a.curves, level, a.hp, a.sp);
                let (max_hp, max_sp) = (max_hp as i32, max_sp as i32);
                // Resume stored HP/SP (clamped to the level's maxima), or start
                // full at the current level when this actor has no stored vitals.
                let (hp, sp) = match vitals.get_stored(a.id) {
                    Some((h, s)) => (h.min(max_hp), s.min(max_sp)),
                    None => (max_hp, max_sp),
                };
                let mut stats = logic::actor_stats_at(&a.curves, level);
                let bonus = logic::equipment_bonus(a, items);
                stats.attack += bonus.attack;
                stats.defense += bonus.defense;
                stats.spirit += bonus.spirit;
                stats.agility += bonus.agility;
                // The equipped weapon lends its hit, crit, and first element for
                // later resolution; id `0` matches no real item, so an empty
                // weapon slot resolves to `None`.
                let weapon = items.iter().find(|i| i.id == a.weapon);
                Fighter {
                    actor_id: a.id,
                    name: a.name.clone(),
                    hp,
                    max_hp,
                    sp,
                    max_sp,
                    stats,
                    defending: false,
                    command: None,
                    weapon_hit: weapon.map_or(0, |w| w.hit),
                    weapon_crit: weapon.map_or(0, |w| w.crit),
                    weapon_element: weapon.and_then(|w| w.attribute_defense.first().copied()),
                    // A weapon animates with its own `weapon_animation`; an empty
                    // slot falls back to the actor's bare-handed animation.
                    attack_animation: match weapon {
                        Some(w) => w.weapon_animation,
                        None => a.unarmed_animation,
                    },
                    states: Vec::new(),
                    resist_attributes: logic::equipment_resist(a, items),
                    known_skills: progression.known_skill_ids(a),
                }
            })
            .collect();
        // The RM2000 escape chance is fixed at battle start from the two sides'
        // AVERAGE agilities (EasyRPG `InitEscapeChance`), then only nudged by +10
        // per failed attempt — never recomputed as combatants fall.
        let party_avg =
            logic::average_agility(&members.iter().map(|f| f.stats.agility).collect::<Vec<_>>());
        let enemy_avg =
            logic::average_agility(&enemies.iter().map(|e| e.stats.agility).collect::<Vec<_>>());
        Battle {
            phase: Phase::PartyCommand,
            background,
            members,
            enemies,
            attributes: attributes.to_vec(),
            states: states.to_vec(),
            skills: skills.to_vec(),
            items: items.to_vec(),
            timer: Timer::from_seconds(RESOLVE_STEP_SECS, TimerMode::Repeating),
            log: vec![format!("{} rátok támad!", troop.name)],
            rng: seed | 1,
            generation: seed | 1,
            round: 1,
            escape_chance: logic::init_escape_chance(party_avg, enemy_avg),
            ..default()
        }
    }

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
        if let Some(i) = target {
            self.members[i].command = None;
            self.turn = i;
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
    pub(super) fn begin_resolve(&mut self) {
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
    pub(super) fn begin_anim_hold(&mut self) {
        self.anim_hold = true;
        self.anim_seen = false;
        self.anim_hold_ticks = 0;
    }

    /// Whether resolution is currently paused waiting on a battle animation.
    pub(super) fn anim_hold_active(&self) -> bool {
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
    pub(super) fn tick_anim_hold(&mut self, anim_live: bool) -> bool {
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

#[cfg(test)]
pub(super) mod testkit {
    use super::*;

    pub fn actor(id: u32, level: u32, hp: u32, sp: u32) -> ActorDef {
        ActorDef {
            id,
            name: format!("A{id}"),
            title: String::new(),
            level,
            max_level: 50,
            hp,
            sp,
            curves: Default::default(),
            learnings: Vec::new(),
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
            face_name: String::new(),
            face_index: 0,
        }
    }

    pub fn monster(id: u32, hp: u32, exp: u32, gold: u32) -> MonsterDef {
        MonsterDef {
            id,
            name: format!("M{id}"),
            battler: String::new(),
            max_hp: hp,
            max_sp: 0,
            attack: 20,
            defense: 8,
            spirit: 0,
            agility: 8,
            exp,
            gold,
            attribute_ranks: vec![],
            state_ranks: vec![],
            actions: vec![],
        }
    }

    pub fn item(id: u32, atk: u32, def: u32, hit: u32, crit: u32, element: u32) -> ItemDef {
        ItemDef {
            id,
            name: String::new(),
            description: String::new(),
            item_type: 1,
            price: 0,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            cure_states: vec![],
            scope: 0,
            only_field: false,
            uses: 0,
            atk,
            def,
            spi: 0,
            agi: 0,
            attribute_defense: if element == 0 { vec![] } else { vec![element] },
            state_defense: vec![],
            two_handed: false,
            hit,
            crit,
            weapon_animation: 0,
        }
    }

    pub fn troop(members: &[(u32, u32, u32)]) -> TroopDef {
        TroopDef {
            id: 1,
            name: "T".into(),
            members: members
                .iter()
                .map(|&(enemy_id, x, y)| amnezia_data::TroopMemberDef { enemy_id, x, y })
                .collect(),
        }
    }

    /// One level-2 hero versus two 30-HP bandits, a deterministic fixture.
    pub fn build_1v2() -> Battle {
        let monsters = vec![monster(1, 30, 10, 30)];
        let ron = actor(1, 2, 63, 37);
        let actors = vec![&ron];
        let troop = troop(&[(1, 100, 100), (1, 200, 100)]);
        Battle::build(
            &troop,
            &monsters,
            &actors,
            &[],
            &[],
            &[],
            &[],
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            42,
        )
    }

    /// A two-member party (heroes 1 and 2) versus one 30-HP bandit, for the
    /// ally-target selection tests.
    pub fn build_party2() -> Battle {
        let ron = actor(1, 2, 63, 37);
        let tiff = actor(2, 3, 38, 75);
        let actors = vec![&ron, &tiff];
        let monsters = vec![monster(1, 30, 10, 30)];
        let troop = troop(&[(1, 100, 100)]);
        Battle::build(
            &troop,
            &monsters,
            &actors,
            &[],
            &[],
            &[],
            &[],
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            7,
        )
    }
}

#[cfg(test)]
mod tests;

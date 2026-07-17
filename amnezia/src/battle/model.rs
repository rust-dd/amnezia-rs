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
use amnezia_data::{ActorDef, MonsterDef, TroopDef};
use bevy::prelude::*;

/// Seconds between two resolved actions, so the log and damage read at a human
/// pace rather than flashing past in one frame.
pub const RESOLVE_STEP_SECS: f32 = 0.7;

/// How many trailing log lines the battle keeps for display.
pub const LOG_TAIL: usize = 5;

/// The battle's coarse phase, which gates the input/resolve/outcome systems.
#[derive(Default, PartialEq, Clone, Copy)]
pub enum Phase {
    #[default]
    Inactive,
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
}

impl Fighter {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// A live enemy in the fight: current HP, stats, reward, and its position on the
/// battle backdrop (RM2000 320×240 pixel space).
pub struct Foe {
    pub name: String,
    pub battler: String,
    pub hp: i32,
    pub stats: Stats,
    pub exp: u32,
    pub gold: u32,
    pub x: u32,
    pub y: u32,
}

impl Foe {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// A chosen action, from either side, awaiting resolution. Enemies only ever
/// [`Command::Attack`].
#[derive(Clone, Copy)]
pub enum Command {
    Attack {
        target: usize,
    },
    Skill {
        power: u32,
        cost: u32,
        target: usize,
    },
    Item,
    Defend,
    Nothing,
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

/// The whole live battle, held as a Bevy resource and reset to `default()` (the
/// `Inactive` phase) between fights.
#[derive(Resource, Default)]
pub struct Battle {
    pub phase: Phase,
    pub background: String,
    pub members: Vec<Fighter>,
    pub enemies: Vec<Foe>,
    pub turn: usize,
    pub menu: MenuLevel,
    pub cursor: usize,
    pub pending_skill: Option<(u32, u32)>,
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
    pub fn build(
        troop: &TroopDef,
        monsters: &[MonsterDef],
        actors: &[&ActorDef],
        vitals: &Vitals,
        progression: &Progression,
        background: String,
        seed: u64,
    ) -> Self {
        let enemies = troop
            .members
            .iter()
            .filter_map(|m| {
                monsters.iter().find(|d| d.id == m.enemy_id).map(|d| Foe {
                    name: d.name.clone(),
                    battler: d.battler.clone(),
                    hp: d.max_hp as i32,
                    stats: Stats::from_monster(d),
                    exp: d.exp,
                    gold: d.gold,
                    x: m.x,
                    y: m.y,
                })
            })
            .collect();
        let members = actors
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
                Fighter {
                    actor_id: a.id,
                    name: a.name.clone(),
                    hp,
                    max_hp,
                    sp,
                    max_sp,
                    stats: logic::actor_stats_at(&a.curves, level),
                    defending: false,
                    command: None,
                }
            })
            .collect();
        Battle {
            phase: Phase::Command,
            background,
            members,
            enemies,
            timer: Timer::from_seconds(RESOLVE_STEP_SECS, TimerMode::Repeating),
            log: vec![format!("{} rátok támad!", troop.name)],
            rng: seed | 1,
            generation: seed | 1,
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

    /// Commit `command` for the member currently choosing, then move on: to the
    /// next chooser, or into resolution once every living member has an order.
    pub fn commit(&mut self, command: Command) {
        if let Some(f) = self.members.get_mut(self.turn) {
            f.command = Some(command);
        }
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        match self.next_chooser() {
            Some(i) => self.turn = i,
            None => self.begin_resolve(),
        }
    }

    /// Step back to the previous living chooser, clearing its order (RM2000 back).
    pub fn undo_choice(&mut self) {
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        if let Some(i) = self.members[..self.turn]
            .iter()
            .rposition(|f| f.alive() && f.command.is_some())
        {
            self.members[i].command = None;
            self.turn = i;
        }
    }

    /// Build the agility-ordered turn queue from every member's committed command
    /// plus one attack per living enemy (random living target), and start
    /// resolving.
    fn begin_resolve(&mut self) {
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
            if !self.enemies[i].alive() {
                continue;
            }
            let roll = rng_next(&mut self.rng) as usize;
            if let Some(target) = logic::select_target(&alive, roll) {
                let agility = self.enemies[i].stats.agility;
                actions.push(Action {
                    source: Source::Enemy(i),
                    kind: Command::Attack { target },
                    agility,
                });
            }
        }
        let agilities: Vec<u32> = actions.iter().map(|a| a.agility).collect();
        self.queue = logic::turn_order(&agilities)
            .into_iter()
            .map(|i| actions[i])
            .collect();
        self.queue_at = 0;
        self.timer.reset();
        self.phase = Phase::Resolve;
    }

    /// Open a fresh command round: clear every living member's order and defence.
    pub fn new_round(&mut self) {
        for f in &mut self.members {
            f.command = None;
            f.defending = false;
        }
        self.queue.clear();
        self.queue_at = 0;
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.turn = self.next_chooser().unwrap_or(0);
        self.phase = Phase::Command;
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
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
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
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            42,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::testkit::build_1v2;
    use super::*;

    #[test]
    fn build_instantiates_both_sides_into_the_command_phase() {
        let battle = build_1v2();
        assert_eq!(battle.members.len(), 1);
        assert_eq!(battle.enemies.len(), 2);
        assert_eq!(battle.members[0].hp, 63);
        assert_eq!(battle.enemies[0].hp, 30);
        assert!(battle.phase == Phase::Command);
        assert_eq!(battle.next_chooser(), Some(0));
    }

    #[test]
    fn committing_all_orders_enters_resolution_with_a_full_queue() {
        let mut battle = build_1v2();
        battle.commit(Command::Attack { target: 0 });
        // one party action + two enemy actions, ordered by agility.
        assert!(battle.phase == Phase::Resolve);
        assert_eq!(battle.queue.len(), 3);
    }

    #[test]
    fn undo_choice_steps_back_to_the_previous_committed_member() {
        let ron = super::testkit::actor(1, 2, 63, 37);
        let tiff = super::testkit::actor(2, 3, 38, 75);
        let actors = vec![&ron, &tiff];
        let monsters = vec![super::testkit::monster(1, 30, 10, 30)];
        let troop = super::testkit::troop(&[(1, 100, 100)]);
        let mut battle = Battle::build(
            &troop,
            &monsters,
            &actors,
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            7,
        );
        battle.commit(Command::Defend); // member 0 acts, turn moves to member 1
        assert_eq!(battle.turn, 1);
        battle.undo_choice();
        assert_eq!(battle.turn, 0);
        assert!(battle.members[0].command.is_none());
    }

    #[test]
    fn new_round_clears_orders_and_defence() {
        let mut battle = build_1v2();
        battle.members[0].command = Some(Command::Defend);
        battle.members[0].defending = true;
        battle.new_round();
        assert!(battle.members[0].command.is_none());
        assert!(!battle.members[0].defending);
        assert!(battle.phase == Phase::Command);
    }
}

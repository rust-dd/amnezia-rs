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
    /// The equipped weapon's hit and crit rates (percent) and its element id,
    /// captured at build time and consumed by the to-hit / critical / elemental
    /// resolution in [`super::resolve`]. Empty-handed leaves them `0` / `0` /
    /// `None`; a `0` hit reads as the RM2000 bare-hands 90% default.
    pub weapon_hit: u32,
    pub weapon_crit: u32,
    pub weapon_element: Option<u32>,
    /// This fighter's active status effects as `(state_id, turns_held)` pairs; the
    /// turn count drives [`logic::tick_recovery`]'s hold-then-wear-off schedule.
    pub states: Vec<(u32, u32)>,
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
    /// This foe's RM2000 battle-AI action list, consulted each round to choose
    /// its command (cast a skill, defend, or attack on turn/HP conditions).
    pub actions: Vec<EnemyActionDef>,
}

impl Foe {
    pub fn alive(&self) -> bool {
        self.hp > 0
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

/// A chosen action, from either side, awaiting resolution. Enemies only ever
/// [`Command::Attack`].
#[derive(Clone, Copy)]
pub enum Command {
    Attack { target: usize },
    Skill { skill_id: u32, target: usize },
    Item { item_id: u32 },
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
    pub turn: usize,
    pub menu: MenuLevel,
    pub cursor: usize,
    /// The chosen skill's id while its target is being picked.
    pub pending_skill: Option<u32>,
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
        let enemies = troop
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
                    actions: d.actions.clone(),
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
                    states: Vec::new(),
                }
            })
            .collect();
        Battle {
            phase: Phase::Command,
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
        self.skip_restricted_choosers();
    }

    /// Step back to the previous living chooser, clearing its order (RM2000 back).
    /// Auto-committed restricted members (asleep/berserk/confused) can't be
    /// re-ordered, so the step skips over them to the last freely-chosen member.
    pub fn undo_choice(&mut self) {
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
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
        let agilities: Vec<u32> = actions.iter().map(|a| a.agility).collect();
        self.queue = logic::turn_order(&agilities)
            .into_iter()
            .map(|i| actions[i])
            .collect();
        self.queue_at = 0;
        self.timer.reset();
        self.phase = Phase::Resolve;
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
        self.run_recovery();
        self.queue.clear();
        self.queue_at = 0;
        self.menu = MenuLevel::Command;
        self.cursor = 0;
        self.pending_skill = None;
        self.phase = Phase::Command;
        self.skip_restricted_choosers();
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
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
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
            &[],
            &[],
            &[],
            &[],
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

    #[test]
    fn build_adds_equipment_bonuses_and_captures_the_weapon() {
        let mut ron = testkit::actor(1, 1, 50, 10);
        ron.weapon = 1;
        ron.armor = 2;
        let actors = vec![&ron];
        let items = vec![
            testkit::item(1, 10, 0, 85, 5, 4), // weapon: +10 atk, hit 85, crit 5, element 4
            testkit::item(2, 0, 20, 0, 0, 0),  // armor: +20 def, no weapon fields
        ];
        let monsters = vec![testkit::monster(1, 30, 10, 30)];
        let troop = testkit::troop(&[(1, 100, 100)]);
        let prog = Progression::default();
        let base = logic::actor_stats_at(&ron.curves, prog.level(&ron));
        let battle = Battle::build(
            &troop,
            &monsters,
            &actors,
            &items,
            &[],
            &[],
            &[],
            &Vitals::default(),
            &prog,
            "Cave1".into(),
            1,
        );
        let f = &battle.members[0];
        assert_eq!(f.stats.attack, base.attack + 10);
        assert_eq!(f.stats.defense, base.defense + 20);
        assert_eq!(f.weapon_hit, 85);
        assert_eq!(f.weapon_crit, 5);
        assert_eq!(f.weapon_element, Some(4));
    }

    #[test]
    fn foe_attribute_rank_reads_the_vector_then_defaults_to_neutral_c() {
        let mut battle = build_1v2();
        battle.enemies[0].attribute_ranks = vec![0, 2, 4]; // attrs 1,2,3 -> A, C, E
        let foe = &battle.enemies[0];
        assert_eq!(foe.attribute_rank(1), 0); // A
        assert_eq!(foe.attribute_rank(2), 2); // C
        assert_eq!(foe.attribute_rank(3), 4); // E
        assert_eq!(foe.attribute_rank(4), 2); // past the truncated vector -> C
        assert_eq!(foe.attribute_rank(0), 2); // non-elemental id -> C
    }

    fn state_def(id: u32, restriction: u32, auto_release_prob: u32) -> StateDef {
        StateDef {
            id,
            name: format!("S{id}"),
            restriction,
            priority: 0,
            hold_turn: 0,
            auto_release_prob,
            release_by_damage: 0,
        }
    }

    fn build_2v1(seed: u64) -> Battle {
        let ron = testkit::actor(1, 2, 63, 37);
        let tiff = testkit::actor(2, 3, 38, 75);
        let actors = vec![&ron, &tiff];
        let monsters = vec![testkit::monster(1, 30, 10, 30)];
        let troop = testkit::troop(&[(1, 100, 100)]);
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
            seed,
        )
    }

    #[test]
    fn a_cant_act_member_is_auto_skipped_in_the_command_flow() {
        let mut battle = build_2v1(7);
        // Afflict member 1 with a can't-act (restriction 1) state.
        battle.states = vec![state_def(7, 1, 0)];
        battle.members[1].states = vec![(7, 0)];
        // Member 0 chooses; the flow must auto-order the sleeping member 1 and
        // enter resolution rather than stop for its input.
        battle.commit(Command::Defend);
        assert!(matches!(battle.members[1].command, Some(Command::Nothing)));
        assert!(battle.phase == Phase::Resolve);
    }

    #[test]
    fn new_round_wears_off_a_timed_state_but_never_the_death_state() {
        let mut battle = build_1v2();
        // Death (id 1) is exempt; state 2 lifts at once (hold 0, 100% release).
        battle.states = vec![state_def(1, 0, 100), state_def(2, 0, 100)];
        battle.members[0].states = vec![(1, 0), (2, 0)];
        battle.new_round();
        assert!(logic::has_state(&battle.members[0].states, 1)); // KO status endures
        assert!(!logic::has_state(&battle.members[0].states, 2)); // timed state worn off
    }
}

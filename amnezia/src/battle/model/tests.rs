use super::testkit::build_1v2;
use super::*;

#[test]
fn build_instantiates_both_sides_into_the_command_phase() {
    let battle = build_1v2();
    assert_eq!(battle.members.len(), 1);
    assert_eq!(battle.enemies.len(), 2);
    assert_eq!(battle.members[0].hp, 63);
    assert_eq!(battle.enemies[0].hp, 30);
    assert!(battle.phase == Phase::PartyCommand);
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
        &[testkit::slots(&ron), testkit::slots(&tiff)],
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
fn undo_choice_on_the_first_chooser_reopens_the_party_option_window() {
    // Cancel on the first actor's command backs out of per-actor entry to the
    // Fight/Auto/Escape window — RM2000 SelectPreviousActor on the first ally
    // returns to State_SelectOption rather than staying stuck on the actor.
    let mut battle = build_1v2();
    battle.begin_actor_commands();
    assert!(battle.phase == Phase::Command);
    assert_eq!(battle.turn, 0);
    battle.undo_choice();
    assert!(battle.phase == Phase::PartyCommand);
}

#[test]
fn new_round_clears_orders_and_defence() {
    let mut battle = build_1v2();
    battle.members[0].command = Some(Command::Defend);
    battle.members[0].defending = true;
    battle.new_round();
    assert!(battle.members[0].command.is_none());
    assert!(!battle.members[0].defending);
    assert!(battle.phase == Phase::PartyCommand);
}

#[test]
fn new_round_clears_every_foe_defence() {
    let mut battle = build_1v2();
    for e in &mut battle.enemies {
        e.defending = true;
    }
    battle.new_round();
    assert!(battle.enemies.iter().all(|e| !e.defending));
}

#[test]
fn build_fixes_the_escape_chance_from_the_two_sides_average_agilities() {
    // Ron (level 2, agi 12) vs two bandits (agi 8): party avg 12, enemy avg 8,
    // so 150 - round(100 * 8 / 12 = 66.67 -> 67) = 83.
    let battle = build_1v2();
    assert_eq!(
        battle.escape_chance,
        logic::init_escape_chance(
            battle.members[0].stats.agility,
            battle.enemies[0].stats.agility
        )
    );
    assert_eq!(battle.escape_chance, 83);
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
        &[testkit::slots(&ron)],
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
    assert_eq!(f.weapon_attributes, [4]);
}

#[test]
fn build_reads_the_runtime_loadout_not_the_actor_default() {
    // The actor's ActorDef default weapon is item 1, but the runtime loadout the
    // equip menu produced has swapped in item 2. The fighter must reflect item 2.
    let mut ron = testkit::actor(1, 1, 50, 10);
    ron.weapon = 1;
    let default_weapon = testkit::item(1, 5, 0, 80, 0, 0); // +5 atk, hit 80
    let mut swapped_weapon = testkit::item(2, 30, 0, 95, 0, 0); // +30 atk, hit 95
    swapped_weapon.weapon_animation = 9;
    let items = vec![default_weapon, swapped_weapon];
    let actors = vec![&ron];
    let equipped = [[2u32, 0, 0, 0, 0]]; // weapon slot holds item 2, not the def's 1
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
    let prog = Progression::default();
    let base = logic::actor_stats_at(&ron.curves, prog.level(&ron));
    let battle = Battle::build(
        &troop,
        &monsters,
        &actors,
        &equipped,
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
    assert_eq!(
        f.stats.attack,
        base.attack + 30,
        "the swapped weapon's +30 atk"
    );
    assert_eq!(f.weapon_hit, 95, "the swapped weapon's hit rate");
    assert_eq!(f.attack_animation, 9, "the swapped weapon's animation");
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
        affect_type: 0,
        affect_stats: [false; 4],
        reduce_hit_ratio: 100,
        restrict_skill: false,
        restrict_skill_level: 0,
        restrict_magic: false,
        restrict_magic_level: 0,
        sp_change_type: 0,
        sp_change_max: 0,
        sp_change_val: 0,
        rates: [100, 80, 60, 30, 0],
        persistence: 0,
        id,
        name: format!("S{id}"),
        restriction,
        priority: 0,
        hold_turn: 0,
        auto_release_prob,
        release_by_damage: 0,
        hp_change_type: 0,
        hp_change_max: 0,
        hp_change_val: 0,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
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
        &[testkit::slots(&ron), testkit::slots(&tiff)],
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

/// The turn order for `seed`: the source of each queued action, in order, for a
/// two-member party versus two foes whose agilities are all equalised so only
/// the RM2000 per-round agility jitter can reorder them.
fn turn_order_for(seed: u64) -> Vec<(u8, usize)> {
    let ron = testkit::actor(1, 2, 63, 37);
    let tiff = testkit::actor(2, 3, 38, 75);
    let actors = vec![&ron, &tiff];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100), (1, 200, 100)]);
    let mut battle = Battle::build(
        &troop,
        &monsters,
        &actors,
        &[testkit::slots(&ron), testkit::slots(&tiff)],
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        seed,
    );
    for m in &mut battle.members {
        m.stats.agility = 8;
    }
    for e in &mut battle.enemies {
        e.stats.agility = 8;
    }
    battle.members[0].command = Some(Command::Defend);
    battle.members[1].command = Some(Command::Defend);
    battle.begin_resolve();
    battle
        .queue
        .iter()
        .map(|a| match a.source {
            Source::Party(i) => (0u8, i),
            Source::Enemy(i) => (1u8, i),
        })
        .collect()
}

#[test]
fn turn_order_varies_with_the_rng_yet_is_deterministic_per_seed() {
    // Deterministic per seed: the same seed always yields the same order.
    assert_eq!(turn_order_for(123), turn_order_for(123));
    assert_eq!(turn_order_for(999), turn_order_for(999));
    // Yet identical agilities order differently across seeds, because the
    // per-round jitter is re-rolled each round from the battle RNG.
    let orders: std::collections::HashSet<Vec<(u8, usize)>> =
        (1..60u64).map(turn_order_for).collect();
    assert!(
        orders.len() > 1,
        "randomised turn order should differ across seeds"
    );
}

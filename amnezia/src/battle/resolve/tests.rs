use super::super::model::testkit::{build_1v2, build_party2};
use super::*;

fn fire_attr() -> amnezia_data::AttributeDef {
    amnezia_data::AttributeDef {
        id: 5,
        name: "Tűz".into(),
        attribute_type: 0,
        a_rate: 200, // rank A: weak, double damage
        b_rate: 150,
        c_rate: 100,
        d_rate: 50,
        e_rate: 0, // rank E: immune, no damage
    }
}

/// A single-enemy (scope 0) damage skill carrying `attributes` (elements) and
/// `states` (statuses it may inflict).
fn damage_skill(id: u32, power: u32, attributes: Vec<u32>, states: Vec<u32>) -> SkillDef {
    SkillDef {
        id,
        name: "S".into(),
        description: String::new(),
        sp_cost: 3,
        power,
        hit: 0,
        skill_type: 0,
        scope: 0,
        animation_id: 0,
        physical_rate: 0,
        magical_rate: 3,
        variance: 4,
        affect_hp: true,
        affect_sp: false,
        absorb: false,
        attributes,
        affected_states: states,
    }
}

/// A single-ally (scope 3) HP heal.
fn heal_skill(id: u32, power: u32) -> SkillDef {
    let mut s = damage_skill(id, power, vec![], vec![]);
    s.scope = 3;
    s
}

fn poison_state(id: u32) -> amnezia_data::StateDef {
    amnezia_data::StateDef {
        id,
        name: "Méreg".into(),
        restriction: 0,
        priority: 50,
        hold_turn: 0,
        auto_release_prob: 0,
        release_by_damage: 0,
        hp_change_type: 0,
        hp_change_max: 0,
        hp_change_val: 0,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

fn hp_change_state(
    id: u32,
    hp_change_type: u32,
    hp_change_max: u32,
    hp_change_val: u32,
) -> amnezia_data::StateDef {
    amnezia_data::StateDef {
        id,
        name: "Méreg".into(),
        restriction: 0,
        priority: 50,
        hold_turn: 99,
        auto_release_prob: 0,
        release_by_damage: 0,
        hp_change_type,
        hp_change_max,
        hp_change_val,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

#[test]
fn poison_drains_a_fighter_and_a_foe_by_val_plus_max_percent_at_turn_start() {
    let mut battle = build_1v2();
    // Méreg (state 2): type 0 lose, 5% of max HP + 1 flat, each turn.
    battle.states = vec![hp_change_state(2, 0, 5, 1)];
    battle.members[0].states = vec![(2, 0)];
    battle.enemies[0].states = vec![(2, 0)];
    let (m_max, f_max) = (battle.members[0].max_hp, battle.enemies[0].max_hp);
    let (m_before, f_before) = (battle.members[0].hp, battle.enemies[0].hp);
    battle.tick_state_hp(Source::Party(0));
    battle.tick_state_hp(Source::Enemy(0));
    assert_eq!(m_before - battle.members[0].hp, 1 + m_max * 5 / 100);
    assert_eq!(f_before - battle.enemies[0].hp, 1 + f_max * 5 / 100);
    assert!(battle.log.iter().any(|l| l.contains("Méreg -")));
}

#[test]
fn a_type_2_hp_change_state_drains_nothing() {
    let mut battle = build_1v2();
    // Type 2 = nothing, even with a large configured amount.
    battle.states = vec![hp_change_state(2, 2, 50, 10)];
    battle.members[0].states = vec![(2, 0)];
    let before = battle.members[0].hp;
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(battle.members[0].hp, before);
}

#[test]
fn poison_can_reduce_a_battler_to_zero_and_kill_it() {
    let mut battle = build_1v2();
    // A 100%-of-max drain empties the fighter's HP outright.
    battle.states = vec![hp_change_state(2, 0, 100, 0)];
    battle.members[0].states = vec![(2, 0)];
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(battle.members[0].hp, 0);
    assert!(!battle.members[0].alive());
}

#[test]
fn a_type_1_hp_change_state_regenerates_capped_at_max() {
    let mut battle = build_1v2();
    battle.states = vec![hp_change_state(2, 1, 10, 5)]; // gain 5 + 10% of max
    let max = battle.members[0].max_hp;
    battle.members[0].hp = 10;
    battle.members[0].states = vec![(2, 0)];
    battle.tick_state_hp(Source::Party(0));
    assert_eq!(battle.members[0].hp, (10 + 5 + max * 10 / 100).min(max));
}

#[test]
fn a_party_attack_wounds_its_target_and_logs() {
    let mut battle = build_1v2();
    // Force a guaranteed hit so the wound assertion doesn't ride on the new
    // 90% bare-hands to-hit roll (which would miss 10% of seeds).
    battle.members[0].weapon_hit = 100;
    battle.commit(Command::Attack { target: 0 });
    let before = battle.enemies[0].hp;
    while battle.resolve_next() {}
    assert!(battle.enemies[0].hp < before);
    assert!(battle.log.iter().any(|l| l.contains("rácsap")));
}

/// One hero with a weapon whose `weapon_animation` is 7, versus a lone foe
/// placed at RM2000 (100, 100), for the attack-animation queue tests.
fn build_weapon_anim(weapon_animation: u32) -> Battle {
    use super::super::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let mut ron = testkit::actor(1, 2, 63, 37);
    ron.weapon = 1;
    let mut sword = testkit::item(1, 10, 0, 100, 0, 0);
    sword.weapon_animation = weapon_animation;
    let actors = vec![&ron];
    let items = vec![sword];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
    Battle::build(
        &troop,
        &monsters,
        &actors,
        &items,
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        1,
    )
}

#[test]
fn a_party_strike_queues_the_weapon_animation_on_the_struck_foe() {
    let mut battle = build_weapon_anim(7);
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    // The member's swing queued its weapon animation (7) at the foe's screen
    // slot: x = foe.x - 160, y = foe.y - 120 (foe at 100, 100).
    let anim = battle
        .pending_anims
        .iter()
        .find(|a| a.anim_id == 7)
        .expect("the weapon attack animation should be queued");
    assert_eq!(anim.targets.len(), 1, "one target for the struck foe");
    let (x, y) = anim.targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_unarmed_party_strike_falls_back_to_the_actor_unarmed_animation() {
    use super::super::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let mut ron = testkit::actor(1, 2, 63, 37);
    ron.weapon = 0; // bare-handed
    ron.unarmed_animation = 3;
    let actors = vec![&ron];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
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
        1,
    );
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    assert!(
        battle.pending_anims.iter().any(|a| a.anim_id == 3),
        "the unarmed attacker should queue its unarmed_animation"
    );
}

#[test]
fn a_zero_animation_attacker_queues_nothing() {
    // build_1v2's hero is bare-handed with unarmed_animation 0, so its swing
    // queues no member animation, and rpg2k enemy normal attacks play none
    // either — so a full round queues nothing, and id 0 is never pushed.
    let mut battle = build_1v2();
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    assert!(battle.pending_anims.iter().all(|a| a.anim_id != 0));
}

#[test]
fn an_enemy_normal_attack_queues_no_animation() {
    // An rpg2k enemy normal attack shows no animation, so a strike queues
    // nothing regardless of whether the to-hit roll lands.
    let mut battle = build_1v2();
    battle.enemy_strike_member(0, 0);
    assert!(
        battle.pending_anims.is_empty(),
        "an enemy normal attack plays no animation"
    );
}

#[test]
fn new_round_clears_the_pending_animation_queue() {
    let mut battle = build_1v2();
    battle.pending_anims.push(PendingAnim {
        anim_id: 5,
        targets: vec![(1.0, 2.0)],
    });
    battle.new_round();
    assert!(battle.pending_anims.is_empty());
}

#[test]
fn a_forced_critical_triples_the_blow() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.members[0].weapon_crit = 100; // always crit
    let base = logic::physical_damage(
        battle.members[0].stats.attack,
        battle.enemies[0].stats.defense,
    );
    let Strike::Hit { dmg, crit } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    assert!(crit);
    // var=4 spreads the tripled base by up to ±20%; the crit clearly beats a
    // plain blow either way.
    let tripled = base * 3;
    assert!(dmg >= tripled - tripled * 2 / 10 && dmg <= tripled + tripled * 2 / 10 + 1);
    assert!(dmg > base);
}

#[test]
fn a_missed_strike_deals_no_damage() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90; // bare-hands default
    // The effective chance is the base hit adjusted by the agility gap; wind
    // the rng to a state whose next to-hit roll falls in that miss band.
    let hit = logic::to_hit(
        logic::effective_hit(battle.members[0].weapon_hit),
        battle.members[0].stats.agility,
        battle.enemies[0].stats.agility,
    );
    loop {
        let mut probe = battle.rng;
        if (rng_next(&mut probe) % 100) as i32 >= hit {
            break;
        }
        rng_next(&mut battle.rng);
    }
    let before = battle.enemies[0].hp;
    let strike = battle.strike_enemy(0, 0);
    assert!(matches!(strike, Strike::Miss));
    assert_eq!(battle.enemies[0].hp, before); // a miss deals 0
}

#[test]
fn an_elemental_strike_amplifies_against_a_weak_foe() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.members[0].weapon_crit = 0; // never crit — isolate the element
    battle.members[0].weapon_element = Some(5); // fire
    battle.attributes = vec![fire_attr()];
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 0]; // attr 5 -> rank A (weak)
    let base = logic::physical_damage(
        battle.members[0].stats.attack,
        battle.enemies[0].stats.defense,
    );
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    assert!(dmg > base); // a weak (A) rank amplifies past the plain hit
}

#[test]
fn an_elemental_strike_is_nullified_by_an_immune_foe() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.members[0].weapon_element = Some(5); // fire
    battle.attributes = vec![fire_attr()];
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 4]; // attr 5 -> rank E (0%)
    let before = battle.enemies[0].hp;
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    assert_eq!(dmg, 0); // immune (E, 0%) nullifies the blow
    assert_eq!(battle.enemies[0].hp, before);
}

#[test]
fn killing_every_enemy_yields_victory_and_summed_reward() {
    let mut battle = build_1v2();
    for e in &mut battle.enemies {
        e.hp = 0;
    }
    assert!(matches!(battle.end_state(), Some(BattleOutcome::Victory)));
    assert_eq!(battle.victory_rewards(), (20, 60)); // 10+10 exp, 30+30 gold
}

#[test]
fn wiping_the_party_yields_defeat() {
    let mut battle = build_1v2();
    battle.members[0].hp = 0;
    assert!(matches!(battle.end_state(), Some(BattleOutcome::Defeat)));
}

#[test]
fn retargeting_skips_a_dead_enemy() {
    let mut battle = build_1v2();
    battle.enemies[0].hp = 0; // first target dead
    assert_eq!(battle.retarget_enemy(0), Some(1)); // falls through to the living one
}

#[test]
fn defend_halves_the_hit_a_member_takes() {
    let mut battle = build_1v2();
    battle.members[0].defending = true;
    let full = battle.members[0].hp;
    // base 8 with var=4 spreads to [7,10], halved by defence to [3,5].
    battle.hit_member(0, 8, 4);
    let taken = full - battle.members[0].hp;
    assert!((3..=5).contains(&taken));
}

#[test]
fn finish_victory_records_reward_and_log() {
    let mut battle = build_1v2();
    battle.finish(BattleOutcome::Victory);
    assert!(battle.phase == Phase::Outcome);
    assert_eq!(battle.reward_gold, 60);
    assert!(battle.log_tail().contains("Győzelem"));
}

#[test]
fn a_fire_skill_amplifies_against_a_weak_foe() {
    let mut battle = build_1v2();
    battle.attributes = vec![fire_attr()]; // element id 5
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 0]; // attr 5 -> rank A (weak)
    battle.skills = vec![damage_skill(1, 50, vec![5], vec![])];
    let plain = logic::skill_effect(
        &battle.skills[0],
        &battle.members[0].stats,
        &battle.enemies[0].stats,
        true,
    );
    let before = battle.enemies[0].hp;
    battle.commit(Command::Skill {
        skill_id: 1,
        target: 0,
    });
    while battle.resolve_next() {}
    let dealt = before - battle.enemies[0].hp;
    assert!(
        dealt > plain,
        "fire against a weak (A) foe should beat the plain base ({dealt} <= {plain})"
    );
}

#[test]
fn an_ally_heal_raises_a_wounded_ally_clamped_to_max() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)]; // scope 3, power 40
    let max = battle.members[0].max_hp;
    battle.members[0].hp = max - 3; // wounded, within one heal of full
    battle.cast_skill(0, 2, 0);
    assert_eq!(battle.members[0].hp, max); // healed past the deficit, clamped to max
}

#[test]
fn a_damage_skill_inflicts_its_state_on_a_forced_hit_roll() {
    let mut battle = build_1v2();
    battle.states = vec![poison_state(3)];
    battle.enemies[0].hp = 200; // survive the blow so the status lands on a live foe
    battle.enemies[0].state_ranks = vec![2, 2, 0]; // state 3 -> rank A (100% infliction)
    battle.skills = vec![damage_skill(1, 20, vec![], vec![3])];
    battle.cast_skill(0, 1, 0);
    assert!(logic::has_state(&battle.enemies[0].states, 3));
}

#[test]
fn an_enemy_skill_cast_wounds_the_targeted_member() {
    let mut battle = build_1v2();
    battle.skills = vec![damage_skill(1, 30, vec![], vec![])]; // scope 0 -> hits a member
    let before = battle.members[0].hp;
    let line = battle.enemy_cast(0, 1, 0).unwrap();
    assert!(battle.members[0].hp < before);
    assert!(line.contains("varázsol"));
}

#[test]
fn an_enemy_ally_scope_skill_heals_the_caster_clamped_to_max() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)]; // scope 3 -> caster heals itself
    battle.enemies[0].hp = battle.enemies[0].max_hp - 5; // wounded, within one heal of full
    battle.enemy_cast(0, 2, 0);
    assert_eq!(battle.enemies[0].hp, battle.enemies[0].max_hp);
}

#[test]
fn a_single_target_skill_queues_one_animation_at_the_targeted_foe() {
    let mut battle = build_1v2(); // foes at (100, 100) and (200, 100)
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.animation_id = 9;
    battle.skills = vec![s];
    battle.cast_skill(0, 1, 0); // scope 0 -> the targeted foe (index 0)
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 9)
        .collect();
    assert_eq!(hits.len(), 1, "one animation for the one struck foe");
    assert_eq!(hits[0].targets.len(), 1, "one target");
    // foe 0 at (100, 100): x = 100 - 160 = -60, y = 100 - 120 = -20.
    let (x, y) = hits[0].targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_all_enemy_skill_queues_one_animation_over_every_living_foe() {
    let mut battle = build_1v2(); // two living foes, at x = 100 and x = 200
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.scope = 1; // all enemies
    s.animation_id = 8;
    battle.skills = vec![s];
    battle.cast_skill(0, 1, 0);
    // One queued animation for the whole cast (its SE fires once), carrying
    // every living foe as a target so its cells and flashes reach each.
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 8)
        .collect();
    assert_eq!(hits.len(), 1, "one animation for the multi-target cast");
    let xs: Vec<f32> = hits[0].targets.iter().map(|&(x, _)| x).collect();
    assert_eq!(xs.len(), 2, "one target per living foe");
    assert!(xs.iter().any(|x| (x + 60.0).abs() < 1e-6), "foe at x=100");
    assert!(xs.iter().any(|x| (x - 40.0).abs() < 1e-6), "foe at x=200");
}

#[test]
fn an_all_ally_heal_queues_one_animation_per_living_member_at_the_party_area() {
    let mut battle = build_party2(); // two living members
    let mut s = heal_skill(2, 30);
    s.scope = 4; // all allies
    s.animation_id = 5;
    battle.skills = vec![s];
    battle.cast_skill(0, 2, 0);
    // One queued animation for the whole cast, carrying every living ally.
    let heals: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 5)
        .collect();
    assert_eq!(heals.len(), 1, "one animation for the multi-target heal");
    let slots = &heals[0].targets;
    assert_eq!(slots.len(), 2, "one target per living ally");
    assert!(
        slots.iter().all(|&(_, y)| (y - 80.0).abs() < 1e-6),
        "each plays at the party-area y"
    );
    assert!(
        (slots[0].0 - slots[1].0).abs() > 1e-6,
        "the two members' slots are spread apart"
    );
}

#[test]
fn a_zero_animation_skill_queues_nothing() {
    let mut battle = build_1v2();
    // damage_skill leaves animation_id at its 0 default.
    battle.skills = vec![damage_skill(1, 20, vec![], vec![])];
    battle.cast_skill(0, 1, 0);
    assert!(
        battle.pending_anims.is_empty(),
        "a 0 animation id queues nothing"
    );
}

#[test]
fn an_enemy_damage_cast_queues_the_animation_at_the_targeted_member_slot() {
    let mut battle = build_1v2(); // one member
    let mut s = damage_skill(1, 30, vec![], vec![]); // scope 0 -> hits a member
    s.animation_id = 6;
    battle.skills = vec![s];
    battle.enemy_cast(0, 1, 0);
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 6)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].targets.len(), 1);
    // The lone member's party slot: centred x, party-area y.
    let (x, y) = hits[0].targets[0];
    assert!(x.abs() < 1e-6, "x = {x}");
    assert!((y - 80.0).abs() < 1e-6, "y = {y}");
}

#[test]
fn an_enemy_ally_scope_cast_queues_the_animation_on_the_casting_foe() {
    let mut battle = build_1v2(); // caster foe 0 at (100, 100)
    let mut s = heal_skill(2, 40); // scope 3 -> the foe heals itself
    s.animation_id = 4;
    battle.skills = vec![s];
    battle.enemy_cast(0, 2, 0);
    let hits: Vec<_> = battle
        .pending_anims
        .iter()
        .filter(|a| a.anim_id == 4)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].targets.len(), 1);
    let (x, y) = hits[0].targets[0];
    assert!((x + 60.0).abs() < 1e-6, "x = {x}");
    assert!((y + 20.0).abs() < 1e-6, "y = {y}");
}

fn damage_release_state(id: u32) -> amnezia_data::StateDef {
    amnezia_data::StateDef {
        id,
        name: "Bódulat".into(),
        restriction: 0,
        priority: 0,
        hold_turn: 0,
        auto_release_prob: 0,
        release_by_damage: 100,
        hp_change_type: 0,
        hp_change_max: 0,
        hp_change_val: 0,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

fn confusion_state(id: u32) -> amnezia_data::StateDef {
    amnezia_data::StateDef {
        id,
        name: "Zavar".into(),
        restriction: 3,
        priority: 0,
        hold_turn: 0,
        auto_release_prob: 0,
        release_by_damage: 0,
        hp_change_type: 0,
        hp_change_max: 0,
        hp_change_val: 0,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

#[test]
fn being_hit_wears_off_a_damage_release_state_but_never_death() {
    let mut battle = build_1v2();
    // id 1 is the death state (exempt); id 2 shakes off on any hit.
    battle.states = vec![poison_state(1), damage_release_state(2)];
    battle.members[0].states = vec![(1, 0), (2, 0)];
    battle.hit_member(0, 8, 4);
    assert!(logic::has_state(&battle.members[0].states, 1)); // KO exempt
    assert!(!logic::has_state(&battle.members[0].states, 2)); // lifted by the blow
}

#[test]
fn a_confused_member_turns_on_a_living_ally() {
    use super::super::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let ron = testkit::actor(1, 5, 80, 40);
    let tiff = testkit::actor(2, 5, 80, 40);
    let actors = vec![&ron, &tiff];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
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
        3,
    );
    battle.states = vec![confusion_state(9)];
    battle.members[0].states = vec![(9, 0)]; // member 0 is confused
    let ally_hp = battle.members[1].hp;
    // The command flow auto-orders the confused member to strike an ally.
    battle.skip_restricted_choosers();
    assert!(matches!(
        battle.members[0].command,
        Some(Command::Attack { .. })
    ));
    battle.commit(Command::Defend); // member 1 (free) finishes the round
    while battle.resolve_next() {}
    assert!(battle.members[1].hp < ally_hp);
}

fn medicine(
    id: u32,
    recover_hp: u32,
    recover_sp: u32,
    cure_states: Vec<u32>,
) -> amnezia_data::ItemDef {
    amnezia_data::ItemDef {
        id,
        name: "Gyógyfű".into(),
        description: String::new(),
        item_type: 6,
        price: 0,
        recover_hp,
        recover_hp_rate: 0,
        recover_sp,
        recover_sp_rate: 0,
        cure_states,
        scope: 0,
        only_field: false,
        uses: 0,
        atk: 0,
        def: 0,
        spi: 0,
        agi: 0,
        attribute_defense: vec![],
        state_defense: vec![],
        two_handed: false,
        hit: 0,
        crit: 0,
        weapon_animation: 0,
    }
}

#[test]
fn a_recover_hp_item_raises_the_users_hp_clamped_to_max() {
    let mut battle = build_1v2();
    let max = battle.members[0].max_hp;
    // A flat +20 raises a wounded user by exactly 20.
    battle.items = vec![medicine(50, 20, 0, vec![])];
    battle.members[0].hp = 10;
    let line = battle.apply_item(0, 50, 0);
    assert_eq!(battle.members[0].hp, 30);
    assert!(line.contains("+20 HP"));
    // A heal that overshoots the maximum clamps to it.
    battle.members[0].hp = max - 5;
    battle.items = vec![medicine(51, 200, 0, vec![])];
    battle.apply_item(0, 51, 0);
    assert_eq!(battle.members[0].hp, max);
}

#[test]
fn a_cure_states_item_lifts_that_state_from_the_user() {
    let mut battle = build_1v2();
    battle.states = vec![poison_state(3)];
    battle.members[0].states = vec![(3, 0)];
    battle.items = vec![medicine(60, 0, 0, vec![3])];
    let line = battle.apply_item(0, 60, 0);
    assert!(!logic::has_state(&battle.members[0].states, 3));
    assert!(line.contains("gyógyul"));
}

#[test]
fn an_item_used_on_an_ally_heals_that_member_not_the_caster() {
    let mut battle = build_party2();
    battle.items = vec![medicine(50, 20, 0, vec![])];
    let max = battle.members[1].max_hp;
    battle.members[1].hp = (max - 25).max(0);
    let before_ally = battle.members[1].hp;
    let before_caster = battle.members[0].hp;
    let line = battle.apply_item(0, 50, 1); // caster 0 uses the item on ally 1
    assert_eq!(battle.members[1].hp, (before_ally + 20).min(max));
    assert_eq!(battle.members[0].hp, before_caster); // the caster is untouched
    assert!(line.contains("+20 HP"));
}

/// An always-eligible enemy AI entry of the given `basic` code.
fn enemy_action_def(basic: u32) -> amnezia_data::EnemyActionDef {
    amnezia_data::EnemyActionDef {
        kind: 0,
        basic,
        skill_id: 0,
        enemy_id: 0,
        condition_type: 0, // always eligible
        condition_min: 0,
        condition_max: 0,
        priority: 1,
    }
}

#[test]
fn a_defending_foe_takes_half_of_an_identical_strike() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.members[0].weapon_crit = 0; // never crit — isolate the halving
    // Foe 0 (open) and foe 1 (defending) are identical bandits; strike each
    // from the same RNG state so only the Defend stance differs.
    let rng_save = battle.rng;
    let Strike::Hit { dmg: full, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    battle.rng = rng_save;
    battle.enemies[1].defending = true;
    let Strike::Hit { dmg: half, .. } = battle.strike_enemy(0, 1) else {
        panic!("a forced-hit strike missed");
    };
    assert!(half < full);
    assert_eq!(half, (full / 2).max(1));
}

#[test]
fn a_defending_enemy_guards_and_deals_no_damage_that_turn() {
    let mut battle = build_1v2();
    for e in &mut battle.enemies {
        e.actions = vec![enemy_action_def(2)]; // basic 2 = defend
    }
    let hp_before = battle.members[0].hp;
    battle.commit(Command::Defend); // the lone member defends -> resolution
    while battle.resolve_next() {}
    assert!(battle.enemies.iter().all(|e| e.defending));
    assert_eq!(battle.members[0].hp, hp_before); // no foe attacked
    assert!(battle.log.iter().any(|l| l.contains("védekezik")));
}

/// Advance `battle.rng` until the enemy to-hit rolls at the given draw
/// `offsets` all land (roll % 100 < 50, comfortably under the agility-adjusted
/// enemy hit here), so a strike-count comparison is not spoiled by a miss.
fn wind_enemy_hits(battle: &mut Battle, offsets: &[usize]) {
    let span = offsets.iter().copied().max().map_or(0, |m| m + 1);
    loop {
        let mut probe = battle.rng;
        let draws: Vec<u64> = (0..span).map(|_| rng_next(&mut probe)).collect();
        if offsets.iter().all(|&o| draws[o] % 100 < 50) {
            return;
        }
        rng_next(&mut battle.rng);
    }
}

#[test]
fn a_double_attack_strikes_the_target_twice() {
    let mut battle = build_1v2();
    let hp0 = battle.members[0].hp;
    // Land both to-hit rolls (draw 0 for the single blow, draws 0 and 2 for
    // the double) so the comparison reflects strike count, not a chance miss.
    wind_enemy_hits(&mut battle, &[0, 2]);
    let rng_save = battle.rng;
    // Baseline: a single enemy strike from this RNG state.
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    let single = hp0 - battle.members[0].hp;
    assert!(single > 0);
    // Same RNG state, but a double-attack: the first blow matches `single`,
    // the second adds more, so the total clearly exceeds one strike.
    battle.rng = rng_save;
    battle.members[0].hp = hp0;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::DoubleAttack { target: 0 },
        agility: 0,
    });
    let double = hp0 - battle.members[0].hp;
    assert!(
        double > single,
        "double-attack ({double}) should exceed a single strike ({single})"
    );
}

#[test]
fn self_destruct_hits_every_member_then_kills_the_foe() {
    let mut battle = build_party2(); // 2 members, 1 foe (attack 20)
    let hp = [battle.members[0].hp, battle.members[1].hp];
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::SelfDestruct,
        agility: 0,
    });
    assert!(battle.members[0].hp < hp[0]);
    assert!(battle.members[1].hp < hp[1]);
    assert_eq!(battle.enemies[0].hp, 0);
    assert!(!battle.enemies[0].alive());
    assert!(battle.log.iter().any(|l| l.contains("felrobban")));
}

#[test]
fn an_escaping_foe_leaves_battle_and_grants_no_reward() {
    let mut battle = build_1v2(); // 2 foes, each 10 exp / 30 gold
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Escape,
        agility: 0,
    });
    assert!(battle.enemies[0].fled);
    assert!(!battle.enemies[0].alive());
    assert!(!battle.living_enemies().contains(&0));
    // Defeat the remaining foe: victory pays only for the one truly beaten.
    battle.enemies[1].hp = 0;
    assert!(matches!(battle.end_state(), Some(BattleOutcome::Victory)));
    assert_eq!(battle.victory_rewards(), (10, 30));
    assert!(battle.log.iter().any(|l| l.contains("elmenekül")));
}

#[test]
fn a_charged_foe_doubles_its_next_strike_then_clears() {
    let mut battle = build_1v2();
    let hp0 = battle.members[0].hp;
    wind_enemy_hits(&mut battle, &[0]); // land the single to-hit roll
    let rng_save = battle.rng;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    let normal = hp0 - battle.members[0].hp;
    assert!(normal > 0);
    // Same RNG, but the foe has charged: the strike lands double, then clears.
    battle.rng = rng_save;
    battle.members[0].hp = hp0;
    battle.enemies[0].charging = true;
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::Attack { target: 0 },
        agility: 0,
    });
    let charged = hp0 - battle.members[0].hp;
    assert!(
        charged > normal,
        "charged strike ({charged}) should exceed a normal one ({normal})"
    );
    assert!(!battle.enemies[0].charging); // consumed by the strike
}

#[test]
fn equipped_element_defence_halves_a_matching_enemy_skill_only() {
    use super::super::model::testkit;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    let mut ron = testkit::actor(1, 3, 200, 50);
    ron.armor = 2; // equip armor guarding attribute 5
    let actors = vec![&ron];
    let items = vec![testkit::item(2, 0, 0, 0, 0, 5)];
    let monsters = vec![testkit::monster(1, 30, 10, 30)];
    let troop = testkit::troop(&[(1, 100, 100)]);
    let mut battle = Battle::build(
        &troop,
        &monsters,
        &actors,
        &items,
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        5,
    );
    assert!(battle.members[0].resist_attributes.contains(&5));
    // With the skill's variance set to 0 the halving is exact: the guarded
    // cast deals base/2 and the unguarded cast deals the full base, so the two
    // compare cleanly without depending on the variance draw.
    let before = battle.members[0].hp;
    // A guarded (attribute 5) enemy skill is halved (before variance).
    let mut guarded = damage_skill(1, 40, vec![5], vec![]);
    guarded.variance = 0;
    battle.skills = vec![guarded];
    battle.enemy_cast(0, 1, 0);
    let resisted = before - battle.members[0].hp;
    // The same skill on an unguarded element (6) lands full.
    battle.members[0].hp = before;
    let mut unguarded = damage_skill(1, 40, vec![6], vec![]);
    unguarded.variance = 0;
    battle.skills = vec![unguarded];
    battle.enemy_cast(0, 1, 0);
    let full = before - battle.members[0].hp;
    assert!(resisted < full);
    assert_eq!(resisted, (full / 2).max(1));
}

#[test]
fn a_landed_strike_pops_a_damage_number_at_the_foe() {
    let mut battle = build_1v2(); // foe 0 at (100, 100)
    battle.members[0].weapon_hit = 100; // never miss
    let Strike::Hit { dmg, .. } = battle.strike_enemy(0, 0) else {
        panic!("a forced-hit strike missed");
    };
    let pos = battle.foe_anim_pos(0);
    let (text, kind) = number_for(dmg);
    let number = battle
        .pending_numbers
        .last()
        .expect("a landed hit pops a floating number");
    assert_eq!(number.pos, pos);
    assert_eq!(number.text, text);
    assert!(number.kind == kind);
}

#[test]
fn a_heal_pops_a_heal_coloured_number() {
    let mut battle = build_1v2();
    battle.skills = vec![heal_skill(2, 40)]; // scope 3 HP heal
    battle.members[0].hp = 10;
    battle.cast_skill(0, 2, 0);
    assert!(
        battle
            .pending_numbers
            .iter()
            .any(|n| n.kind == NumberKind::Heal),
        "a heal enqueues a heal-coloured number"
    );
}

#[test]
fn a_missed_strike_pops_a_miss_number() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90;
    let hit = logic::to_hit(
        logic::effective_hit(battle.members[0].weapon_hit),
        battle.members[0].stats.agility,
        battle.enemies[0].stats.agility,
    );
    loop {
        let mut probe = battle.rng;
        if (rng_next(&mut probe) % 100) as i32 >= hit {
            break;
        }
        rng_next(&mut battle.rng);
    }
    assert!(matches!(battle.strike_enemy(0, 0), Strike::Miss));
    let number = battle.pending_numbers.last().expect("a miss pops a number");
    assert_eq!(number.text, "Miss");
    assert!(number.kind == NumberKind::Miss);
}

#[test]
fn a_foe_hit_enqueues_a_guaranteed_blink_without_any_animation_flash() {
    // build_1v2's hero is bare-handed (unarmed_animation 0), so its swing plays
    // no animation and carries no flash timing — yet the struck foe blinks.
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    assert!(battle.pending_blinks.is_empty());
    battle.strike_enemy(0, 0);
    let pos = battle.foe_anim_pos(0);
    assert_eq!(battle.pending_blinks.first().copied(), Some(pos));
}

#[test]
fn felling_a_foe_starts_a_death_that_holds_resolve_then_clears() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.enemies[0].hp = 1; // one blow from death
    battle.strike_enemy(0, 0);
    assert!(!battle.enemies[0].alive());
    assert!(battle.enemies[0].dying.is_some());
    assert!(battle.death_in_progress()); // resolve_tick holds while this is true
    battle.advance_deaths(DEATH_SECS + 0.1); // let the beat play out
    assert!(!battle.death_in_progress()); // hold released
    assert!(!battle.enemies[0].alive()); // and the foe is gone for good
}

#[test]
fn self_destruct_starts_an_explosion_death_out() {
    let mut battle = build_party2(); // one foe
    battle.apply(Action {
        source: Source::Enemy(0),
        kind: Command::SelfDestruct,
        agility: 0,
    });
    let dying = battle.enemies[0]
        .dying
        .as_ref()
        .expect("a self-destruct explodes");
    assert!(dying.explode);
    assert!(battle.death_in_progress());
}

#[test]
fn a_multi_target_cast_staggers_its_damage_numbers_across_ticks() {
    let mut battle = build_1v2(); // two foes
    let mut s = damage_skill(1, 20, vec![], vec![]);
    s.scope = 1; // all enemies
    battle.skills = vec![s];
    // The foes survive the cast (so no death-hold) and only defend, so the sole
    // damage numbers each tick come from the staggered cast, not enemy attacks.
    for e in &mut battle.enemies {
        e.hp = 500;
        e.actions = vec![enemy_action_def(2)]; // basic 2 = defend
    }
    battle.commit(Command::Skill {
        skill_id: 1,
        target: 0,
    });
    // Drive resolution tick by tick, counting how many floating numbers land
    // each tick. The all-enemy cast must spread its two numbers over two ticks.
    let mut per_tick: Vec<usize> = Vec::new();
    let mut prev = battle.pending_numbers.len();
    while battle.resolve_next() {
        let now = battle.pending_numbers.len();
        per_tick.push(now - prev);
        prev = now;
    }
    let ticks_with_a_number = per_tick.iter().filter(|&&c| c > 0).count();
    assert!(
        ticks_with_a_number >= 2,
        "an all-enemy cast should stagger its numbers across ticks, got {per_tick:?}"
    );
    assert_eq!(
        per_tick.iter().sum::<usize>(),
        2,
        "exactly the two foe damage numbers landed, one per tick"
    );
}

#[test]
fn a_critical_announces_on_its_own_line_before_the_damage_line() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.members[0].weapon_crit = 100; // always crit
    // Foes survive and only defend, so no enemy lines crowd the log.
    for e in &mut battle.enemies {
        e.hp = 500;
        e.actions = vec![enemy_action_def(2)];
    }
    battle.commit(Command::Attack { target: 0 });
    while battle.resolve_next() {}
    // "Kritikus!" is emitted as its own distinct log line.
    assert!(
        battle.log.iter().any(|l| l == "Kritikus!"),
        "a critical announces on its own line, log: {:?}",
        battle.log
    );
    // The damage line follows separately and no longer folds in the crit word.
    let damage_line = battle
        .log
        .iter()
        .find(|l| l.contains("rácsap:") && l.contains('-'))
        .expect("the critical's damage line");
    assert!(
        !damage_line.contains("Kritikus"),
        "the damage line is a separate beat: {damage_line}"
    );
}

#[test]
fn a_landed_foe_hit_enqueues_the_enemy_damaged_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    assert!(matches!(battle.strike_enemy(0, 0), Strike::Hit { .. }));
    assert!(
        battle.pending_se.contains(&BattleSe::EnemyDamaged),
        "a landed blow queues the enemy-damaged SE: {:?}",
        battle.pending_se
    );
    // A 30-HP foe survives one blow, so no kill SE yet.
    assert!(!battle.pending_se.contains(&BattleSe::EnemyDefeated));
}

#[test]
fn felling_a_foe_enqueues_the_enemy_defeated_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 100; // never miss
    battle.enemies[0].hp = 1; // one blow from death
    battle.strike_enemy(0, 0);
    assert!(!battle.enemies[0].alive());
    assert!(
        battle.pending_se.contains(&BattleSe::EnemyDefeated),
        "felling a foe queues the kill SE: {:?}",
        battle.pending_se
    );
    // The damage SE still fires for the killing blow itself.
    assert!(battle.pending_se.contains(&BattleSe::EnemyDamaged));
}

#[test]
fn a_missed_strike_enqueues_the_dodge_se() {
    let mut battle = build_1v2();
    battle.members[0].weapon_hit = 90; // bare-hands default
    let hit = logic::to_hit(
        logic::effective_hit(battle.members[0].weapon_hit),
        battle.members[0].stats.agility,
        battle.enemies[0].stats.agility,
    );
    // Wind the rng to a state whose next to-hit roll lands in the miss band.
    loop {
        let mut probe = battle.rng;
        if (rng_next(&mut probe) % 100) as i32 >= hit {
            break;
        }
        rng_next(&mut battle.rng);
    }
    assert!(matches!(battle.strike_enemy(0, 0), Strike::Miss));
    assert!(
        battle.pending_se.contains(&BattleSe::Dodge),
        "an evaded blow queues the dodge SE: {:?}",
        battle.pending_se
    );
    // A pure miss lands nothing, so no damage SE.
    assert!(!battle.pending_se.contains(&BattleSe::EnemyDamaged));
}

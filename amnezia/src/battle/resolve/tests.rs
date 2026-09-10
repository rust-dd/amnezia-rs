use super::*;
use crate::battle::model::testkit::{build_1v2, build_party2};

mod animation_mode;
mod attributes;
mod basic_messages;
mod charge;
mod criticals;
mod drops;
mod equipment;
mod impact_rules;
mod item_messages;
mod messages;
mod skill_requirements;
mod state_behavior;
mod state_messages;

fn fire_attr() -> amnezia_data::AttributeDef {
    amnezia_data::AttributeDef {
        id: 5,
        name: "Tűz".into(),
        attribute_type: 1,
        a_rate: 200,
        b_rate: 150,
        c_rate: 100,
        d_rate: 50,
        e_rate: 0,
    }
}

/// A single-enemy (scope 0) damage skill carrying `attributes` (elements) and
/// `states` (statuses it may inflict). Its `hit` is 100 for deterministic damage.
fn damage_skill(id: u32, power: u32, attributes: Vec<u32>, states: Vec<u32>) -> SkillDef {
    SkillDef {
        using_message1: String::new(),
        using_message2: String::new(),
        affect_stats: [false; 4],
        ignore_defense: false,
        id,
        name: "S".into(),
        description: String::new(),
        sp_cost: 3,
        power,
        hit: 100,
        skill_type: 0,
        failure_message: 0,
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
        color: 6,
        message_actor: String::new(),
        message_enemy: String::new(),
        message_already: String::new(),
        message_affected: String::new(),
        message_recovery: String::new(),
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
        color: 6,
        message_actor: String::new(),
        message_enemy: String::new(),
        message_already: String::new(),
        message_affected: String::new(),
        message_recovery: String::new(),
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

/// One hero with a weapon whose `weapon_animation` is 7, versus a lone foe
/// placed at RM2000 (100, 100), for the attack-animation queue tests.
fn build_weapon_anim(weapon_animation: u32) -> Battle {
    use crate::battle::model::testkit;
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
        &[testkit::slots(&ron)],
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

fn damage_release_state(id: u32) -> amnezia_data::StateDef {
    amnezia_data::StateDef {
        color: 6,
        message_actor: String::new(),
        message_enemy: String::new(),
        message_already: String::new(),
        message_affected: String::new(),
        message_recovery: String::new(),
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
        color: 6,
        message_actor: String::new(),
        message_enemy: String::new(),
        message_already: String::new(),
        message_affected: String::new(),
        message_recovery: String::new(),
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

fn medicine(
    id: u32,
    recover_hp: u32,
    recover_sp: u32,
    cure_states: Vec<u32>,
) -> amnezia_data::ItemDef {
    amnezia_data::ItemDef {
        prevent_critical: false,
        raise_evasion: false,
        half_sp_cost: false,
        actor_set: Vec::new(),
        state_chance: 0,
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
        ko_only: false,
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

/// An always-eligible enemy AI entry of the given `basic` code.
fn enemy_action_def(basic: u32) -> amnezia_data::EnemyActionDef {
    amnezia_data::EnemyActionDef {
        kind: 0,
        basic,
        skill_id: 0,
        enemy_id: 0,
        condition_type: 0,
        condition_min: 0,
        condition_max: 0,
        priority: 1,
        ..Default::default()
    }
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

/// A can't-act (restriction 1) state — asleep or paralyzed.
fn sleep_state(id: u32) -> amnezia_data::StateDef {
    amnezia_data::StateDef {
        color: 6,
        message_actor: String::new(),
        message_enemy: String::new(),
        message_already: String::new(),
        message_affected: String::new(),
        message_recovery: String::new(),
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
        name: "Alvás".into(),
        restriction: 1,
        priority: 0,
        hold_turn: 99,
        auto_release_prob: 0,
        release_by_damage: 0,
        hp_change_type: 0,
        hp_change_max: 0,
        hp_change_val: 0,
        hp_change_map_steps: 0,
        hp_change_map_val: 0,
    }
}

mod attacks;
mod conditions;
mod enemies;
mod enemy_ai;
mod enemy_targets;
mod item_consumption;
mod presentation;
mod recovery;
mod skill_accuracy;
mod skill_pools;
mod skill_states;
mod skill_stats;
mod skills;
mod weapon_accuracy;

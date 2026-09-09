use super::*;
use crate::assets::{asset_root, load_ron};
use crate::battle::model::Fighter;
use crate::equipment::EquipmentEffects;
use amnezia_data::{ActorDef, ItemDef, MonsterDef, StateDef};

fn original_items() -> Vec<ItemDef> {
    load_ron(&format!("{}/items.ron", asset_root()))
}

fn original_battle(slots: [u32; 5]) -> Battle {
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let mut battle = build_party2();
    battle.states = load_ron::<Vec<StateDef>>(&format!("{}/states.ron", asset_root()));
    battle.members[0] = Fighter::build(
        &actors[2],
        slots,
        &original_items(),
        &crate::vitals::Vitals::default(),
        &crate::progression::Progression::default(),
    );
    battle
}

#[test]
fn original_equipment_flags_apply_once_and_only_to_defensive_gear() {
    let mut items = original_items();
    for (flags, expected) in [
        (
            items
                .iter()
                .filter(|i| i.prevent_critical)
                .map(|i| i.id)
                .collect::<Vec<_>>(),
            vec![143, 148, 153, 158, 163],
        ),
        (
            items
                .iter()
                .filter(|i| i.raise_evasion)
                .map(|i| i.id)
                .collect(),
            vec![144, 149, 154, 159, 164],
        ),
        (
            items
                .iter()
                .filter(|i| i.half_sp_cost)
                .map(|i| i.id)
                .collect(),
            vec![147, 152, 157, 162, 167],
        ),
    ] {
        assert_eq!(flags, expected);
    }
    let half = EquipmentEffects::from_slots([0, 152, 157, 167, 0], &items);
    for (cost, expected) in [(0, 0), (1, 1), (8, 4), (9, 5), (u32::MAX, 2_147_483_648)] {
        assert_eq!(half.skill_cost(cost), expected);
    }
    let evade = EquipmentEffects::from_slots([0, 149, 154, 164, 0], &items);
    assert_eq!(evade.physical_hit(90, true), 65);
    assert_eq!(evade.physical_hit(100, false), 100);
    let guard = EquipmentEffects::from_slots([0, 148, 153, 163, 0], &items);
    assert_eq!(guard.critical_chance(100), 0);
    items[0].prevent_critical = true;
    items[0].raise_evasion = true;
    items[0].half_sp_cost = true;
    assert_eq!(
        EquipmentEffects::from_slots([1, 0, 0, 0, 0], &items),
        EquipmentEffects::default()
    );
}

#[test]
fn original_moonstone_changes_battle_cost_display_affordability_and_single_payment() {
    let mut battle = original_battle([0, 152, 157, 167, 0]);
    let mut skill = damage_skill(1, 1, vec![], vec![]);
    skill.sp_cost = 9;
    skill.scope = 1;
    skill.animation_id = 4;
    battle.skills = vec![skill.clone()];
    let data = crate::gamedata::GameData {
        actors: vec![],
        items: vec![],
        skills: battle.skills.clone(),
    };
    let rows =
        crate::battle::input::skill_choices(&data, &[1], battle.members[0].equipment_effects);
    assert_eq!(rows[0].1, 5);
    battle.members[0].sp = 4;
    assert!(!battle.skill_usable_by(Source::Party(0), &skill));
    battle.members[0].sp = 5;
    assert!(battle.skill_usable_by(Source::Party(0), &skill));
    assert_eq!(battle.skill_cost(Source::Enemy(0), &skill), 9);
    battle.cast_skill(0, 1, 0);
    assert_eq!(battle.members[0].sp, 0);
    while !battle.steps.is_empty() {
        battle.resolve_next();
    }
    assert_eq!(battle.members[0].sp, 0);
}

#[test]
fn opal_affects_normal_confused_and_physical_skill_accuracy_but_not_magic_or_sleepers() {
    let mut battle = original_battle([0, 149, 154, 164, 0]);
    battle.members[0].stats.agility = 10;
    battle.enemies[0].stats.agility = 10;
    let mut skill = damage_skill(1, 1, vec![], vec![]);
    skill.hit = 90;
    skill.failure_message = 3;
    assert_eq!(
        battle.skill_hit_chance(Source::Enemy(0), Source::Party(0), &skill),
        65
    );
    skill.failure_message = 0;
    assert_eq!(
        battle.skill_hit_chance(Source::Enemy(0), Source::Party(0), &skill),
        90
    );
    let seed = (1..1000)
        .find(|&seed| {
            let mut rng = seed;
            (65..90).contains(&(rng_next(&mut rng) % 100))
        })
        .unwrap();
    battle.rng = seed;
    assert_eq!(battle.enemy_strike_member(0, 0), None);
    battle.members[1].stats.agility = 10;
    battle.members[1].weapon_hit = 90;
    battle.members[1].attack_animation = 0;
    battle.rng = seed;
    let hp = battle.members[0].hp;
    battle.confused_attack(Source::Party(1), Source::Party(0));
    assert_eq!(battle.members[0].hp, hp);
    battle.members[0].states = vec![(7, 0)];
    skill.failure_message = 3;
    assert_eq!(
        battle.skill_hit_chance(Source::Enemy(0), Source::Party(0), &skill),
        100
    );
    battle.rng = seed;
    assert!(battle.enemy_strike_member(0, 0).is_some());
}

#[test]
fn jasper_blocks_enemy_criticals_and_original_enemies_do_not_have_them_enabled() {
    let monsters = load_ron::<Vec<MonsterDef>>(&format!("{}/monsters.ron", asset_root()));
    assert_eq!(monsters.len(), 44);
    assert!(monsters.iter().all(|m| !m.critical_hit));
    for (slots, range) in [([0; 5], 120..=180), ([0, 148, 153, 163, 0], 40..=60)] {
        let mut battle = original_battle(slots);
        battle.enemies[0].base_critical_denominator = Some(1);
        battle.enemies[0].stats.attack = 100;
        battle.members[0].stats.defense = 0;
        battle.members[0].hp = 9999;
        battle.members[0].max_hp = 9999;
        battle.rng = 0;
        assert!(range.contains(&battle.enemy_strike_member(0, 0).unwrap()));
    }
}

#[test]
fn ice_claw_respects_ranks_and_applies_after_critical_or_confused_impact() {
    let mut battle = original_battle([13, 0, 0, 0, 0]);
    assert_eq!(battle.members[0].weapon_states, [(8, 50)]);
    battle.enemies[0].state_ranks = vec![0; 8];
    let expected = battle.battler_state_probability(Source::Enemy(0), 8) / 2;
    for seed in 1..100 {
        battle.enemies[0].states.clear();
        battle.rng = seed;
        let mut rng = seed;
        let roll = rng_next(&mut rng) % 100;
        battle.weapon_states(Source::Party(0), Source::Enemy(0));
        assert_eq!(
            logic::has_state(&battle.enemies[0].states, 8),
            roll < u64::from(expected)
        );
    }
    battle.enemies[0].states.clear();
    battle.enemies[0].state_ranks = vec![4; 8];
    battle.rng = 0;
    battle.weapon_states(Source::Party(0), Source::Enemy(0));
    assert!(battle.enemies[0].states.is_empty());
    battle.enemies[0].state_ranks = vec![0; 8];
    battle.resolve_strike_impact(0, 0, Strike::Hit { dmg: 0, crit: true });
    assert!(battle.enemies[0].states.is_empty());
    battle.rng = 0;
    battle.resolve_next();
    assert!(logic::has_state(&battle.enemies[0].states, 8));
    battle.members[1].state_ranks = vec![0; 8];
    battle.rng = 0;
    battle.land_ally_strike(Source::Party(0), Source::Party(1), Some(0));
    assert!(logic::has_state(&battle.members[1].states, 8));
    battle.enemies[0].hp = 1;
    battle.enemies[0].states.clear();
    battle.land_strike(0, 0, 1);
    assert!(!logic::has_state(&battle.enemies[0].states, 8));
}

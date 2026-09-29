use super::*;
use crate::assets::{asset_root, load_ron};
use crate::battle::model::Fighter;
use amnezia_data::ActorDef;

#[test]
fn original_actors_retain_their_critical_flags_and_denominators() {
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let expected = [
        Some(30),
        None,
        None,
        Some(20),
        Some(20),
        Some(30),
        None,
        None,
        Some(50),
        None,
    ];
    for (actor, denominator) in actors.iter().zip(expected) {
        let fighter = Fighter::build(
            actor,
            [0; 5],
            &[],
            &crate::vitals::Vitals::default(),
            &crate::progression::Progression::default(),
        );
        assert_eq!(
            fighter.base_critical_denominator, denominator,
            "{}",
            actor.name
        );
        assert_eq!(fighter.weapon_crit, 0);
    }
}

#[test]
fn unarmed_ron_can_critical_and_equipping_a_weapon_keeps_his_base_chance() {
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let mut weapon = crate::battle::model::testkit::item(1, 0, 0, 100, 5, 0);
    weapon.item_type = 1;
    let items = [weapon];
    let mut battle = build_1v2();
    for (slots, gear, expected) in [([0; 5], &[][..], 3), ([1, 0, 0, 0, 0], &items[..], 8)] {
        battle.members[0] = Fighter::build(
            &actors[0],
            slots,
            gear,
            &crate::vitals::Vitals::default(),
            &crate::progression::Progression::default(),
        );
        let member = &battle.members[0];
        assert_eq!(
            logic::critical_chance(member.base_critical_denominator, member.weapon_crit),
            expected
        );
        let mut critical = false;
        let mut ordinary = false;
        for seed in 1..=300 {
            battle.rng = seed;
            match battle.plan_strike(0, 0) {
                Strike::Hit { crit: true, .. } => critical = true,
                Strike::Hit { crit: false, .. } => ordinary = true,
                _ => {}
            }
        }
        assert!(critical && ordinary);
    }
}

#[test]
fn actor_base_critical_never_multiplies_an_ordinary_skill() {
    let mut battle = build_1v2();
    battle.members[0].base_critical_denominator = Some(1);
    battle.members[0].weapon_crit = 100;
    let mut skill = damage_skill(1, 20, vec![], vec![]);
    skill.magical_rate = 0;
    skill.variance = 0;
    battle.enemies[0].hp = 100;
    battle.enemies[0].max_hp = 100;
    battle.skill_hit_enemy(0, 0, &skill);
    assert_eq!(battle.enemies[0].hp, 80);
}

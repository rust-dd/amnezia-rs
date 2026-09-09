use super::*;
use crate::battle::model::testkit;
use crate::progression::Progression;
use crate::vitals::Vitals;

#[test]
fn an_unarmed_fighter_starts_with_the_original_ninety_percent_hit_rate() {
    let battle = build_1v2();
    assert_eq!(battle.members[0].weapon_hit, 90);
}

#[test]
fn the_original_dragon_claw_keeps_its_explicit_zero_hit_rate() {
    let items = crate::assets::load_ron::<Vec<amnezia_data::ItemDef>>(&format!(
        "{}/items.ron",
        crate::assets::asset_root()
    ));
    let weapon = items.iter().find(|item| item.id == 202).unwrap();
    assert_eq!(weapon.name, "Sárkánykarom");
    assert_eq!(weapon.hit, 0);
    let mut actor = testkit::actor(1, 2, 63, 37);
    actor.weapon = weapon.id;
    let mut battle = Battle::build(
        &testkit::troop(&[(1, 100, 100)]),
        &[testkit::monster(1, 100, 0, 0)],
        &[&actor],
        &[testkit::slots(&actor)],
        &items,
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        1,
    );
    assert_eq!(battle.members[0].weapon_hit, 0);
    battle.enemies[0].stats.agility = battle.members[0].stats.agility;
    for seed in 1..=32 {
        battle.rng = seed;
        assert!(matches!(battle.strike_enemy(0, 0), Strike::Miss));
    }
}

use super::*;
use crate::battle::model::Fighter;
use amnezia_data::{ActorDef, AttributeDef, ItemDef};

fn original<T: serde::de::DeserializeOwned>(name: &str) -> Vec<T> {
    crate::assets::load_ron(&format!("{}/{name}.ron", crate::assets::asset_root()))
}

fn original_member(id: u32, slots: [u32; 5]) -> Fighter {
    let actors = original::<ActorDef>("actors");
    Fighter::build(
        actors.iter().find(|actor| actor.id == id).unwrap(),
        slots,
        &original::<ItemDef>("items"),
        &crate::vitals::Vitals::default(),
        &crate::progression::Progression::default(),
    )
}

#[test]
fn original_tray_and_daren_use_their_own_attribute_ranks() {
    let actors = original::<ActorDef>("actors");
    assert_eq!(
        actors[4].attribute_ranks,
        [2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1]
    );
    assert_eq!(actors[9].attribute_ranks, [4; 17]);
    assert_eq!(
        actors
            .iter()
            .filter(|actor| !actor.attribute_ranks.is_empty())
            .count(),
        2
    );
    let mut battle = build_1v2();
    battle.attributes = original::<AttributeDef>("attributes");
    for (actor, sword, magic) in [(1, 100, 100), (5, 100, 150), (10, 50, 0)] {
        battle.members[0] = original_member(actor, [0; 5]);
        assert_eq!(
            battle.battler_attribute_damage(100, Source::Party(0), &[1]),
            sword
        );
        for attribute in 5..=12 {
            assert_eq!(
                battle.battler_attribute_damage(100, Source::Party(0), &[attribute]),
                magic
            );
        }
    }
}

#[test]
fn defensive_gear_improves_a_rank_once_and_weapon_attributes_never_defend() {
    let mut weapon = crate::battle::model::testkit::item(1, 0, 0, 100, 0, 5);
    weapon.item_type = 1;
    let mut armor = weapon.clone();
    armor.id = 2;
    armor.item_type = 3;
    let mut shield = armor.clone();
    shield.id = 3;
    shield.item_type = 2;
    let items = [weapon, armor, shield];
    assert!(logic::equipment_resist_slots([1, 0, 0, 0, 0], &items).is_empty());
    let guards = logic::equipment_resist_slots([1, 3, 2, 0, 0], &items);
    assert_eq!(guards, [5]);
    let mut battle = build_1v2();
    battle.attributes = original::<AttributeDef>("attributes");
    for (actor, expected) in [(1, 50), (5, 100), (10, 0)] {
        battle.members[0] = original_member(actor, [0; 5]);
        battle.members[0].resist_attributes = guards.clone();
        assert_eq!(
            battle.battler_attribute_damage(100, Source::Party(0), &[5]),
            expected
        );
    }
}

#[test]
fn skills_use_all_attributes_on_both_sides_including_healing() {
    let mut battle = build_1v2();
    battle.attributes = original::<AttributeDef>("attributes");
    let mut skill = damage_skill(1, 101, vec![1, 2, 5, 6], vec![]);
    skill.physical_rate = 0;
    skill.magical_rate = 0;
    skill.variance = 0;
    battle.enemies[0].attribute_ranks = vec![1, 4, 2, 2, 1, 4];
    assert_eq!(
        battle.skill_magnitude(Source::Party(0), Source::Enemy(0), &skill),
        189
    );
    battle.members[0] = original_member(5, [0; 5]);
    assert_eq!(
        battle.skill_magnitude(Source::Enemy(0), Source::Party(0), &skill),
        151
    );
    skill.scope = 3;
    battle.members[0].hp = 1;
    battle.skill_heal_battler(Source::Party(0), Source::Party(0), &skill);
    assert_eq!(battle.members[0].hp, 152);
    battle.members[0] = original_member(10, [0; 5]);
    battle.members[0].hp = 1;
    battle.skill_heal_battler(Source::Party(0), Source::Party(0), &skill);
    assert_eq!(battle.members[0].hp, 1);
}

#[test]
fn weapon_swing_uses_every_attribute_without_adding_it_to_a_skill() {
    let mut battle = build_1v2();
    battle.attributes = original::<AttributeDef>("attributes");
    battle.members[0].weapon_hit = 100;
    battle.members[0].weapon_crit = 0;
    battle.members[0].stats.attack = 200;
    battle.enemies[0].stats.defense = 0;
    battle.enemies[0].attribute_ranks = vec![2, 2, 2, 2, 4, 0];
    battle.members[0].weapon_attributes = vec![5];
    assert!(matches!(
        battle.plan_strike(0, 0),
        Strike::Hit { dmg: 0, .. }
    ));
    battle.members[0].weapon_attributes = vec![5, 6];
    assert!(matches!(
        battle.plan_strike(0, 0),
        Strike::Hit { dmg: 160..=240, .. }
    ));
    let mut skill = damage_skill(1, 100, vec![5], vec![]);
    skill.variance = 0;
    assert_eq!(
        battle.skill_magnitude(Source::Party(0), Source::Enemy(0), &skill),
        0
    );
}

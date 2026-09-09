use super::*;
use crate::assets::{asset_root, load_ron};
use crate::battle::model::Fighter;
use amnezia_data::{ActorDef, AttributeDef, ItemDef};

#[test]
fn original_sword_skills_need_a_weapon_even_when_sp_is_sufficient() {
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let items = load_ron::<Vec<ItemDef>>(&format!("{}/items.ron", asset_root()));
    let mut battle = build_1v2();
    battle.skills = load_ron(&format!("{}/skills.ron", asset_root()));
    battle.attributes = load_ron(&format!("{}/attributes.ron", asset_root()));
    for weapon in [0, 1] {
        battle.members[0] = Fighter::build(
            &actors[0],
            [weapon, 0, 0, 0, 0],
            &items,
            &crate::vitals::Vitals::default(),
            &crate::progression::Progression::default(),
        );
        battle.members[0].sp = 9999;
        for id in 1..=6 {
            let skill = battle.skills.iter().find(|skill| skill.id == id).unwrap();
            assert_eq!(battle.skill_usable_by(Source::Party(0), skill), weapon == 1);
            assert!(battle.skill_usable_by(Source::Enemy(0), skill));
        }
    }
    battle.members[0].weapon_attributes.clear();
    let sp = battle.members[0].sp;
    let hp = battle.enemies[0].hp;
    battle.apply(Action {
        source: Source::Party(0),
        kind: Command::Skill {
            skill_id: 1,
            target: 0,
        },
        agility: 1,
    });
    assert_eq!(battle.members[0].sp, sp);
    assert_eq!(battle.enemies[0].hp, hp);
    assert!(battle.pending_anims.is_empty());
    assert!(battle.steps.is_empty());
}

#[test]
fn every_physical_attribute_is_required_and_magic_or_damage_weights_are_not_requirements() {
    let attributes = load_ron::<Vec<AttributeDef>>(&format!("{}/attributes.ron", asset_root()));
    let mut skill = damage_skill(1, 1, vec![1, 2, 5], vec![]);
    assert!(!logic::weapon_allows_skill(&skill, &[1, 5], &attributes));
    assert!(logic::weapon_allows_skill(&skill, &[1, 2], &attributes));
    skill.attributes = vec![5, 6];
    skill.physical_rate = 10;
    assert!(logic::weapon_allows_skill(&skill, &[], &attributes));
}

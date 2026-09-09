use super::*;
use crate::assets::{asset_root, load_ron};
use amnezia_data::{ActorDef, ItemDef, MonsterDef, TroopDef, TroopMemberDef};

fn original_battle(ids: &[u32]) -> Battle {
    let monsters = load_ron::<Vec<MonsterDef>>(&format!("{}/monsters.ron", asset_root()));
    let actors = load_ron::<Vec<ActorDef>>(&format!("{}/actors.ron", asset_root()));
    let items = load_ron::<Vec<ItemDef>>(&format!("{}/items.ron", asset_root()));
    let troop = TroopDef {
        id: 1,
        name: String::new(),
        pages: vec![],
        members: ids
            .iter()
            .map(|&enemy_id| TroopMemberDef {
                enemy_id,
                x: 100,
                y: 100,
            })
            .collect(),
    };
    Battle::build(
        &troop,
        &monsters,
        &[&actors[0]],
        &[],
        &items,
        &[],
        &[],
        &[],
        &crate::vitals::Vitals::default(),
        &crate::progression::Progression::default(),
        String::new(),
        1,
    )
}

#[test]
fn all_four_original_drops_are_imported_and_guaranteed_drops_survive_a_high_roll() {
    let monsters = load_ron::<Vec<MonsterDef>>(&format!("{}/monsters.ron", asset_root()));
    let drops = monsters
        .iter()
        .filter(|m| m.drop_id != 0)
        .map(|m| (m.id, m.drop_id, m.drop_prob))
        .collect::<Vec<_>>();
    assert_eq!(
        drops,
        [(9, 33, 100), (10, 43, 50), (16, 3, 100), (32, 139, 100)]
    );
    let mut battle = original_battle(&[9, 16, 32]);
    for enemy in &mut battle.enemies {
        enemy.hp = 0;
    }
    battle.rng = (1..10000)
        .find(|&seed| {
            let mut rng = seed;
            rng_next(&mut rng) % 100 == 99
        })
        .unwrap();
    battle.finish(BattleOutcome::Victory);
    assert_eq!(battle.reward_items, [33, 3, 139]);
    let rng = battle.rng;
    battle.finish(BattleOutcome::Victory);
    assert_eq!(battle.rng, rng);
    assert_eq!(battle.reward_items, [33, 3, 139]);
}

#[test]
fn bridge_guard_fifty_percent_drop_uses_an_exclusive_upper_bound() {
    for roll in [49, 50] {
        let mut battle = original_battle(&[10]);
        battle.enemies[0].hp = 0;
        battle.rng = (1..10000)
            .find(|&seed| {
                let mut rng = seed;
                rng_next(&mut rng) % 100 == roll
            })
            .unwrap();
        battle.finish(BattleOutcome::Victory);
        assert_eq!(battle.reward_items.len(), usize::from(roll < 50));
    }
}

#[test]
fn living_fled_invalid_and_zero_chance_enemies_never_drop_items() {
    let mut battle = original_battle(&[32, 32, 32, 32]);
    battle.enemies[1].hp = 0;
    battle.enemies[1].fled = true;
    battle.enemies[2].hp = 0;
    battle.enemies[2].drop_id = 9999;
    battle.enemies[3].hp = 0;
    battle.enemies[3].drop_prob = 0;
    battle.rng = 0;
    battle.finish(BattleOutcome::Victory);
    assert!(battle.reward_items.is_empty());
    for outcome in [
        BattleOutcome::Defeat,
        BattleOutcome::Escape,
        BattleOutcome::Abort,
    ] {
        let mut battle = original_battle(&[32]);
        battle.enemies[0].hp = 0;
        let rng = battle.rng;
        battle.finish(outcome);
        assert!(battle.reward_items.is_empty());
        assert_eq!(battle.rng, rng);
    }
}

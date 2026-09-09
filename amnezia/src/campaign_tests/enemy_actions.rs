use super::*;
use amnezia_data::MonsterDef;

#[test]
fn original_enemy_ai_preserves_all_conditions_and_the_final_battle_switches() {
    let enemies = load_ron::<Vec<MonsterDef>>(&format!("{}/monsters.ron", asset_root()));
    let mut conditions = BTreeMap::new();
    for action in enemies.iter().flat_map(|e| &e.actions) {
        *conditions.entry(action.condition_type).or_insert(0) += 1;
    }
    assert_eq!(
        conditions,
        BTreeMap::from([(0, 95), (1, 2), (2, 2), (4, 4), (5, 1)])
    );
    for id in [25, 43] {
        let reaper = enemies.iter().find(|e| e.id == id).unwrap();
        assert_eq!(reaper.actions.len(), 1);
        let action = &reaper.actions[0];
        assert_eq!(
            (action.skill_id, action.condition_type, action.switch_id),
            (58, 1, 545)
        );
        assert!(!action.switch_on && action.switch_off);
        assert_eq!(action.switch_off_id, 545);
    }
    let dragon = enemies.iter().find(|e| e.id == 40).unwrap();
    assert!(
        dragon
            .actions
            .iter()
            .all(|a| (a.condition_type, a.condition_min, a.condition_max) == (2, 2, 1))
    );
    let drain = enemies
        .iter()
        .find(|e| e.id == 41)
        .unwrap()
        .actions
        .last()
        .unwrap();
    assert_eq!(
        (
            drain.skill_id,
            drain.condition_type,
            drain.condition_min,
            drain.condition_max,
            drain.priority
        ),
        (65, 5, 0, 10, 100)
    );
}

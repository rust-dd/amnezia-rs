use super::*;

#[test]
fn all_condition_codes_read_their_original_live_quantity() {
    let context = EnemyAiContext {
        turn: 3,
        enemies: 2,
        hp: 25,
        sp: 10,
        level: 17,
        fatigue: 42,
    };
    for (code, value) in [(3, 2), (4, 25), (5, 10), (6, 17), (7, 42)] {
        let mut action = EnemyActionDef {
            condition_type: code,
            condition_min: value,
            condition_max: value,
            ..Default::default()
        };
        assert!(condition(&action, &context, |_| false));
        action.condition_max = value - 1;
        assert!(!condition(&action, &context, |_| true));
        action.condition_max = value + 1;
        action.condition_min = value + 1;
        assert!(!condition(&action, &context, |_| true));
    }
    let action = EnemyActionDef {
        condition_type: 1,
        switch_id: 545,
        ..Default::default()
    };
    assert!(condition(&action, &context, |id| id == 545));
    assert!(!condition(&action, &context, |_| false));
    let action = EnemyActionDef {
        condition_type: 2,
        condition_min: 2,
        condition_max: 1,
        ..Default::default()
    };
    assert!(condition(&action, &context, |_| false));
    assert!(!condition(
        &action,
        &EnemyAiContext { turn: 2, ..context },
        |_| false
    ));
    assert!(condition(&EnemyActionDef::default(), &context, |_| false));
}

#[test]
fn rating_cutoff_keeps_the_original_ten_point_weight_window() {
    let actions = [50, 49, 48, 40, 0]
        .into_iter()
        .enumerate()
        .map(|(id, priority)| EnemyActionDef {
            skill_id: id as u32,
            priority,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let mut counts = [0; 5];
    for roll in 0..27 {
        let chosen = choose_enemy_action(
            &actions,
            &EnemyAiContext::default(),
            |_| true,
            |_| true,
            |_| false,
            roll,
        )
        .unwrap();
        counts[chosen.skill_id as usize] += 1;
    }
    assert_eq!(counts, [10, 9, 8, 0, 0]);
}

#[test]
fn unusable_actions_are_removed_before_weighting_but_targetless_skills_after() {
    let actions = [50, 100].map(|priority| EnemyActionDef {
        priority,
        ..Default::default()
    });
    let context = EnemyAiContext::default();
    assert_eq!(
        choose_enemy_action(
            &actions,
            &context,
            |a| a.priority < 100,
            |_| true,
            |_| false,
            0
        )
        .unwrap()
        .priority,
        50
    );
    assert!(
        choose_enemy_action(
            &actions,
            &context,
            |_| true,
            |a| a.priority < 100,
            |_| false,
            0
        )
        .is_none()
    );
    assert!(choose_enemy_action(&[], &context, |_| true, |_| true, |_| false, 0).is_none());
    assert!(
        choose_enemy_action(
            &[EnemyActionDef {
                priority: 0,
                ..Default::default()
            }],
            &context,
            |_| true,
            |_| true,
            |_| false,
            0
        )
        .is_none()
    );
}

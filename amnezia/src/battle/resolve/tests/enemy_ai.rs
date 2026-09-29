use super::*;

fn original_enemy(battle: &mut Battle, id: u32) {
    let monsters = crate::assets::load_ron::<Vec<amnezia_data::MonsterDef>>(&format!(
        "{}/monsters.ron",
        crate::assets::asset_root()
    ));
    let monster = monsters.iter().find(|monster| monster.id == id).unwrap();
    battle.enemies[0].actions.clone_from(&monster.actions);
    battle.enemies[0].max_sp = monster.max_sp as i32;
    battle.enemies[0].sp = monster.max_sp as i32;
    battle.skills = crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()));
}

#[test]
fn original_reapers_wait_for_switch_545_and_only_clear_it_after_the_rocket() {
    for id in [25, 43] {
        let mut battle = build_1v2();
        original_enemy(&mut battle, id);
        assert!(matches!(
            battle.enemy_action(0, &[true]).unwrap().kind,
            Command::Nothing
        ));
        battle.ai_switches.insert(545);
        let action = battle.enemy_action(0, &[true]).unwrap();
        assert!(matches!(action.kind, Command::Skill { skill_id: 58, .. }));
        assert!(battle.ai_switches.contains(&545));
        battle.queue = vec![action];
        battle.tick_action();
        assert!(battle.ai_switches.contains(&545));
        assert!(battle.pending_switches.is_empty());
        assert!(!!battle.action_in_progress());
        while battle.resolve_next() {}
        assert!(!battle.ai_switches.contains(&545));
        assert_eq!(battle.pending_switches, [(545, false)]);
    }
}

#[test]
fn original_fire_dragon_turn_skills_only_appear_on_odd_turns() {
    let mut battle = build_1v2();
    original_enemy(&mut battle, 40);
    for turn in 0..=6 {
        battle.events.turn = turn;
        let mut saw_turn_skill = false;
        for _ in 0..200 {
            let action = battle.enemy_action(0, &[true]).unwrap();
            if matches!(
                action.kind,
                Command::Skill {
                    skill_id: 60 | 61,
                    ..
                }
            ) {
                saw_turn_skill = true;
            }
        }
        assert_eq!(saw_turn_skill, turn % 2 == 1);
    }
}

#[test]
fn original_demonlord_sp_condition_and_cost_share_the_live_pool() {
    let mut battle = build_1v2();
    original_enemy(&mut battle, 41);
    for sp in [0, 50, 109, 110, 999] {
        battle.enemies[0].sp = sp;
        for _ in 0..100 {
            let action = battle.enemy_action(0, &[true]).unwrap();
            assert_eq!(
                matches!(action.kind, Command::Skill { skill_id: 65, .. }),
                sp <= 109
            );
        }
    }
}

#[test]
fn an_unaffordable_only_skill_does_not_turn_into_a_free_attack() {
    let mut battle = build_1v2();
    battle.skills = vec![damage_skill(1, 10, vec![], vec![])];
    battle.enemies[0].actions = vec![amnezia_data::EnemyActionDef {
        kind: 1,
        skill_id: 1,
        ..Default::default()
    }];
    battle.enemies[0].sp = 2;
    assert!(matches!(
        battle.enemy_action(0, &[true]).unwrap().kind,
        Command::Nothing
    ));
    battle.enemies[0].sp = 3;
    assert!(matches!(
        battle.enemy_action(0, &[true]).unwrap().kind,
        Command::Skill { .. }
    ));
}

#[test]
fn ally_selection_includes_full_hp_and_revivable_dead_foes_but_not_fled_ones() {
    let mut battle = build_1v2();
    let mut skill = heal_skill(1, 10);
    assert_eq!(battle.enemy_ally_skill_targets(&skill), [0, 1]);
    battle.enemies[1].hp = 0;
    assert_eq!(battle.enemy_ally_skill_targets(&skill), [0]);
    skill.affected_states = vec![1];
    assert_eq!(battle.enemy_ally_skill_targets(&skill), [0, 1]);
    battle.enemies[1].fled = true;
    assert_eq!(battle.enemy_ally_skill_targets(&skill), [0]);
    skill.affect_hp = false;
    skill.affected_states = vec![2];
    assert!(battle.enemy_ally_skill_targets(&skill).is_empty());
    battle.enemies[0].states = vec![(2, 0)];
    assert_eq!(battle.enemy_ally_skill_targets(&skill), [0]);
}

#[test]
fn live_ai_level_fatigue_hp_sp_and_enemy_count_are_not_placeholders() {
    let mut battle = build_party2();
    battle.members[0].level = 10;
    battle.members[1].level = 21;
    for member in &mut battle.members {
        member.hp = member.max_hp;
        member.sp = 0;
    }
    battle.enemies[0].hp = 15;
    battle.enemies[0].sp = 0;
    let context = battle.enemy_ai_context(0);
    assert_eq!(
        (
            context.level,
            context.fatigue,
            context.hp,
            context.sp,
            context.enemies
        ),
        (15, 33, 50, 0, 1)
    );
    for member in &mut battle.members {
        member.hp = 0;
    }
    assert_eq!(battle.enemy_ai_context(0).fatigue, 100);
}

#[test]
fn skipped_actions_do_not_run_post_action_switches_and_on_precedes_off() {
    for skip in [false, true] {
        let mut battle = build_1v2();
        battle.enemies[0].actions = vec![amnezia_data::EnemyActionDef {
            basic: 2,
            switch_on: true,
            switch_on_id: 7,
            switch_off: true,
            switch_off_id: 7,
            ..Default::default()
        }];
        let action = battle.enemy_action(0, &[true]).unwrap();
        battle.queue = vec![action];
        if skip {
            battle.enemies[0].hp = 0;
        }
        while battle.resolve_next() {}
        assert!(!battle.ai_switches.contains(&7));
        if skip {
            assert!(battle.pending_switches.is_empty());
        } else {
            assert_eq!(battle.pending_switches, [(7, true), (7, false)]);
        }
    }
}

#[test]
fn losing_sp_after_selection_cancels_the_skill_and_its_switch_effect() {
    let mut battle = build_1v2();
    battle.skills = vec![damage_skill(1, 10, vec![], vec![])];
    battle.enemies[0].actions = vec![amnezia_data::EnemyActionDef {
        kind: 1,
        skill_id: 1,
        switch_on: true,
        switch_on_id: 7,
        ..Default::default()
    }];
    let action = battle.enemy_action(0, &[true]).unwrap();
    battle.queue = vec![action];
    battle.enemies[0].sp = 0;
    while battle.resolve_next() {}
    assert!(battle.pending_anims.is_empty());
    assert!(battle.pending_switches.is_empty());
    assert!(!battle.ai_switches.contains(&7));
}

#[test]
fn completed_actions_flush_switches_to_map_and_troop_events_once() {
    use bevy::prelude::*;
    let mut battle = build_1v2();
    battle.pending_switches = vec![(545, true), (545, false), (617, true)];
    let mut app = App::new();
    app.insert_resource(battle)
        .init_resource::<crate::state::Switches>()
        .add_systems(Update, crate::battle::events::sync_switches);
    app.update();
    let switches = app.world().resource::<crate::state::Switches>();
    assert!(!switches.get(545));
    assert!(switches.get(617));
    assert!(app.world().resource::<Battle>().pending_switches.is_empty());
}

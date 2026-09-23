use super::*;

#[test]
fn a_threshold_victory_levels_up_before_the_outcome_and_pays_exactly_once() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    crate::dialogue::testing::register_playback(&mut app);
    app.init_resource::<crate::battle::BattleFlow>();
    let ron = ActorDef {
        character_name: String::new(),
        character_index: 0,
        rename_skill: false,
        skill_name: String::new(),
        critical_hit: false,
        critical_hit_chance: 30,
        state_ranks: Vec::new(),
        attribute_ranks: Vec::new(),
        id: 1,
        name: "Ron".into(),
        title: String::new(),
        level: 1,
        max_level: 50,
        hp: 40,
        sp: 10,
        curves: Default::default(),
        learnings: vec![amnezia_data::Learning {
            level: 2,
            skill_id: 1,
        }],
        exp_base: 30,
        exp_inflation: 30,
        exp_correction: 0,
        weapon: 0,
        shield: 0,
        armor: 0,
        helmet: 0,
        accessory: 0,
        two_weapons: false,
        fix_equipment: false,
        unarmed_animation: 0,
        face_name: String::new(),
        face_index: 0,
    };
    app.insert_resource(GameData {
        actors: vec![ron.clone()],
        items: vec![],
        skills: vec![SkillDef {
            using_message1: String::new(),
            using_message2: String::new(),
            affect_stats: [false; 4],
            ignore_defense: false,
            id: 1,
            name: "Tűzcsapás".into(),
            description: String::new(),
            sp_cost: 0,
            power: 10,
            hit: 100,
            skill_type: 0,
            failure_message: 0,
            scope: 0,
            animation_id: 0,
            physical_rate: 0,
            magical_rate: 0,
            variance: 0,
            affect_hp: true,
            affect_sp: false,
            absorb: false,
            attributes: vec![],
            affected_states: vec![],
        }],
    });
    app.init_resource::<Inventory>();
    app.init_resource::<Vitals>();
    app.init_resource::<Progression>();
    app.init_resource::<BattleResult>();
    app.init_resource::<BattleActive>();
    app.init_resource::<MapBgm>();
    app.add_message::<AudioRequest>();
    app.init_resource::<ButtonInput<KeyCode>>();
    let monsters = vec![MonsterDef {
        battler_hue: 0,
        drop_id: 0,
        drop_prob: 100,
        critical_hit: false,
        critical_hit_chance: 30,
        id: 1,
        name: "Rabló".into(),
        battler: String::new(),
        max_hp: 30,
        max_sp: 0,
        attack: 20,
        defense: 8,
        spirit: 0,
        agility: 8,
        exp: 30,
        gold: 30,
        attribute_ranks: vec![],
        state_ranks: vec![],
        actions: vec![],
    }];
    let troop = TroopDef {
        id: 1,
        name: "T".into(),
        pages: Vec::new(),
        members: vec![TroopMemberDef {
            enemy_id: 1,
            x: 100,
            y: 100,
        }],
    };
    let battle = Battle::build(
        &troop,
        &monsters,
        &[&ron],
        &[[0, 0, 0, 0, 0]],
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        1,
    );
    app.insert_resource(battle);
    app.world_mut().resource_mut::<Battle>().text.level_up = ". szintre lépett".into();
    app.world_mut().resource_mut::<Battle>().text.skill_learned = " kifejlődött!".into();
    app.add_systems(
        Update,
        (apply_victory_rewards.before(outcome_input), outcome_input),
    );
    app.world_mut()
        .resource_mut::<Battle>()
        .finish(BattleOutcome::Victory);
    app.update();
    assert_eq!(
        app.world().resource::<Progression>().level(&ron),
        2,
        "the level rises on entering the outcome, not next fight"
    );
    assert_eq!(
        app.world().resource::<Inventory>().gold(),
        30,
        "gold is paid once, at victory time"
    );
    assert!(
        app.world()
            .resource::<Battle>()
            .log
            .iter()
            .any(|l| l == "Ron  2. szintre lépett"),
        "a level-up line is staged into the battle log"
    );
    assert!(
        app.world()
            .resource::<Battle>()
            .log
            .iter()
            .any(|l| l == "Tűzcsapás kifejlődött!"),
        "crossing level 2 learns and logs the level-2 skill"
    );
    let total_once = app.world().resource::<Progression>().total(&ron);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<Inventory>().gold(),
        30,
        "gold is not double-paid on confirm"
    );
    assert_eq!(
        app.world().resource::<Progression>().total(&ron),
        total_once,
        "experience is not double-applied on confirm"
    );
}

use super::*;

#[test]
fn victory_grants_each_drop_once_and_waits_until_all_reward_pages_are_confirmed() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let items = crate::assets::load_ron::<Vec<amnezia_data::ItemDef>>(&format!(
        "{}/items.ron",
        crate::assets::asset_root()
    ));
    let mut battle = crate::battle::model::testkit::build_1v2();
    battle.items = items.clone();
    for enemy in &mut battle.enemies {
        enemy.hp = 0;
        enemy.drop_id = 139;
        enemy.drop_prob = 100;
    }
    battle.text.item_received = " a tiéd!".into();
    battle.finish(BattleOutcome::Victory);
    app.insert_resource(battle);
    app.insert_resource(GameData {
        actors: vec![],
        items,
        skills: vec![],
    });
    app.init_resource::<Inventory>();
    app.init_resource::<Progression>();
    app.init_resource::<Vitals>();
    app.init_resource::<BattleActive>();
    app.init_resource::<BattleResult>();
    app.init_resource::<MapBgm>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_message::<AudioRequest>();
    app.add_systems(Update, (apply_victory_rewards, outcome_input).chain());
    app.update();
    assert_eq!(app.world().resource::<Inventory>().count(139), 2);
    assert_eq!(
        app.world()
            .resource::<Battle>()
            .log
            .iter()
            .filter(|l| *l == "Topáz a tiéd!")
            .count(),
        2
    );
    app.update();
    assert_eq!(app.world().resource::<Inventory>().count(139), 2);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert!(app.world().resource::<Battle>().phase == Phase::Outcome);
    assert_eq!(app.world().resource::<BattleResult>().0, None);
    assert!(
        crate::battle::outcome_text::page(app.world().resource::<Battle>())
            .iter()
            .any(|l| l == "Topáz a tiéd!")
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<BattleResult>().0,
        Some(BattleOutcome::Victory)
    );
    assert_eq!(app.world().resource::<Inventory>().count(139), 2);
}

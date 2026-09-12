use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .add_message::<MapChanged>()
        .init_resource::<BattleActive>()
        .init_resource::<MenuOpen>()
        .init_resource::<ShopOpen>()
        .init_resource::<GameOverActive>()
        .insert_resource(TitleActive(false))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add_plugins(WeatherPlugin);
    app.world_mut().spawn((MainCamera, Transform::default()));
    *app.world_mut().resource_mut::<Weather>() = Weather::Rain;
    app.world_mut().resource_mut::<WeatherStrength>().0 = 1;
    for _ in 0..20 {
        app.update();
    }
    app
}

fn entities(app: &mut App) -> Vec<Entity> {
    let mut values = app
        .world_mut()
        .query_filtered::<Entity, With<Sprite>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    values.sort();
    assert!(!values.is_empty());
    values
}

#[test]
fn arriving_on_a_map_does_not_replace_the_live_rain() {
    let mut app = app();
    let before = entities(&mut app);
    app.world_mut().write_message(MapChanged);
    app.update();
    assert_eq!(entities(&mut app), before);
}

#[test]
fn changing_rain_strength_keeps_the_live_renderer() {
    let mut app = app();
    let before = entities(&mut app);
    app.world_mut().resource_mut::<WeatherStrength>().0 = 2;
    app.update();
    for entity in before {
        assert!(app.world().get::<Sprite>(entity).is_some());
    }
}

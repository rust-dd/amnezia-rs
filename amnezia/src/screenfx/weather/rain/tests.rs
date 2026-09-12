use super::*;

mod transfers;

fn app(fps: u32) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<WeatherStrength>()
        .insert_resource(Weather::Rain)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / f64::from(fps)),
        ));
    register(&mut app);
    app.insert_resource(Rain::new(crate::interpreter::EventRng::seeded(31415)));
    app.update();
    app
}

#[test]
fn scene_pauses_hold_the_fraction_and_rain_without_catching_up() {
    for scene in 0..4 {
        let mut app = app(144);
        app.update();
        let initial = app.world().resource::<Rain>().drops.clone();
        let fraction = app.world().resource::<Rain>().fraction;
        match scene {
            0 => {
                app.insert_resource(crate::menu::MenuOpen(true));
            }
            1 => {
                app.insert_resource(crate::shop::ShopOpen(true));
            }
            2 => {
                app.insert_resource(crate::title::TitleActive(true));
            }
            _ => {
                app.insert_resource(crate::gameover::GameOverActive(true));
            }
        }
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(app.world().resource::<Rain>().drops, initial);
        assert_eq!(app.world().resource::<Rain>().fraction, fraction);
        app.world_mut().remove_resource::<crate::menu::MenuOpen>();
        app.world_mut().remove_resource::<crate::shop::ShopOpen>();
        app.world_mut()
            .remove_resource::<crate::title::TitleActive>();
        app.world_mut()
            .remove_resource::<crate::gameover::GameOverActive>();
        app.update();
        assert_eq!(app.world().resource::<Rain>().drops, initial);
        app.update();
        assert_ne!(app.world().resource::<Rain>().drops, initial);
    }
}

#[test]
fn normal_battle_keeps_rain_running_above_the_battlers() {
    let mut app = app(60);
    let before = app.world().resource::<Rain>().drops.clone();
    app.insert_resource(crate::battle::BattleActive(true));
    app.update();
    assert_ne!(app.world().resource::<Rain>().drops, before);
    let (layers, transform) = app
        .world_mut()
        .query_filtered::<(&RenderLayers, &Transform), With<Canvas>>()
        .single(app.world())
        .unwrap();
    assert_eq!(*layers, crate::animation::overlay_layer());
    assert_eq!(transform.translation, Vec3::new(0.0, 0.0, 250.0));
}

#[test]
fn disabled_rain_keeps_its_drops_and_strength_changes_do_not_restart_them() {
    let mut app = app(60);
    let before = app.world().resource::<Rain>().drops.clone();
    app.insert_resource(Weather::None);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(app.world().resource::<Rain>().drops, before);
    app.world_mut().resource_mut::<WeatherStrength>().0 = 2;
    app.update();
    assert_eq!(app.world().resource::<Rain>().drops, before);
    app.insert_resource(Weather::Rain);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.update();
    assert_eq!(app.world().resource::<Rain>().drops, before);
}

#[test]
fn session_reset_discards_old_particle_lifetimes_and_scroll() {
    let mut app = app(60);
    app.world_mut()
        .resource_mut::<Rain>()
        .drops
        .fill(model::Drop {
            x: -999,
            y: 999,
            life: 0,
        });
    app.world_mut().resource_mut::<Scroll>().pan = Vec2::new(12.0, 13.0);
    crate::screenfx::reset_transient(app.world_mut());
    assert_eq!(*app.world().resource::<Scroll>(), Scroll::default());
    assert_eq!(app.world().resource::<Rain>().drops.len(), 100);
    assert!(
        app.world()
            .resource::<Rain>()
            .drops
            .iter()
            .all(|drop| drop.x >= 0 && drop.y < 160)
    );
}

#[test]
fn camera_tracking_preserves_fractional_scroll_and_ignores_teleport_recentering() {
    let mut app = app(60);
    let mut camera = crate::player::CameraPan::default();
    camera.position = Some(Vec2::ZERO);
    app.insert_resource(camera);
    app.update();
    app.world_mut()
        .resource_mut::<crate::player::CameraPan>()
        .position = Some(Vec2::new(0.25, -0.5));
    app.update();
    assert_eq!(
        app.world().resource::<Scroll>().pan,
        Vec2::new(319.75, 159.5)
    );
    app.world_mut().write_message(MapChanged);
    app.world_mut()
        .resource_mut::<crate::player::CameraPan>()
        .position = Some(Vec2::new(100.0, -80.0));
    app.update();
    assert_eq!(
        app.world().resource::<Scroll>().pan,
        Vec2::new(319.75, 159.5)
    );
    app.world_mut()
        .resource_mut::<crate::player::CameraPan>()
        .position = Some(Vec2::new(101.0, -79.0));
    app.update();
    assert_eq!(app.world().resource::<Scroll>().pan, Vec2::new(318.75, 0.5));
}

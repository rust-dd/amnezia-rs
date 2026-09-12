use super::*;

#[test]
fn battle_shake_moves_the_scene_without_moving_windows_or_normal_animation_cells() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .insert_resource(Battle {
            phase: Phase::PartyCommand,
            ..default()
        })
        .init_resource::<crate::battle::BattleActive>()
        .init_resource::<crate::menu::MenuOpen>()
        .init_resource::<crate::shop::ShopOpen>()
        .init_resource::<crate::title::TitleActive>()
        .init_resource::<crate::gameover::GameOverActive>()
        .init_resource::<crate::timing::GameFrames>()
        .add_message::<crate::world::MapChanged>()
        .add_message::<crate::animation::BattlerFlash>()
        .add_plugins(crate::screenfx::ScreenFxPlugin);
    register(&mut app);
    let base = Vec2::new(16.0, -24.0);
    let enemy = app
        .world_mut()
        .spawn((
            SceneEntity,
            Battler {
                index: 0,
                base,
                height: 48.0,
            },
            Sprite::default(),
            Transform::from_translation(overlay_translation(base, BATTLER_Z)),
        ))
        .id();
    let backdrop = app
        .world_mut()
        .spawn((SceneEntity, Transform::from_xyz(0.0, 0.0, BACKDROP_Z)))
        .id();
    let other = app
        .world_mut()
        .spawn(Transform::from_xyz(40.0, 30.0, 510.0))
        .id();
    let canvas = app
        .world_mut()
        .spawn((
            SceneEntity,
            Canvas,
            Transform::from_xyz(0.0, 0.0, BACKDROP_Z - 1.0),
        ))
        .id();
    app.update();
    app.world_mut()
        .write_message(crate::screenfx::ScreenEffect::Shake {
            power: 3,
            speed: 5,
            secs: 8.0 / 60.0,
        });
    for x in [5.0, 5.0, 2.0, -2.0, -6.0, -6.0, -4.0, 0.0, 0.0] {
        app.update();
        assert_eq!(
            app.world().get::<Transform>(backdrop).unwrap().translation,
            Vec3::new(x, 0.0, BACKDROP_Z)
        );
        assert_eq!(
            app.world().get::<Transform>(enemy).unwrap().translation,
            Vec3::new(base.x + x, -base.y, BATTLER_Z)
        );
        assert_eq!(
            app.world().get::<Transform>(other).unwrap().translation,
            Vec3::new(40.0, 30.0, 510.0)
        );
        assert_eq!(
            app.world().get::<Transform>(canvas).unwrap().translation,
            Vec3::new(0.0, 0.0, BACKDROP_Z - 1.0)
        );
    }
}

#[test]
fn original_monster_hues_reach_each_live_battler_sprite() {
    use crate::battle::model::{Progression, Vitals, testkit};
    let monsters = crate::assets::load_ron::<Vec<amnezia_data::MonsterDef>>(&format!(
        "{}/monsters.ron",
        crate::assets::asset_root(),
    ));
    let ids = [2, 12, 14, 15, 18, 30, 37];
    let troop = testkit::troop(&ids.map(|id| (id, 160, 120)));
    let hero = testkit::actor(1, 2, 63, 37);
    let mut battle = Battle::build(
        &troop,
        &monsters,
        &[&hero],
        &[testkit::slots(&hero)],
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        1,
    );
    battle.generation = 1;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<crate::screenfx::TintState>()
        .insert_resource(battle)
        .add_systems(Update, sync_scene);
    app.world_mut()
        .resource_mut::<crate::screenfx::TintState>()
        .set_tone([50.0, 100.0, 150.0, 0.0]);
    app.update();
    let world = app.world_mut();
    assert!(
        world
            .query_filtered::<&Sprite, With<SceneEntity>>()
            .iter(world)
            .any(|sprite| {
                sprite.color == Color::BLACK && sprite.custom_size == Some(Vec2::new(320.0, 240.0))
            }),
        "battle has a fixed black canvas beneath the moving backdrop"
    );
    let backdrop = world
        .query_filtered::<&Sprite, (With<SceneEntity>, Without<Battler>, Without<Canvas>)>()
        .single(world)
        .unwrap();
    assert_eq!(backdrop.custom_size, Some(Vec2::new(320.0, 240.0)));
    assert_eq!(
        backdrop.image_mode,
        SpriteImageMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1.0,
        }
    );
    let mut query = world.query::<(&Battler, &Sprite, &crate::legacy_colors::hue::HueShift)>();
    assert_eq!(query.iter(world).count(), 7);
    for (battler, sprite, hue) in query.iter(world) {
        let expected = monsters
            .iter()
            .find(|m| m.id == ids[battler.index])
            .unwrap();
        assert_eq!(hue.degrees, expected.battler_hue);
        assert_eq!(sprite.image, hue.original);
    }
    assert_eq!(
        world
            .query_filtered::<Entity, (
                With<SceneEntity>,
                With<crate::legacy_colors::tone::SpriteTone>
            )>()
            .iter(world)
            .count(),
        8,
        "the backdrop and every enemy must follow the screen tone"
    );
    assert!(
        world
            .query::<&SpriteTone>()
            .iter(world)
            .all(|tone| tone.0 == [50.0, 100.0, 150.0, 0.0])
    );
}

#[test]
fn battle_screen_tone_updates_only_live_scene_sprites() {
    let mut app = App::new();
    app.init_resource::<crate::screenfx::TintState>()
        .add_systems(PostUpdate, sync_tone);
    let battle = app
        .world_mut()
        .spawn((SceneEntity, SpriteTone::default()))
        .id();
    let other_layer = app.world_mut().spawn(SpriteTone::default()).id();
    for value in [
        [50.0, 100.0, 150.0, 0.0],
        [200.0, 200.0, 200.0, 100.0],
        [100.0; 4],
    ] {
        app.world_mut()
            .resource_mut::<crate::screenfx::TintState>()
            .set_tone(value);
        app.update();
        assert_eq!(app.world().get::<SpriteTone>(battle).unwrap().0, value);
        assert_eq!(
            app.world().get::<SpriteTone>(other_layer).unwrap().0,
            [100.0; 4]
        );
    }
}

#[test]
fn battler_base_and_translation_map_a_centre_to_the_overlay() {
    let base = battler_base(176, 96);
    assert_eq!(base, Vec2::new(16.0, -24.0));
    assert_eq!(
        overlay_translation(base, BATTLER_Z),
        Vec3::new(16.0, 24.0, BATTLER_Z)
    );
}

#[test]
fn battler_look_keeps_natural_colors_and_original_death_alpha() {
    assert_eq!(battler_look(true, None), (Color::WHITE, 1.0));
    assert_eq!(battler_look(false, None).0.alpha(), 0.0);
    let death = Dying {
        elapsed: 0.0,
        secs: 0.6,
        explode: false,
    };
    let (color, zoom) = battler_look(false, Some(&death));
    assert!(color.alpha() > 0.9);
    assert_eq!(zoom, 1.0);
    let boom = Dying {
        elapsed: 0.4,
        secs: 0.4,
        explode: true,
    };
    let (color, zoom) = battler_look(false, Some(&boom));
    assert!(zoom > 1.9);
    assert!(color.alpha() < 0.01);
}

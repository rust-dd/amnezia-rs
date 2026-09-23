use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Fx>()
        .init_resource::<crate::screenfx::flash::channel::Inbox>()
        .init_resource::<TintState>()
        .init_resource::<crate::battle::Battle>()
        .init_resource::<crate::battle::BattleActive>()
        .add_message::<ScreenEffect>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 144.0),
        ))
        .add_systems(Update, (tone::update_tone, step_effects).chain());
    app.update();
    app
}

fn begin(app: &mut App) {
    for effect in [
        ScreenEffect::tint(&[70, 90, 110, 50, 30, 0]),
        ScreenEffect::flash(&[31, 10, 5, 20, 30, 0]),
        ScreenEffect::shake(&[3, 5, 30, 0]),
    ] {
        app.world_mut().write_message(effect);
    }
    for _ in 0..10 {
        app.update();
    }
}

fn pause(world: &mut World, scene: usize, active: bool) {
    match scene {
        0 => {
            world.insert_resource(crate::menu::MenuOpen(active));
        }
        1 => {
            world.insert_resource(crate::shop::ShopOpen(active));
        }
        2 => {
            world.insert_resource(crate::title::TitleActive(active));
        }
        _ => {
            world.insert_resource(crate::gameover::GameOverActive(active));
        }
    }
}

#[test]
fn menu_shop_title_and_gameover_preserve_effects_without_catchup() {
    for scene in 0..4 {
        let mut app = app();
        begin(&mut app);
        let expected = saved::snapshot(app.world());
        pause(app.world_mut(), scene, true);
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(saved::snapshot(app.world()), expected);
        pause(app.world_mut(), scene, false);
        app.update();
        let continued = saved::snapshot(app.world());
        assert_ne!(continued, expected);
        assert!(continued.valid());
        assert!(continued.tone.tone()[0] > 95.0);
        assert!(app.world().resource::<Fx>().flash.is_some());
    }
}

#[test]
fn the_normal_battle_scene_advances_tone_flash_and_shake() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<crate::battle::BattleActive>()
        .0 = true;
    app.world_mut()
        .resource_mut::<crate::battle::Battle>()
        .phase = crate::battle::Phase::PartyCommand;
    app.update();
    begin(&mut app);
    let expected = saved::snapshot(app.world());
    for _ in 0..20 {
        app.update();
    }
    assert_ne!(saved::snapshot(app.world()), expected);
    assert_ne!(app.world().resource::<Fx>().shake_offset, Vec2::ZERO);
    assert!(app.world().resource::<Fx>().flash.is_some());
    assert!(app.world().resource::<TintState>().tone()[0] < expected.tone.tone()[0]);
}

use super::*;
use crate::player::{CameraPan, Player};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport, TeleportPlugin};
use crate::world::{MapData, MapEvents, MoveQueue, RouteStepper};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        crate::transitions::TransitionPlugin,
        TeleportPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<Switches>()
    .init_resource::<Variables>()
    .init_resource::<Party>()
    .init_resource::<Inventory>()
    .init_resource::<CameraPan>()
    .init_resource::<MapEvents>()
    .init_resource::<crate::battle::BattleActive>()
    .init_resource::<crate::menu::MenuOpen>()
    .init_resource::<crate::shop::ShopOpen>()
    .init_resource::<crate::gameover::GameOverActive>()
    .insert_resource(crate::title::TitleActive(false))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ))
    .add_plugins(ScreenFxPlugin);
    let mut map = MapData::for_test(20, 15);
    map.map_id = 3;
    app.insert_resource(map);
    app.world_mut().spawn((
        Player {
            tile_x: 15,
            tile_y: 12,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        },
        Transform::default(),
        MoveQueue::default(),
        RouteStepper::default(),
    ));
    app.update();
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([70.0, 90.0, 110.0, 50.0]);
    let mut fx = app.world_mut().resource_mut::<Fx>();
    apply_effect(&mut fx, &ScreenEffect::flash(&[31, 10, 5, 20, 100, 0]));
    apply_effect(&mut fx, &ScreenEffect::shake(&[3, 5, 100, 0]));
    step_flash(&mut fx, 0.125);
    step_shake(&mut fx, 0.125);
    app
}

fn transfer(app: &mut App, map_id: u32, reload: bool) {
    if reload {
        app.world_mut()
            .resource_mut::<PendingTeleport>()
            .reload(map_id, 8, 7);
    } else {
        app.world_mut().resource_mut::<PendingTeleport>().0 = Some((map_id, 8, 7));
    }
    for frame in 0..80 {
        app.world_mut()
            .resource_mut::<crate::timing::GameFrames>()
            .frame = frame;
        app.update();
    }
    assert!(!app.world().resource::<Fade>().busy());
    assert_eq!(app.world().resource::<MapData>().map_id, map_id);
}

#[test]
fn same_map_teleports_preserve_flash_tone_and_shake() {
    let mut app = app();
    let expected = saved::snapshot(app.world());
    transfer(&mut app, 3, false);
    assert_eq!(saved::snapshot(app.world()), expected);
}

#[test]
fn relocation_and_the_scheduled_projection_apply_camera_shake_only_once() {
    let mut app = app();
    let camera = app
        .world_mut()
        .spawn((MainCamera, Transform::default()))
        .id();
    assert_ne!(app.world().resource::<Fx>().shake_offset, Vec2::ZERO);
    crate::player::relocate_camera(app.world_mut());
    let before = app.world().get::<Transform>(camera).unwrap().translation;
    for _ in 0..3 {
        app.world_mut()
            .run_system_cached(apply_camera_shake)
            .unwrap();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            before
        );
    }
}

#[test]
fn a_stop_command_removes_the_current_shake_before_a_snapshot_without_aging_the_flash() {
    let mut app = app();
    let camera = app
        .world_mut()
        .spawn((MainCamera, Transform::default()))
        .id();
    crate::player::relocate_camera(app.world_mut());
    assert_ne!(
        app.world().get::<Transform>(camera).unwrap().translation.x,
        0.0
    );
    let flash = app.world().resource::<Fx>().flash.clone();
    app.world_mut()
        .write_message(ScreenEffect::shake(&[0, 0, 0, 0]));
    for _ in 0..3 {
        apply_pending(app.world_mut());
        assert_eq!(app.world().resource::<Fx>().shake_offset, Vec2::ZERO);
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation.x,
            0.0
        );
        assert_eq!(app.world().resource::<Fx>().flash, flash);
    }
}

#[test]
fn rebuilt_maps_clear_the_old_flash_but_keep_tone_and_shake() {
    for (map_id, reload) in [(2, false), (3, true)] {
        let mut app = app();
        let tone = app.world().resource::<TintState>().clone();
        let shake = app.world().resource::<Fx>().shake.clone();
        transfer(&mut app, map_id, reload);
        assert!(app.world().resource::<Fx>().flash.is_none());
        assert_eq!(app.world().resource::<TintState>(), &tone);
        assert_eq!(app.world().resource::<Fx>().shake, shake);
        let overlay = app
            .world_mut()
            .query_filtered::<&Sprite, With<FlashOverlay>>()
            .single(app.world())
            .unwrap();
        assert_eq!(overlay.color, Color::NONE);
    }
}

#[test]
fn quick_transfers_preserve_the_live_flash_tone_shake_and_weather_scroll() {
    let mut app = app();
    weather::rain::Scroll::restore(app.world_mut(), [12.25, 149.5]);
    let expected = saved::snapshot(app.world());
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .quick(4, 7, 6);
    crate::teleport::flush_quick(app.world_mut());
    app.update();
    assert_eq!(app.world().resource::<MapData>().map_id, 4);
    assert_eq!(saved::snapshot(app.world()), expected);
    let overlay = app
        .world_mut()
        .query_filtered::<&Sprite, With<FlashOverlay>>()
        .single(app.world())
        .unwrap();
    assert_ne!(overlay.color, Color::NONE);
}

#[test]
fn quick_relocation_preserves_the_current_shake_in_immediate_screen_coordinates() {
    let mut app = app();
    let camera = app
        .world_mut()
        .spawn((MainCamera, Transform::default()))
        .id();
    let shake = app.world().resource::<Fx>().shake_offset;
    assert_ne!(shake, Vec2::ZERO);
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .quick(4, 7, 6);
    crate::teleport::flush_quick(app.world_mut());
    let position = app
        .world()
        .get::<Transform>(camera)
        .unwrap()
        .translation
        .truncate();
    assert_eq!(position, shake);
    assert_eq!(
        app.world().resource::<CameraPan>().position,
        Some(Vec2::ZERO)
    );
}

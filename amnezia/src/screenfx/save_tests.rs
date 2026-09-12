use super::*;
use crate::save::{EventSaveRequest, LoadRequest, SavePlugin};
use crate::world::{MapData, MapRebuilt};

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path =
        std::env::temp_dir().join(format!("amnezia_screen_{tag}_{}.ron", std::process::id()));
    let mut app = crate::save::tests::save_resources(path.clone());
    app.add_plugins(SavePlugin)
        .init_resource::<Fx>()
        .init_resource::<crate::interpreter::RunningEvent>()
        .add_message::<MapRebuilt>()
        .add_message::<ScreenEffect>()
        .add_systems(Update, tone::update_tone.in_set(ScreenEffectsSet))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.world_mut().spawn(crate::player::Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    (app, path)
}

fn save(app: &mut App) {
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
}

fn reload(app: &mut App) {
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(MapRebuilt);
    app.update();
}

fn active_effects(app: &mut App) -> saved::ScreenState {
    weather::rain::Scroll::restore(app.world_mut(), [12.25, 149.5]);
    app.world_mut()
        .write_message(ScreenEffect::tint(&[70, 90, 110, 50, 30, 0]));
    app.update();
    tone::step_tint(&mut app.world_mut().resource_mut::<TintState>(), 0.4375);
    let mut fx = app.world_mut().resource_mut::<Fx>();
    apply_effect(&mut fx, &ScreenEffect::flash(&[31, 10, 5, 20, 30, 0]));
    apply_effect(&mut fx, &ScreenEffect::shake(&[3, 5, 30, 0]));
    step_flash(&mut fx, 0.4375);
    step_shake(&mut fx, 0.4375);
    saved::snapshot(app.world())
}

#[test]
fn full_effect_state_survives_the_file_scene_cleanup_and_rebuilt_map() {
    let (mut app, path) = app("full_state");
    let expected = active_effects(&mut app);
    save(&mut app);
    assert!(std::fs::read_to_string(&path).unwrap().contains(&format!(
        "format_version: {},",
        crate::save::SAVE_FORMAT_VERSION
    )));
    app.insert_resource(TintState::default());
    app.insert_resource(Fx::default());
    reload(&mut app);
    assert_eq!(saved::snapshot(app.world()), expected);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn screen_restore_waits_for_its_rebuilt_map_and_does_not_repeat() {
    let (mut app, path) = app("arrival");
    let expected = active_effects(&mut app);
    save(&mut app);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(!app.world().contains_resource::<saved::Pending>());
    assert_eq!(saved::snapshot(app.world()), expected);
    app.insert_resource(Fx::default());
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(app.world().resource::<Fx>().flash.is_none());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn nonfinite_screen_data_does_not_change_live_state_or_the_save_file() {
    let (mut app, path) = app("invalid");
    active_effects(&mut app);
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([70.125, 90.0, 110.0, 50.0]);
    let expected = saved::snapshot(app.world());
    save(&mut app);
    let original = std::fs::read_to_string(&path).unwrap();
    let invalid = original.replace("70.125", "NaN");
    assert_ne!(invalid, original);
    std::fs::write(&path, &invalid).unwrap();
    app.world_mut()
        .resource_mut::<crate::state::Switches>()
        .set(99, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(
        app.world().resource::<crate::save::LoadOutcome>().0,
        Some(false)
    );
    assert_eq!(saved::snapshot(app.world()), expected);
    assert!(app.world().resource::<crate::state::Switches>().get(99));
    assert!(!app.world().contains_resource::<saved::Pending>());
    assert!(
        app.world()
            .resource::<crate::teleport::PendingTeleport>()
            .0
            .is_none()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn loading_keeps_fractional_tone_channels_instead_of_rounding_them() {
    let (mut app, path) = app("fractional_tone");
    let expected = [70.125, 90.25, 110.375, 80.5];
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone(expected);
    save(&mut app);
    app.insert_resource(TintState::default());
    reload(&mut app);
    std::fs::remove_file(path).unwrap();
    assert_eq!(app.world().resource::<TintState>().tone(), expected);
}

#[test]
fn legacy_screen_defaults_keep_the_old_tone_without_resuming_stale_effects() {
    for version in 0..6 {
        let (mut app, path) = app(&format!("legacy_{version}"));
        let previous = active_effects(&mut app);
        saved::prepare(app.world_mut(), 2, Some(previous));
        let original = format!(
            "(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,tone:(70,90,110,50))"
        );
        std::fs::write(&path, &original).unwrap();
        reload(&mut app);
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(true)
        );
        assert!(!app.world().contains_resource::<saved::Pending>());
        assert_eq!(
            app.world().resource::<TintState>().tone(),
            [70.0, 90.0, 110.0, 50.0]
        );
        assert!(app.world().resource::<Fx>().flash.is_none());
        assert_eq!(app.world().resource::<Fx>().shake_offset, Vec2::ZERO);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn map_rebuild_cleanup_precedes_screen_restore_and_paused_effect_updates() {
    let (mut app, path) = app("order");
    let expected = active_effects(&mut app);
    save(&mut app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.add_systems(
        Update,
        (
            (|world: &mut World| {
                if world.contains_resource::<saved::Pending>() {
                    reset_transient(world);
                    world.insert_resource(TintState::default());
                    world.insert_resource(crate::menu::MenuOpen(true));
                    world.write_message(MapRebuilt);
                }
            })
            .in_set(crate::teleport::MapTransfer),
            step_effects.in_set(ScreenEffectsSet),
        ),
    );
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(saved::snapshot(app.world()), expected);
    app.update();
    assert_eq!(saved::snapshot(app.world()), expected);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn loading_keeps_the_active_flash_and_exact_shake_position() {
    let (mut app, path) = app("flash_shake");
    {
        let mut fx = app.world_mut().resource_mut::<Fx>();
        apply_effect(&mut fx, &ScreenEffect::flash(&[31, 10, 5, 20, 30, 0]));
        apply_effect(&mut fx, &ScreenEffect::shake(&[3, 5, 30, 0]));
        step_flash(&mut fx, 0.125);
        step_shake(&mut fx, 0.125);
    }
    let fx = app.world().resource::<Fx>();
    let flash = fx.flash.as_ref().unwrap().color();
    let shake = fx.shake_offset;
    assert_ne!(shake, Vec2::ZERO);
    save(&mut app);
    reload(&mut app);
    std::fs::remove_file(path).unwrap();
    let fx = app.world().resource::<Fx>();
    assert_eq!(fx.flash.as_ref().map(|flash| flash.color()), Some(flash));
    assert_eq!(fx.shake_offset, shake);
}

use super::*;
use crate::save::{EventSaveRequest, LoadRequest, SavePlugin};
use crate::world::MapData;
use bevy::ecs::system::RunSystemOnce;

fn snapshot(world: &mut World) -> Vec<saved::PictureState> {
    world
        .run_system_once(|capture: saved::Capture| capture.snapshot())
        .unwrap()
}

fn save_app(tag: &str) -> (App, std::path::PathBuf) {
    let path =
        std::env::temp_dir().join(format!("amnezia_picture_{tag}_{}.ron", std::process::id()));
    let mut app = crate::save::tests::save_resources(path.clone());
    app.add_plugins((
        AssetPlugin {
            file_path: crate::assets::asset_root().into(),
            ..default()
        },
        SavePlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Mesh>()
    .init_asset::<render::PictureMaterial>()
    .init_resource::<crate::interpreter::RunningEvent>()
    .add_message::<PictureCommand>()
    .add_message::<MapRebuilt>()
    .add_message::<MapEffectsReset>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ))
    .add_systems(Startup, render::setup_picture_mesh)
    .add_systems(
        Update,
        (
            clear_on_map_change,
            saved::restore,
            render::apply_commands,
            render::size_pictures,
            drive_tweens,
        )
            .chain(),
    );
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
    app.world_mut()
        .spawn((crate::world::MainCamera, Transform::default()));
    (app, path)
}

fn show(app: &mut App, id: u32, name: &str) {
    app.world_mut().write_message(PictureCommand::show(
        id,
        name,
        160.0,
        120.0,
        &[0, 0, 0, 0, 0, 120, 60, 0, 100, 100, 100, 100, 2, 1],
    ));
    app.update();
}

#[test]
fn saving_a_live_picture_keeps_its_graphic_in_the_slot() {
    let (mut app, path) = save_app("graphic");
    show(&mut app, 1, "Fog");
    assert_eq!(count_pictures(&mut app), 1);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        text.contains("\"Fog\""),
        "the live picture asset must be saved"
    );
}

#[test]
fn restored_pictures_replace_the_previous_session_after_its_scene_is_cleared() {
    let (mut app, path) = save_app("restore");
    show(&mut app, 1, "Fog");
    show(&mut app, 2, "Cross");
    app.world_mut().write_message(PictureCommand::move_to(
        1,
        90.0,
        70.0,
        &[1, 0, 90, 70, 0, 75, 10, 0, 50, 120, 180, 0, 2, 5, 30],
    ));
    app.update();
    for mut picture in app
        .world_mut()
        .query::<&mut Picture>()
        .iter_mut(app.world_mut())
    {
        picture.advance(0.4375);
    }
    let expected = snapshot(app.world_mut());
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().write_message(PictureCommand::erase(1));
    app.world_mut().write_message(PictureCommand::erase(2));
    show(&mut app, 9, "Staff1");
    assert_eq!(count_pictures(&mut app), 1);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(MapEffectsReset);
    app.update();
    assert_eq!(snapshot(app.world_mut()), expected);
    std::fs::remove_file(path).unwrap();
    let mut ids = app
        .world_mut()
        .query::<&Picture>()
        .iter(app.world())
        .map(|p| p.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    assert_eq!(ids, [1, 2]);
}

#[test]
fn picture_restore_waits_for_a_rebuilt_destination_and_is_applied_only_once() {
    let (mut app, path) = save_app("arrival");
    show(&mut app, 1, "Fog");
    let expected = snapshot(app.world_mut());
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().write_message(PictureCommand::erase(1));
    app.update();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
    assert_eq!(count_pictures(&mut app), 0);
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(MapEffectsReset);
    app.update();
    assert_eq!(count_pictures(&mut app), 0);
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(MapEffectsReset);
    app.update();
    assert_eq!(snapshot(app.world_mut()), expected);
    assert!(!app.world().contains_resource::<saved::Pending>());
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
    assert_eq!(snapshot(app.world_mut()), expected);
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(MapEffectsReset);
    app.update();
    assert_eq!(count_pictures(&mut app), 0);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_saved_empty_picture_list_removes_the_previous_session_images() {
    let (mut app, path) = save_app("empty");
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    show(&mut app, 1, "Fog");
    let original = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(MapEffectsReset);
    app.update();
    assert_eq!(count_pictures(&mut app), 0);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_saves_without_picture_data_start_empty_and_leave_the_slot_untouched() {
    for version in 0..5 {
        let (mut app, path) = save_app(&format!("legacy_{version}"));
        show(&mut app, 1, "Fog");
        let original = format!(
            "(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)"
        );
        std::fs::write(&path, &original).unwrap();
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(true)
        );
        app.world_mut().write_message(MapRebuilt);
        app.world_mut().write_message(MapEffectsReset);
        app.update();
        assert_eq!(count_pictures(&mut app), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_picture_data_leaves_live_images_the_session_and_the_file_unchanged() {
    let (mut app, path) = save_app("invalid");
    show(&mut app, 1, "Fog");
    let expected = snapshot(app.world_mut());
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let original = std::fs::read_to_string(&path).unwrap();
    let invalid = original.replace("id: 1,", "id: 0,");
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
    assert_eq!(snapshot(app.world_mut()), expected);
    assert!(app.world().resource::<crate::state::Switches>().get(99));
    assert!(
        app.world()
            .resource::<crate::teleport::PendingTeleport>()
            .0
            .is_none()
    );
    assert!(!app.world().contains_resource::<saved::Pending>());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    std::fs::remove_file(path).unwrap();
}

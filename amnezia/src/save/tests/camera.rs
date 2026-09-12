use super::*;
use crate::player::CameraPan;
use crate::world::MapChanged;

fn camera_app(path: PathBuf) -> App {
    let mut app = save_resources(path);
    app.add_plugins(SavePlugin)
        .init_resource::<RunningEvent>()
        .init_resource::<CameraPan>();
    let mut map = MapData::for_test(140, 140);
    map.map_id = 13;
    app.insert_resource(map);
    app.world_mut().spawn(Player {
        tile_x: 60,
        tile_y: 60,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app
}

#[test]
fn saved_camera_lock_position_and_pending_pan_survive_the_map_reload() {
    let path = temp_slot("camera_reload");
    let mut app = camera_app(path.clone());
    {
        let mut pan = app.world_mut().resource_mut::<CameraPan>();
        pan.locked = true;
        pan.offset = Vec2::new(32.0, -48.0);
        pan.target = Vec2::new(64.0, -96.0);
        pan.speed = 30.0;
        pan.position = Some(Vec2::new(160.0, -240.0));
    }
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.insert_resource(CameraPan::default());
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(MapChanged);
    app.update();
    std::fs::remove_file(path).unwrap();
    let pan = app.world().resource::<CameraPan>();
    assert!(pan.locked);
    assert_eq!(pan.offset, Vec2::new(32.0, -48.0));
    assert_eq!(pan.target, Vec2::new(64.0, -96.0));
    assert_eq!(pan.speed, 30.0);
    assert_eq!(pan.position, Some(Vec2::new(160.0, -240.0)));
}

fn save_checkpoint(
    app: &mut App,
    path: &std::path::Path,
) -> crate::player::saved_camera::CameraState {
    let mut pan = app.world_mut().resource_mut::<CameraPan>();
    pan.locked = true;
    pan.offset = Vec2::new(32.0, -48.0);
    pan.position = Some(Vec2::new(160.0, -240.0));
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    read_save(path).unwrap().camera.unwrap()
}

#[test]
fn a_camera_restore_waits_for_its_map_and_cannot_reapply_on_a_later_transfer() {
    use crate::player::saved_camera::Pending;
    let path = temp_slot("camera_arrival");
    let mut app = camera_app(path.clone());
    let expected = save_checkpoint(&mut app, &path);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    assert_eq!(
        app.world().resource::<CameraPan>().snapshot(),
        CameraPan::default().snapshot()
    );
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(MapChanged);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 13;
    app.world_mut().write_message(MapChanged);
    app.update();
    assert!(!app.world().contains_resource::<Pending>());
    assert_eq!(app.world().resource::<CameraPan>().snapshot(), expected);
    app.world_mut().resource_mut::<CameraPan>().locked = false;
    app.world_mut().write_message(MapChanged);
    app.update();
    assert!(!app.world().resource::<CameraPan>().locked);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn map_transfer_recentering_runs_before_restoring_the_saved_camera() {
    let path = temp_slot("camera_transfer_order");
    let mut app = camera_app(path.clone());
    let expected = save_checkpoint(&mut app, &path);
    app.add_systems(
        Update,
        (|outcome: Res<LoadOutcome>,
          mut done: Local<bool>,
          mut pan: ResMut<CameraPan>,
          mut changed: MessageWriter<MapChanged>| {
            if outcome.0 == Some(true) && !*done {
                *done = true;
                pan.recenter(true);
                changed.write(MapChanged);
            }
        })
        .in_set(crate::teleport::MapTransfer),
    );
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<CameraPan>().snapshot(), expected);
    app.update();
    assert_eq!(app.world().resource::<CameraPan>().snapshot(), expected);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_camera_defaults_cancel_old_pending_state_without_rewriting_the_slot() {
    use crate::player::saved_camera::{Pending, prepare};
    for version in 0..4 {
        let path = temp_slot(&format!("camera_legacy_{version}"));
        let original = format!(
            "(format_version:{version},map_id:13,x:60,y:60,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)"
        );
        std::fs::write(&path, &original).unwrap();
        let mut app = camera_app(path.clone());
        app.world_mut().resource_mut::<CameraPan>().locked = true;
        let previous = app.world().resource::<CameraPan>().snapshot();
        prepare(app.world_mut(), 13, Some(previous));
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert!(!app.world().contains_resource::<Pending>());
        app.world_mut().write_message(MapChanged);
        app.update();
        assert_eq!(
            app.world().resource::<CameraPan>().snapshot(),
            CameraPan::default().snapshot()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_camera_data_does_not_change_the_live_camera_session_or_file() {
    let path = temp_slot("camera_invalid");
    let mut app = camera_app(path.clone());
    let expected = save_checkpoint(&mut app, &path);
    let mut game = read_save(&path).unwrap();
    game.camera.as_mut().unwrap().target[0] = f32::INFINITY;
    write_save(&path, &game).unwrap();
    let original = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<Switches>().set(99, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert_eq!(app.world().resource::<CameraPan>().snapshot(), expected);
    assert!(app.world().resource::<Switches>().get(99));
    assert!(
        !app.world()
            .contains_resource::<crate::player::saved_camera::Pending>()
    );
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

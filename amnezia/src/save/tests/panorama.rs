use super::*;
use crate::panorama::Panorama;

fn app(path: PathBuf) -> App {
    let mut app = save_resources(path);
    app.add_plugins(SavePlugin)
        .init_resource::<RunningEvent>()
        .init_resource::<crate::player::CameraPan>()
        .init_resource::<Panorama>();
    let mut map = MapData::for_test(20, 16);
    map.map_id = 94;
    app.insert_resource(map);
    app.world_mut().spawn(Player {
        tile_x: 9,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app
}

fn snapshot(phase: i64) -> Panorama {
    ron::from_str(&format!(
        "(map_id:Some(94),definition:Some((name:\"Sky\",loop_x:true,loop_y:true,auto_x:true,auto_y:true,speed_x:1,speed_y:-1)),scroll:(0.0,0.0),motion:Some((name:Some(\"Sky\"),size:(640,480),phase:({phase},125),on_map_init:false)),clock:(frame:37,fraction:0.4166666667))"
    )).unwrap()
}

#[test]
fn saving_and_loading_preserve_exact_background_phase_and_clock_without_touching_the_slot() {
    let path = temp_slot("panorama_phase");
    let mut app = app(path.clone());
    let expected = snapshot(20109);
    assert!(expected.valid());
    app.insert_resource(expected.clone());
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let file = std::fs::read(&path).unwrap();
    let saved = read_save(&path).unwrap();
    assert_eq!(saved.format_version, SAVE_FORMAT_VERSION);
    assert_eq!(saved.panorama, Some(expected.clone()));
    app.insert_resource(Panorama::default());
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert_eq!(*app.world().resource::<Panorama>(), expected);
    assert_eq!(std::fs::read(&path).unwrap(), file);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn an_invalid_saved_background_does_not_replace_the_live_game_or_modify_the_file() {
    let path = temp_slot("panorama_invalid_phase");
    let mut app = app(path.clone());
    let expected = snapshot(20109);
    app.insert_resource(expected.clone());
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let mut saved = read_save(&path).unwrap();
    saved.panorama = Some(snapshot(i64::MIN));
    write_save(&path, &saved).unwrap();
    let file = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert_eq!(*app.world().resource::<Panorama>(), expected);
    assert_eq!(std::fs::read(&path).unwrap(), file);
    std::fs::remove_file(path).unwrap();
}

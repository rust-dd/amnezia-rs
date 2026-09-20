use super::*;
use crate::timing::{GameFrames, SceneFrames};

#[test]
fn save_load_keeps_global_animation_phase_and_transition_overrides() {
    let path = temp_slot("frame_clock");
    let mut app = save_app(path.clone());
    app.init_resource::<RunningEvent>();
    app.init_resource::<crate::transitions::Transition>();
    app.init_resource::<crate::transitions::Settings>();
    let defaults = crate::transitions::Defaults([0, 0, 16, 17, 17, 16]);
    app.world_mut()
        .resource_mut::<crate::transitions::Settings>()
        .change(&[0, 20], &defaults);
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.world_mut().spawn(Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app.world_mut().resource_mut::<GameFrames>().advance(12.345);
    app.world_mut().resource_mut::<SceneFrames>().frame = 321;
    let before = *app.world().resource::<GameFrames>();
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().resource_mut::<GameFrames>().advance(100.0);
    app.world_mut().resource_mut::<SceneFrames>().frame = 999;
    app.world_mut()
        .resource_mut::<crate::transitions::Settings>()
        .change(&[0, 0], &defaults);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .hold_black();
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .erased()
    );
    assert_eq!(*app.world().resource::<GameFrames>(), before);
    assert_eq!(app.world().resource::<SceneFrames>().frame, 321);
    assert_eq!(
        app.world()
            .resource::<crate::transitions::Settings>()
            .get(0, &defaults),
        crate::transitions::Kind::None
    );
    let legacy = ron::from_str::<SaveGame>(
        "(map_id:1,x:0,y:0,dir:0,switches:[],variables:[],party:[1],items:[],gold:0)",
    )
    .unwrap();
    assert_eq!(legacy.game_frames, GameFrames::default());
    assert_eq!(legacy.scene_frame, None);
    assert_eq!(legacy.transitions, crate::transitions::Settings::default());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_clock_migration_preserves_phase_and_never_rewrites_the_slot() {
    for version in 0..SAVE_FORMAT_VERSION {
        let path = temp_slot(&format!("legacy_scene_clock_{version}"));
        let source = format!(
            "(format_version:{version},map_id:2,x:0,y:0,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,game_frames:(frame:1234,fraction:0.75))"
        );
        std::fs::write(&path, &source).unwrap();
        let mut app = save_app(path.clone());
        app.init_resource::<RunningEvent>();
        app.world_mut().resource_mut::<SceneFrames>().frame = 99;
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(app.world().resource::<SceneFrames>().frame, 1234);
        let mut resumed = *app.world().resource::<GameFrames>();
        resumed.advance(0.25 / 60.0);
        assert_eq!(resumed.frame, 1235);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn an_explicitly_saved_zero_scene_counter_does_not_fall_back_to_raw_time() {
    let path = temp_slot("zero_scene_clock");
    let source = format!(
        "(format_version:{SAVE_FORMAT_VERSION},map_id:2,x:0,y:0,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,game_frames:(frame:800,fraction:0.5),scene_frame:Some(0))"
    );
    std::fs::write(&path, &source).unwrap();
    let mut app = save_app(path.clone());
    app.init_resource::<RunningEvent>();
    app.world_mut().resource_mut::<SceneFrames>().frame = 99;
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert_eq!(app.world().resource::<GameFrames>().frame, 800);
    assert_eq!(app.world().resource::<SceneFrames>().frame, 0);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    std::fs::remove_file(path).unwrap();
}

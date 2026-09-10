use super::*;
use crate::timing::GameFrames;

#[test]
fn save_load_keeps_global_animation_phase_and_old_slots_default_to_zero() {
    let path = temp_slot("frame_clock");
    let mut app = save_app(path.clone());
    app.init_resource::<RunningEvent>();
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
    let before = *app.world().resource::<GameFrames>();
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().resource_mut::<GameFrames>().advance(100.0);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert_eq!(*app.world().resource::<GameFrames>(), before);
    let legacy = ron::from_str::<SaveGame>(
        "(map_id:1,x:0,y:0,dir:0,switches:[],variables:[],party:[1],items:[],gold:0)",
    )
    .unwrap();
    assert_eq!(legacy.game_frames, GameFrames::default());
    std::fs::remove_file(path).unwrap();
}

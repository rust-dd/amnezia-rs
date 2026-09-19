use super::*;
use std::time::SystemTime;

#[derive(serde::Deserialize)]
struct Metadata {
    #[serde(default)]
    saved_at: Option<u64>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn both_save_request_origins_write_the_current_whole_second_not_the_loaded_timestamp() {
    for event in [false, true] {
        let path = temp_slot(if event {
            "timestamp_event"
        } else {
            "timestamp_menu"
        });
        let original = "(format_version:15,saved_at:Some(1),map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)";
        std::fs::write(&path, original).unwrap();
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
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        if event {
            app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        } else {
            app.world_mut().resource_mut::<SaveRequest>().0 = true;
        }
        let before = now();
        app.update();
        let after = now();
        let contents = std::fs::read_to_string(&path).unwrap();
        let metadata = ron::from_str::<Metadata>(&contents).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(
            metadata
                .saved_at
                .is_some_and(|stamp| (before..=after).contains(&stamp))
        );
    }
}

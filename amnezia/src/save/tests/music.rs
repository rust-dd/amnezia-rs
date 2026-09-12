use super::*;
use crate::audio::{AudioRequest, BgmTrack, CurrentBgm, saved};
use crate::map_bgm::{MapBgmPlugin, MapInfoData, MapMusic};
use crate::title::TitleActive;

fn music_app(path: PathBuf) -> App {
    let mut app = save_app(path);
    saved::register(&mut app);
    app.add_plugins(MapBgmPlugin)
        .init_resource::<TitleActive>()
        .configure_sets(Update, MapMusic.after(save_or_load));
    app.world_mut().resource_mut::<TitleActive>().0 = false;
    app.insert_resource(MapInfoData(vec![amnezia_data::MapInfoDef {
        id: 2,
        parent: 0,
        music_type: 2,
        music: amnezia_data::MusicDef {
            name: "MapDefault".into(),
            volume: 100,
            tempo: 100,
            balance: 50,
            fadein: 0,
        },
    }]));
    app.init_resource::<RunningEvent>()
        .add_message::<AudioRequest>();
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
    app
}

#[test]
fn saving_keeps_the_current_event_selected_music_in_the_file() {
    let path = temp_slot("current_music");
    let mut app = music_app(path.clone());
    app.insert_resource(CurrentBgm::with_track("House", 0.25, 1.1));
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(
        saved.contains("\"House\""),
        "the playing BGM must survive the file snapshot"
    );
    std::fs::remove_file(path).unwrap();
}

fn requests(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect::<Vec<_>>()
}

fn arrive(app: &mut App, map_id: u32) {
    app.insert_resource(PendingTeleport::default());
    app.world_mut().resource_mut::<MapData>().map_id = map_id;
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
}

#[test]
fn restored_music_restarts_after_map_arrival_without_an_intermediate_default_track() {
    for silent in [false, true] {
        let path = temp_slot(&format!("music_arrival_{silent}"));
        let mut app = music_app(path.clone());
        app.insert_resource(if silent {
            CurrentBgm::default()
        } else {
            CurrentBgm::with_track("House", 0.25, 1.1)
        });
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        let saved = read_save(&path).unwrap().music.unwrap();
        assert_eq!(saved.current.is_none(), silent);
        requests(&mut app);
        app.insert_resource(CurrentBgm::with_track("Theme", 1.0, 1.0));
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert!(requests(&mut app).is_empty());
        arrive(&mut app, 2);
        let mut expected = vec![AudioRequest::StopBgm];
        if let Some(track) = saved.current {
            expected.push(track.replay());
        }
        assert_eq!(requests(&mut app), expected);
        app.update();
        assert!(requests(&mut app).is_empty());
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn a_loaded_track_waits_for_the_right_map_and_the_title_to_release_it() {
    let path = temp_slot("music_title_release");
    let mut app = music_app(path.clone());
    app.insert_resource(CurrentBgm::with_track("House", 0.25, 1.1));
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    requests(&mut app);
    app.world_mut().resource_mut::<TitleActive>().0 = true;
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    arrive(&mut app, 3);
    assert!(requests(&mut app).is_empty());
    arrive(&mut app, 2);
    assert!(requests(&mut app).is_empty());
    app.world_mut().resource_mut::<TitleActive>().0 = false;
    app.update();
    assert_eq!(
        requests(&mut app),
        [
            AudioRequest::StopBgm,
            BgmTrack {
                name: "House".into(),
                volume: 0.25,
                speed: 1.1,
                fade_in: 0.0
            }
            .replay()
        ]
    );
    app.update();
    assert!(requests(&mut app).is_empty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_legacy_save_without_music_uses_the_map_default_and_does_not_rewrite_the_file() {
    let path = temp_slot("legacy_music");
    let original = "(format_version:1,map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)";
    std::fs::write(&path, original).unwrap();
    let mut app = music_app(path.clone());
    app.update();
    requests(&mut app);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(requests(&mut app), [AudioRequest::StopBgm]);
    arrive(&mut app, 2);
    assert_eq!(
        requests(&mut app),
        [AudioRequest::Bgm {
            name: "MapDefault".into(),
            volume: 1.0,
            speed: 1.0,
            fade_in: 0.0
        }]
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn memorized_music_survives_the_file_round_trip_separately_from_the_current_track() {
    use bevy::ecs::system::RunSystemOnce;
    let path = temp_slot("memorized_music");
    let mut app = music_app(path.clone());
    let current = BgmTrack {
        name: "House".into(),
        volume: 0.25,
        speed: 1.1,
        fade_in: 0.0,
    };
    let remembered = BgmTrack {
        name: "Elven".into(),
        volume: 0.4,
        speed: 0.8,
        fade_in: 0.7,
    };
    app.insert_resource(CurrentBgm::with_track("House", 0.25, 1.1));
    saved::prepare(
        app.world_mut(),
        2,
        Some(saved::MusicState {
            current: Some(current.clone()),
            memorized: Some(remembered.clone()),
        }),
    );
    arrive(&mut app, 2);
    requests(&mut app);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    let state = read_save(&path).unwrap().music.unwrap();
    assert_eq!(state.current, Some(current.clone()));
    assert_eq!(state.memorized, Some(remembered.clone()));
    app.insert_resource(crate::audio::MemorizedBgm::default());
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    arrive(&mut app, 2);
    assert_eq!(
        requests(&mut app),
        [AudioRequest::StopBgm, current.replay()]
    );
    let state = app
        .world_mut()
        .run_system_once(|capture: saved::Capture| capture.snapshot())
        .unwrap()
        .unwrap();
    assert_eq!(state.memorized, Some(remembered));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_saved_audio_is_rejected_without_changing_the_session_or_file() {
    let path = temp_slot("invalid_music");
    let mut app = music_app(path.clone());
    app.insert_resource(CurrentBgm::with_track("House", 0.25, 1.1));
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    requests(&mut app);
    let mut game = read_save(&path).unwrap();
    game.music
        .as_mut()
        .unwrap()
        .current
        .as_mut()
        .unwrap()
        .volume = f32::NAN;
    write_save(&path, &game).unwrap();
    let original = std::fs::read_to_string(&path).unwrap();
    app.world_mut().resource_mut::<Switches>().set(99, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert!(app.world().resource::<Switches>().get(99));
    assert!(app.world().get_resource::<saved::Pending>().is_none());
    assert!(requests(&mut app).is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_same_frame_map_transfer_does_not_replay_the_default_music_after_the_restore() {
    #[derive(Resource)]
    struct Arrival(bool);
    let path = temp_slot("music_transfer_order");
    let mut app = music_app(path.clone());
    app.insert_resource(CurrentBgm::with_track("House", 0.25, 1.1));
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    requests(&mut app);
    app.insert_resource(Arrival(true)).add_systems(
        Update,
        (|mut arrival: ResMut<Arrival>,
          mut map: ResMut<MapData>,
          mut changed: MessageWriter<crate::world::MapChanged>| {
            if std::mem::take(&mut arrival.0) {
                map.set_changed();
                changed.write(crate::world::MapChanged);
            }
        })
        .in_set(crate::teleport::MapTransfer)
        .after(save_or_load),
    );
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(
        requests(&mut app),
        [
            AudioRequest::StopBgm,
            BgmTrack {
                name: "House".into(),
                volume: 0.25,
                speed: 1.1,
                fade_in: 0.0
            }
            .replay()
        ]
    );
    app.update();
    assert!(requests(&mut app).is_empty());
    std::fs::remove_file(path).unwrap();
}

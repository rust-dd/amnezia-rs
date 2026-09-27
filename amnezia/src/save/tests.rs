use super::*;

mod actor_state;
mod camera;
mod fixture;
mod frame_clock;
mod gameplay;
mod identities;
pub(crate) use fixture::save_resources;
mod message;
mod music;
mod panorama;
mod slots;
mod timestamps;

#[test]
fn save_game_ron_round_trip() {
    let game = SaveGame {
        saved_at: Some(1234567890),
        foreground: None,
        vehicle_motion: None,
        hero_motion: None,
        map_events: Vec::new(),
        map_animation: default(),
        screen: None,
        pictures: Vec::new(),
        camera: None,
        message: default(),
        music: None,
        format_version: SAVE_FORMAT_VERSION,
        game_frames: default(),
        scene_frame: Some(0),
        transitions: default(),
        map_id: 5,
        x: 12,
        y: 7,
        dir: 2,
        switches: vec![(1, true), (3, false), (10, true)],
        variables: vec![(2, -4), (5, 100)],
        party: vec![1, 3],
        items: vec![(181, 2), (200, 1)],
        gold: 250,
        progression: vec![(1, 500), (3, 20)],
        vitals: vec![(1, (40, 12)), (3, (30, 0))],
        conditions: vec![(1, vec![2])],
        field_steps: 37,
        hero_name: "Kettu".into(),
        charset: "Poses2".into(),
        charset_index: 4,
        hero_hidden: false,
        tone: (70, 60, 70, 100),
        weather: 2,
        weather_strength: 5,
        equipment: vec![(1, [10, 0, 5, 0, 0]), (3, [7, 0, 0, 0, 0])],
        learned_skills: vec![(1, vec![3, 4]), (3, vec![16])],
        playtime: 3661,
        timer_remaining: 45.5,
        timer_running: true,
        timer_visible: true,
        timer_in_battle: true,
        vehicles: default(),
        system_bgm: default(),
        panorama: None,
        appearance: default(),
        menu_access: None,
        save_access: false,
    };
    let ron = ron::ser::to_string_pretty(&game, PrettyConfig::default()).unwrap();
    let decoded = ron::from_str::<SaveGame>(&ron).unwrap();
    assert_eq!(game, decoded);
}

/// A unique temp slot path per test, so file-touching tests never race on a
/// shared file (cargo runs them in parallel) and never touch the real save.
pub(super) fn temp_slot(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("amnezia_{tag}_{}.ron", std::process::id()))
}

/// A minimal but complete resource set for driving [`save_or_load`] headlessly,
/// with the save slot pointed at `location` so the real developer save is never
/// read or written.
fn save_app(location: PathBuf) -> App {
    let mut app = save_resources(location);
    app.add_systems(Update, save_or_load);
    app
}

#[test]
fn slot_exists_tracks_the_file() {
    let path = temp_slot("exists");
    let _ = std::fs::remove_file(&path);
    assert!(!slot_exists(&path));
    std::fs::write(&path, "x").unwrap();
    assert!(slot_exists(&path));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_continue_does_not_mutate_the_current_session() {
    let path = temp_slot("corrupt");
    std::fs::write(&path, "broken save").unwrap();
    let mut app = save_app(path.clone());
    app.init_resource::<RunningEvent>();
    app.world_mut().resource_mut::<Switches>().set(8, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert!(app.world().resource::<Switches>().get(8));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn resolved_save_path_is_absolute_and_cwd_independent() {
    assert!(
        save_path().is_absolute(),
        "the save path must be absolute so it is independent of the CWD"
    );
    assert!(save_dir().is_absolute());
}

#[test]
fn resolve_lets_event_save_and_menu_load_bypass_the_running_gate() {
    assert_eq!(resolve(true, false, false, false, true), Some(Action::Save));
    assert_eq!(resolve(false, true, false, false, true), None);
    assert_eq!(resolve(false, false, true, false, true), Some(Action::Load));
    assert_eq!(resolve(false, false, false, true, true), None);
    assert_eq!(
        resolve(false, true, false, false, false),
        Some(Action::Save)
    );
    assert_eq!(
        resolve(false, false, false, true, false),
        Some(Action::Load)
    );
    assert_eq!(
        resolve(false, false, true, false, false),
        Some(Action::Load)
    );
    assert_eq!(resolve(false, true, false, true, false), Some(Action::Save));
    assert_eq!(resolve(false, false, false, false, false), None);
}

#[test]
fn open_save_menu_saves_while_its_event_is_running() {
    let path = temp_slot("eventsave");
    let _ = std::fs::remove_file(&path);
    let mut app = save_app(path.clone());
    app.insert_resource(MapData::for_test(20, 15));
    let mut running = RunningEvent::default();
    running.start(1, Vec::new());
    assert!(running.active());
    app.insert_resource(running);
    app.world_mut().spawn(Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();

    let saved = path.exists();
    let consumed = !app.world().resource::<EventSaveRequest>().0;
    let _ = std::fs::remove_file(&path);
    assert!(
        saved,
        "an OpenSaveMenu save must be written even while its event runs"
    );
    assert!(
        consumed,
        "the event-save request must be consumed once saved"
    );
}

#[test]
fn continue_load_targets_the_saved_map_even_when_an_autostart_is_pending() {
    let path = temp_slot("continue");
    let game = SaveGame {
        saved_at: None,
        foreground: None,
        vehicle_motion: None,
        hero_motion: None,
        map_events: Vec::new(),
        map_animation: default(),
        screen: None,
        pictures: Vec::new(),
        camera: None,
        message: default(),
        format_version: 0,
        music: None,
        game_frames: default(),
        scene_frame: None,
        transitions: default(),
        map_id: 2,
        x: 16,
        y: 6,
        dir: 4,
        switches: vec![(8, true)],
        variables: vec![(3, 42)],
        party: vec![1, 3],
        items: vec![(181, 2)],
        gold: 250,
        progression: vec![(1, 500)],
        vitals: vec![(1, (40, 12))],
        conditions: vec![],
        field_steps: 0,
        hero_name: String::new(),
        charset: String::new(),
        charset_index: 0,
        hero_hidden: false,
        tone: (100, 100, 100, 100),
        weather: 0,
        weather_strength: 0,
        equipment: vec![],
        learned_skills: vec![(1, vec![3, 4])],
        playtime: 0,
        timer_remaining: 0.0,
        timer_running: false,
        timer_visible: false,
        timer_in_battle: false,
        vehicles: default(),
        system_bgm: default(),
        panorama: None,
        appearance: default(),
        menu_access: None,
        save_access: false,
    };
    write_save(&path, &game).unwrap();

    let mut app = save_app(path.clone());
    let mut running = RunningEvent::default();
    running.start(1, Vec::new());
    assert!(running.active());
    app.insert_resource(running);
    app.world_mut().spawn(Player {
        tile_x: 0,
        tile_y: 0,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();

    let world = app.world();
    assert_eq!(
        world.resource::<PendingTeleport>().0,
        Some((2, 16, 6)),
        "Continue must teleport to the SAVED map/tile, not the start map"
    );
    assert!(
        world.resource::<Switches>().get(8),
        "restored switches must be applied"
    );
    assert_eq!(world.resource::<Variables>().get(3), 42);
    assert_eq!(world.resource::<Party>().snapshot(), vec![1, 3]);
    assert!(!world.resource::<RunningEvent>().active());
    assert_eq!(
        world.resource::<Progression>().skill_entries(),
        vec![(1, vec![3, 4])]
    );
    assert!(
        !world.resource::<LoadRequest>().0,
        "the Continue request must be consumed"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_restores_name_charset_and_screen_state() {
    let path = temp_slot("scene");
    let game = SaveGame {
        saved_at: None,
        foreground: None,
        vehicle_motion: None,
        hero_motion: None,
        map_events: Vec::new(),
        map_animation: default(),
        screen: None,
        pictures: Vec::new(),
        camera: None,
        message: default(),
        format_version: 0,
        music: None,
        game_frames: default(),
        scene_frame: None,
        transitions: default(),
        map_id: 2,
        x: 16,
        y: 6,
        dir: 4,
        switches: vec![],
        variables: vec![],
        party: vec![1],
        items: vec![],
        gold: 0,
        progression: vec![],
        vitals: vec![],
        conditions: vec![],
        field_steps: 0,
        hero_name: "Kettu".into(),
        charset: "Poses2".into(),
        charset_index: 4,
        hero_hidden: false,
        tone: (70, 60, 70, 100),
        weather: 2,
        weather_strength: 8,
        equipment: vec![],
        learned_skills: vec![],
        playtime: 0,
        timer_remaining: 724.5,
        timer_running: true,
        timer_visible: true,
        timer_in_battle: true,
        vehicles: default(),
        system_bgm: default(),
        panorama: None,
        appearance: default(),
        menu_access: None,
        save_access: false,
    };
    write_save(&path, &game).unwrap();

    let mut app = save_app(path.clone());
    app.insert_resource(RunningEvent::default());
    let hero = app
        .world_mut()
        .spawn(Player {
            tile_x: 0,
            tile_y: 0,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        })
        .id();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();

    let world = app.world();
    assert_eq!(
        world.resource::<HeroName>().0,
        "Kettu",
        "the saved hero name must be restored"
    );
    let player = world.entity(hero).get::<Player>().unwrap();
    assert_eq!(
        player.charset, "Poses2",
        "the saved costume must be restored"
    );
    assert_eq!(player.index, 4);
    assert_eq!(
        *world.resource::<Weather>(),
        Weather::Snow,
        "the saved weather type must be restored"
    );
    assert_eq!(world.resource::<WeatherStrength>().0, 8);
    let clock = world.resource::<GameClock>();
    assert_eq!(clock.remaining, 724.5);
    assert!(clock.running && clock.visible && clock.in_battle);
    assert!(!clock.expired);
    assert_eq!(
        world.resource::<TintState>().tone().map(|v| v as i32),
        [70, 60, 70, 100],
        "the saved screen tone must be restored"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn old_slot_without_scene_fields_keeps_boot_defaults() {
    let path = temp_slot("legacy");
    let legacy = "(map_id:2,x:16,y:6,dir:4,switches:[(8,true)],\
        variables:[],party:[1],items:[],gold:0,progression:[],vitals:[])";
    std::fs::write(&path, legacy).unwrap();

    let mut app = save_app(path.clone());
    app.insert_resource(RunningEvent::default());
    let hero = app
        .world_mut()
        .spawn(Player {
            tile_x: 0,
            tile_y: 0,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        })
        .id();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();

    let world = app.world();
    assert!(
        world.resource::<Switches>().get(8),
        "an old slot must still restore its core state"
    );
    assert_eq!(
        world.resource::<HeroName>().0,
        "Ron",
        "an old slot must keep the boot hero name, not blank it"
    );
    let player = world.entity(hero).get::<Player>().unwrap();
    assert_eq!(
        player.charset, "Chara1",
        "an old slot must keep the costume"
    );
    assert_eq!(
        world.resource::<TintState>().tone().map(|v| v as i32),
        [100, 100, 100, 100],
        "an old slot must leave the tone neutral, not black the screen"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_and_load_restore_the_runtime_equipment_store() {
    let path = temp_slot("equipment");
    let _ = std::fs::remove_file(&path);
    let mut app = save_app(path.clone());
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.insert_resource(RunningEvent::default());
    app.world_mut().spawn(Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    app.world_mut()
        .resource_mut::<Equipment>()
        .load(vec![(1, [7, 0, 3, 0, 0])]);
    app.world_mut().resource_mut::<SaveRequest>().0 = true;
    app.update();
    assert!(path.exists(), "the save was written");

    app.world_mut().resource_mut::<Equipment>().load(vec![]);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(
        app.world().resource::<Equipment>().entries(),
        vec![(1, [7, 0, 3, 0, 0])],
        "the load restores the saved equipment store"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_round_trips_to_the_resolved_path_and_is_found_after_restart() {
    let path = temp_slot("roundtrip");
    let _ = std::fs::remove_file(&path);
    let game = SaveGame {
        saved_at: None,
        foreground: None,
        vehicle_motion: None,
        hero_motion: None,
        map_events: Vec::new(),
        map_animation: default(),
        screen: None,
        pictures: Vec::new(),
        camera: None,
        message: default(),
        format_version: 0,
        music: None,
        game_frames: default(),
        scene_frame: None,
        transitions: default(),
        map_id: 2,
        x: 16,
        y: 6,
        dir: 2,
        switches: vec![(8, true)],
        variables: vec![],
        party: vec![1],
        items: vec![],
        gold: 0,
        progression: vec![],
        vitals: vec![],
        conditions: vec![],
        field_steps: 0,
        hero_name: String::new(),
        charset: String::new(),
        charset_index: 0,
        hero_hidden: false,
        tone: (100, 100, 100, 100),
        weather: 0,
        weather_strength: 0,
        equipment: vec![],
        learned_skills: vec![],
        playtime: 0,
        timer_remaining: 0.0,
        timer_running: false,
        timer_visible: false,
        timer_in_battle: false,
        vehicles: default(),
        system_bgm: default(),
        panorama: None,
        appearance: default(),
        menu_access: None,
        save_access: false,
    };
    write_save(&path, &game).unwrap();
    assert!(
        slot_exists(&path),
        "the written slot must be found on re-read"
    );
    assert_eq!(read_save(&path), Some(game));
    let _ = std::fs::remove_file(&path);
}

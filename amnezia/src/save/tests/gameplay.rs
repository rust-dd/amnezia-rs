use super::*;
use crate::gamedata::{GameData, GameDataPlugin};

fn load_case(tag: &str, version: u32, fields: &str) -> (App, PathBuf, String) {
    let path = temp_slot(tag);
    let original =
        format!("(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],party:[1],{fields})");
    std::fs::write(&path, &original).unwrap();
    let mut app = save_app(path.clone());
    app.add_plugins(GameDataPlugin)
        .init_resource::<RunningEvent>();
    app.world_mut().resource_mut::<Switches>().set(888, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    (app, path, original)
}

fn unchanged_file(path: PathBuf, original: String) {
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_live_numeric_state_cannot_overwrite_an_existing_slot() {
    let (mut app, path, original) = load_case(
        "invalid_numeric_save",
        SAVE_FORMAT_VERSION,
        "gold:0,items:[],variables:[]",
    );
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
    app.world_mut().resource_mut::<Vitals>().set(1, 2301, 2302);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    assert!(!app.world().resource::<EventSaveRequest>().0);
    assert_eq!(
        app.world().resource::<Vitals>().get_stored(1),
        Some((2301, 2302))
    );
    unchanged_file(path, original);
}

#[test]
fn current_saves_accept_every_original_actors_level_boundaries_unchanged() {
    let (app, path, original) = load_case(
        "original_numeric_boundaries",
        SAVE_FORMAT_VERSION,
        "gold:999999,items:[(181,99)],variables:[(1,-999999),(2,999999)],timer_remaining:60.983333",
    );
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    let data = app.world().resource::<GameData>();
    let mut game = read_save(&path).unwrap();
    for actor in &data.actors {
        for level in 1..=actor.max_level {
            game.party = vec![actor.id];
            game.progression = vec![(actor.id, crate::progression::exp_for_level(level, actor))];
            for fraction in [0, 1] {
                let hp = actor.curves.max_hp[(level - 1) as usize] as i32 * fraction;
                let sp = actor.curves.max_sp[(level - 1) as usize] as i32 * fraction;
                game.vitals = vec![(actor.id, (hp, sp))];
                let before = ron::to_string(&game).unwrap();
                assert!(
                    numeric::prepare(&mut game, Some(data)),
                    "actor {} level {level}",
                    actor.id
                );
                assert_eq!(ron::to_string(&game).unwrap(), before);
            }
        }
    }
    unchanged_file(path, original);
}

#[test]
fn legacy_saved_scalar_values_obey_the_gameplay_limits() {
    for version in 0..=13 {
        let (app, path, original) = load_case(
            &format!("legacy_scalar_limits_{version}"),
            version,
            "gold:2147483647,items:[(181,4294967295)],variables:[(1,-2147483648),(2,2147483647)],progression:[(1,4294967295)],timer_remaining:-3.5",
        );
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        let inventory = app.world().resource::<Inventory>();
        assert_eq!(inventory.gold(), 999_999);
        assert_eq!(inventory.count(181), 99);
        let variables = app.world().resource::<Variables>();
        assert_eq!(variables.get(1), -999_999);
        assert_eq!(variables.get(2), 999_999);
        let ron = app.world().resource::<GameData>().actor(1).unwrap();
        assert_eq!(app.world().resource::<Progression>().total(ron), 999_999);
        assert_eq!(app.world().resource::<GameClock>().remaining, 0.0);
        unchanged_file(path, original);
    }
}

#[test]
fn invalid_current_numeric_values_leave_the_live_session_untouched() {
    for (index, fields) in [
        "gold:-1,items:[],variables:[]",
        "gold:1000000,items:[],variables:[]",
        "gold:0,items:[(181,100)],variables:[]",
        "gold:0,items:[],variables:[(1,-1000000)]",
        "gold:0,items:[],variables:[],progression:[(1,1000000)]",
        "gold:0,items:[],variables:[],vitals:[(1,(64,37))]",
        "gold:0,items:[],variables:[],vitals:[(1,(63,38))]",
        "gold:0,items:[],variables:[],vitals:[(1,(-1,0))]",
        "gold:0,items:[],variables:[],timer_remaining:-0.01",
    ]
    .into_iter()
    .enumerate()
    {
        let (app, path, original) = load_case(
            &format!("current_numeric_limits_{index}"),
            SAVE_FORMAT_VERSION,
            fields,
        );
        assert_eq!(
            app.world().resource::<LoadOutcome>().0,
            Some(false),
            "{fields}"
        );
        assert!(app.world().resource::<Switches>().get(888));
        assert!(app.world().resource::<PendingTeleport>().0.is_none());
        unchanged_file(path, original);
    }
}

#[test]
fn legacy_saved_ron_vitals_are_clamped_to_his_actual_level() {
    for version in 0..=13 {
        for (experience, expected) in [
            ("[]", (63, 37)),
            ("[(1,0)]", (59, 34)),
            ("[(1,4294967295)]", (999, 450)),
        ] {
            let (app, path, original) = load_case(
                &format!("legacy_ron_vitals_{version}"),
                version,
                &format!(
                    "gold:0,items:[],variables:[],progression:{experience},vitals:[(1,(2301,2302)),(2,(-1,-2))]"
                ),
            );
            assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
            let vitals = app.world().resource::<Vitals>();
            assert_eq!(vitals.get_stored(1), Some(expected), "version {version}");
            assert_eq!(vitals.get_stored(2), Some((0, 0)));
            unchanged_file(path, original);
        }
    }
}

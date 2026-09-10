use super::*;
use crate::appearance::{ActorGraphics, Appearance, AppearancePlugin};

fn actor_app(path: PathBuf) -> (App, Entity) {
    let mut app = save_app(path);
    app.add_plugins((crate::gamedata::GameDataPlugin, AppearancePlugin))
        .init_resource::<RunningEvent>()
        .configure_sets(Update, ActorGraphics.after(save_or_load));
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    let hero = app
        .world_mut()
        .spawn(Player {
            tile_x: 3,
            tile_y: 4,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        })
        .id();
    app.update();
    (app, hero)
}

#[test]
fn current_load_resets_temporary_pose_but_retains_saved_actor_costume() {
    for costume in [false, true] {
        let path = temp_slot(&format!("actor_costume_{costume}"));
        let (mut app, hero) = actor_app(path.clone());
        if costume {
            app.world_mut()
                .resource_mut::<Appearance>()
                .set(1, "Chara4".into(), 3);
        }
        app.update();
        {
            let mut player = app.world_mut().get_mut::<Player>(hero).unwrap();
            player.charset = "Poses2".into();
            player.index = 4;
        }
        app.world_mut().resource_mut::<HeroName>().0 = "Áron".into();
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        assert_eq!(
            read_save(&path).unwrap().format_version,
            SAVE_FORMAT_VERSION
        );
        app.insert_resource(Appearance::default());
        app.world_mut().resource_mut::<HeroName>().0 = "Ron".into();
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        let expected = if costume {
            ("Chara4", 3)
        } else {
            ("Chara1", 0)
        };
        let player = app.world().get::<Player>(hero).unwrap();
        assert_eq!((player.charset.as_str(), player.index), expected);
        assert_eq!(
            app.world().resource::<Appearance>().get(1),
            costume.then_some(("Chara4", 3))
        );
        assert_eq!(app.world().resource::<HeroName>().0, "Áron");
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn explicit_empty_name_survives_a_current_save_round_trip() {
    let path = temp_slot("empty_actor_name");
    let (mut app, _) = actor_app(path.clone());
    app.world_mut().resource_mut::<HeroName>().0.clear();
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    app.world_mut().resource_mut::<HeroName>().0 = "Ron".into();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert!(app.world().resource::<HeroName>().0.is_empty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_graphics_are_migrated_without_overwriting_explicit_actor_overrides() {
    for costume in [false, true] {
        let path = temp_slot(&format!("legacy_actor_costume_{costume}"));
        let mut appearance = Appearance::default();
        if costume {
            appearance.set(1, "Chara4".into(), 3);
        }
        let original = format!(
            "(map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1,2],items:[],gold:0,charset:\"Poses2\",charset_index:4,appearance:{})",
            ron::to_string(&appearance).unwrap()
        );
        std::fs::write(&path, &original).unwrap();
        let (mut app, hero) = actor_app(path.clone());
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        let expected = if costume {
            ("Chara4", 3)
        } else {
            ("Poses2", 4)
        };
        assert_eq!(app.world().resource::<Appearance>().get(1), Some(expected));
        for roster in [vec![2, 1], vec![1, 2]] {
            let lead = roster[0];
            app.world_mut().resource_mut::<Party>().restore(roster);
            app.update();
            let player = app.world().get::<Player>(hero).unwrap();
            assert_eq!(
                (player.charset.as_str(), player.index),
                if lead == 1 { expected } else { ("Chara1", 1) }
            );
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn unsupported_save_formats_leave_the_current_session_and_file_untouched() {
    let path = temp_slot("future_save_format");
    let original = format!(
        "(format_version:{},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[2],items:[],gold:0)",
        SAVE_FORMAT_VERSION + 1
    );
    std::fs::write(&path, &original).unwrap();
    let (mut app, hero) = actor_app(path.clone());
    app.world_mut().resource_mut::<Switches>().set(4, true);
    app.world_mut().resource_mut::<HeroName>().0 = "Áron".into();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert!(app.world().resource::<Switches>().get(4));
    assert_eq!(app.world().resource::<Party>().snapshot(), [1]);
    assert_eq!(app.world().resource::<HeroName>().0, "Áron");
    assert_eq!(app.world().get::<Player>(hero).unwrap().charset, "Chara1");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

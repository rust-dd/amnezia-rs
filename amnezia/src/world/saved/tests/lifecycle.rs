use super::*;

#[test]
fn restoration_waits_for_the_matching_rebuild_and_only_applies_once() {
    let (mut app, _) = app("arrival");
    let mut expected = snapshot(app.world_mut());
    expected[0].character.tile_x = 7;
    expected[0].character.tile_y = 8;
    prepare(app.world_mut(), 3, expected.clone());
    app.world_mut().write_message(MapChanged);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    assert_ne!(snapshot(app.world_mut()), expected);
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    assert_ne!(snapshot(app.world_mut()), expected);
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(!app.world().contains_resource::<Pending>());
    assert_eq!(snapshot(app.world_mut()), expected);
    let entity = npc(app.world_mut());
    app.world_mut()
        .get_mut::<EventSprite>(entity)
        .unwrap()
        .tile_x = 9;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().tile_x, 9);
}

#[test]
fn legacy_and_empty_snapshots_cancel_pending_npc_restoration_without_rewriting_the_file() {
    for version in 0..=9 {
        let (mut app, path) = app(&format!("legacy-{version}"));
        let previous = snapshot(app.world_mut());
        prepare(app.world_mut(), 3, previous.clone());
        let original = format!(
            "(format_version:{version},map_id:3,x:10,y:10,dir:2,switches:[],variables:[],party:[1],items:[],gold:0{})",
            if version == 9 { ",map_events:[]" } else { "" }
        );
        std::fs::write(&path, &original).unwrap();
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert!(!app.world().contains_resource::<Pending>());
        assert_eq!(snapshot(app.world_mut()), previous);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_npc_snapshots_leave_the_live_session_and_save_untouched() {
    for case in 0..7 {
        let (mut app, path) = app(&format!("invalid-{case}"));
        let expected = snapshot(app.world_mut());
        let mut bad = expected.clone();
        match case {
            0 => bad[0].character.dir = 4,
            1 => bad[0].character.frame = 4,
            2 => bad[0].character.layer = 3,
            3 => {
                bad[0].character.charset = "Chara1".into();
                bad[0].character.index = 8;
            }
            4 => bad[0].character.id = u32::MAX,
            5 => bad.push(bad[0].clone()),
            _ => {
                let mut value = ron::to_string(&bad[0]).unwrap();
                let old = ron::to_string(&bad[0].page).unwrap();
                value = value.replace(&format!("page:{old}"), "page:(Some(999))");
                bad[0] = ron::from_str(&value).unwrap();
            }
        }
        let original = format!(
            "(format_version:9,map_id:3,x:10,y:10,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,map_events:{})",
            ron::to_string(&bad).unwrap()
        );
        std::fs::write(&path, &original).unwrap();
        app.world_mut().resource_mut::<Switches>().set(9000, true);
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(
            app.world().resource::<LoadOutcome>().0,
            Some(false),
            "case {case}"
        );
        assert_eq!(snapshot(app.world_mut()), expected);
        assert!(app.world().resource::<Switches>().get(9000));
        assert!(!app.world().contains_resource::<Pending>());
        assert!(
            app.world()
                .resource::<crate::teleport::PendingTeleport>()
                .0
                .is_none()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn clearing_the_session_discards_an_unapplied_character_snapshot() {
    let (mut app, _) = app("session");
    let saved = snapshot(app.world_mut());
    prepare(app.world_mut(), 3, saved);
    assert!(app.world().contains_resource::<Pending>());
    crate::session::clear_transient(app.world_mut());
    assert!(!app.world().contains_resource::<Pending>());
}

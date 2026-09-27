use super::*;

#[test]
fn invalid_saved_casts_leave_the_live_session_and_file_untouched() {
    for (tag, from, to) in [
        ("id", "id: 62,", "id: 9999,"),
        ("frame", "elapsed: 13,", "elapsed: 4294967295,"),
        ("target", "target: Hero,", "target: Event(999999),"),
        ("kind", "target: Hero,", "target: Missing,"),
    ] {
        let (mut app, path) = app(tag);
        play(&mut app);
        save(&mut app);
        let original = std::fs::read_to_string(&path).unwrap();
        let invalid = original.replace(from, to);
        assert_ne!(original, invalid);
        std::fs::write(&path, &invalid).unwrap();
        let before = saved::snapshot(app.world_mut());
        app.world_mut()
            .resource_mut::<crate::state::Switches>()
            .set(99, true);
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(false)
        );
        assert_eq!(saved::snapshot(app.world_mut()), before);
        assert!(app.world().resource::<crate::state::Switches>().get(99));
        assert!(!app.world().contains_resource::<saved::Pending>());
        assert!(
            app.world()
                .resource::<crate::teleport::PendingTeleport>()
                .0
                .is_none()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn legacy_and_explicitly_empty_saves_clear_old_animations_without_rewriting() {
    for version in 0..=8 {
        let (mut app, path) = app(&format!("legacy_{version}"));
        play(&mut app);
        let extra = if version == 8 {
            ",map_animation:(cast:None,screen_flash:None)"
        } else {
            ""
        };
        let contents = format!(
            "(format_version:{version},map_id:3,x:8,y:7,dir:2,switches:[],variables:[],party:[1],items:[],gold:0{extra})"
        );
        std::fs::write(&path, &contents).unwrap();
        reload(&mut app);
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(true)
        );
        assert_eq!(saved::snapshot(app.world_mut()), default());
        assert_eq!(app.world().resource::<ActiveAnimations>().total, 0);
        assert!(!app.world().contains_resource::<saved::Pending>());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn a_pending_cast_waits_for_the_correct_rebuilt_map_and_is_applied_once() {
    let (mut app, path) = app("arrival");
    play(&mut app);
    let expected = saved::snapshot(app.world_mut());
    save(&mut app);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(crate::world::MapEffectsReset);
    app.update();
    assert!(app.world().contains_resource::<saved::Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(crate::world::MapEffectsReset);
    app.update();
    assert_eq!(saved::snapshot(app.world_mut()), expected);
    assert!(!app.world().contains_resource::<saved::Pending>());
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().write_message(crate::world::MapEffectsReset);
    app.update();
    assert_eq!(saved::snapshot(app.world_mut()), default());
    std::fs::remove_file(path).unwrap();
}

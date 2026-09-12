use super::*;
use crate::world::saved::hero::{HeroState, Pending, prepare, snapshot};

#[test]
fn saved_hero_motion_waits_for_the_matching_rebuild_and_is_applied_only_once() {
    let (mut app, _, entity) = hero_app("hero-arrival");
    start_motion(app.world_mut(), true);
    let expected = snapshot(app.world_mut()).unwrap();
    prepare(app.world_mut(), 3, Some(expected.clone()));
    app.world_mut()
        .entity_mut(entity)
        .insert((MoveQueue::default(), RouteStepper::default()));
    app.world_mut().write_message(MapChanged);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    assert_ne!(snapshot(app.world_mut()).unwrap(), expected);
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert!(!app.world().contains_resource::<Pending>());
    assert_eq!(snapshot(app.world_mut()).unwrap(), expected);
    app.world_mut().get_mut::<Player>(entity).unwrap().frame = 1;
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert_eq!(app.world().get::<Player>(entity).unwrap().frame, 1);
}

#[test]
fn legacy_saves_drop_stale_hero_motion_without_rewriting_the_file() {
    for version in 0..=10 {
        let (mut app, path, entity) = hero_app(&format!("hero-legacy-{version}"));
        start_motion(app.world_mut(), false);
        let previous = snapshot(app.world_mut()).unwrap();
        prepare(app.world_mut(), 3, Some(previous));
        let original = format!(
            "(format_version:{version},map_id:3,x:10,y:10,dir:2,switches:[],variables:[],party:[1],items:[],gold:0{})",
            if version == 10 {
                ",hero_motion:None"
            } else {
                ""
            }
        );
        std::fs::write(&path, &original).unwrap();
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert!(!app.world().contains_resource::<Pending>());
        assert!(!app.world().get::<MoveQueue>(entity).unwrap().busy());
        assert!(!app.world().get::<RouteStepper>(entity).unwrap().pending());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_saved_hero_state_does_not_mutate_the_session_or_slot() {
    for case in 0..6 {
        let (mut app, path, _) = hero_app(&format!("hero-invalid-{case}"));
        let expected = snapshot(app.world_mut()).unwrap();
        let encoded = ron::to_string(&expected).unwrap();
        let bad = match case {
            0 => HeroState {
                frame: 4,
                ..expected.clone()
            },
            1 => ron::from_str::<HeroState>(
                &encoded.replace("step_secs:0.13333334", "step_secs:0.0"),
            )
            .unwrap(),
            2 => ron::from_str::<HeroState>(&encoded.replace("speed:4", "speed:0")).unwrap(),
            3 => ron::from_str::<HeroState>(&encoded.replace("transparency:0", "transparency:8"))
                .unwrap(),
            4 => ron::from_str::<HeroState>(&encoded.replace("fraction:0.0", "fraction:NaN"))
                .unwrap(),
            _ => expected.clone(),
        };
        assert!(
            case == 5 || !bad.valid(),
            "case {case}: replacement must invalidate the state"
        );
        let original = format!(
            "(format_version:10,map_id:3,x:10,y:10,dir:{},switches:[],variables:[],party:[1],items:[],gold:0,hero_motion:Some({}))",
            if case == 5 { 4 } else { 2 },
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
        assert_eq!(snapshot(app.world_mut()).unwrap(), expected);
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
fn clearing_the_session_discards_unapplied_hero_motion() {
    let (mut app, _, _) = hero_app("hero-clear");
    let state = snapshot(app.world_mut());
    prepare(app.world_mut(), 3, state);
    assert!(app.world().contains_resource::<Pending>());
    crate::session::clear_transient(app.world_mut());
    assert!(!app.world().contains_resource::<Pending>());
}

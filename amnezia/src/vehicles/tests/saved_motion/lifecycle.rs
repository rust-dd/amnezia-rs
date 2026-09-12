use super::*;
use crate::vehicles::saved::{Pending, prepare};

mod validation;

#[test]
fn motion_waits_for_a_matching_rebuild_and_survives_the_first_rider_sync_only_once() {
    let (mut app, _) = app("arrival");
    app.world_mut().resource_scope(|world, data: Mut<MapData>| {
        let mut vehicles = world.resource_mut::<Vehicles>();
        start(&mut vehicles, &data, 2, &[1, 23, 1]);
        vehicles.save.riding = Some(2);
    });
    let expected = app.world().resource::<Vehicles>().motion_snapshot();
    prepare(app.world_mut(), 13, Some(expected.clone()));
    app.world_mut().resource_mut::<Vehicles>().clear_motion();
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    app.world_mut().write_message(crate::world::MapChanged);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    assert_ne!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        expected
    );
    app.world_mut().write_message(crate::world::MapRebuilt);
    app.update();
    assert!(app.world().contains_resource::<Pending>());
    assert_ne!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        expected
    );
    app.world_mut().resource_mut::<MapData>().map_id = 13;
    app.world_mut().write_message(crate::world::MapRebuilt);
    app.update();
    assert!(!app.world().contains_resource::<Pending>());
    assert_eq!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        expected
    );
    app.world_mut().resource_mut::<Vehicles>().clear_motion();
    let cleared = app.world().resource::<Vehicles>().motion_snapshot();
    app.world_mut().write_message(crate::world::MapRebuilt);
    app.update();
    assert_eq!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        cleared
    );
}

#[test]
fn legacy_and_empty_vehicle_motion_cancel_stale_pending_state_without_rewriting_the_slot() {
    for version in 0..=11 {
        let (mut app, path) = app(&format!("legacy-{version}"));
        app.world_mut().resource_scope(|world, data: Mut<MapData>| {
            start(&mut world.resource_mut::<Vehicles>(), &data, 0, &[1, 23, 1]);
        });
        let expected = app.world().resource::<Vehicles>().save.clone();
        let state = app.world().resource::<Vehicles>().motion_snapshot();
        prepare(app.world_mut(), 13, Some(state));
        let original = format!(
            "(format_version:{version},map_id:13,x:20,y:20,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,vehicles:{}{})",
            ron::to_string(&expected).unwrap(),
            if version == 11 {
                ",vehicle_motion:None"
            } else {
                ""
            }
        );
        std::fs::write(&path, &original).unwrap();
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert!(!app.world().contains_resource::<Pending>());
        let vehicles = app.world().resource::<Vehicles>();
        assert_eq!(vehicles.save, expected);
        for motion in &vehicles.motion {
            assert!(!motion.queue.busy() && !motion.route.pending());
            assert_eq!(motion.alpha, 1.0);
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn clearing_a_session_discards_unapplied_vehicle_motion() {
    let (mut app, _) = app("clear");
    let state = app.world().resource::<Vehicles>().motion_snapshot();
    prepare(app.world_mut(), 13, Some(state));
    assert!(app.world().contains_resource::<Pending>());
    crate::session::clear_transient(app.world_mut());
    assert!(!app.world().contains_resource::<Pending>());
}

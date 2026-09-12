use super::*;
use crate::vehicles::saved::State;

fn rejects(app: &mut App, path: &std::path::Path, state: State, base: VehicleSave) {
    assert!(!state.valid(&base));
    let expected = app.world().resource::<Vehicles>().motion_snapshot();
    let previous = app.world().resource::<Vehicles>().save.clone();
    let original = format!(
        "(format_version:11,map_id:13,x:20,y:20,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,vehicles:{},vehicle_motion:Some({}))",
        ron::to_string(&base).unwrap(),
        ron::to_string(&state).unwrap()
    );
    std::fs::write(path, &original).unwrap();
    app.world_mut().resource_mut::<Switches>().set(9000, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert_eq!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        expected
    );
    assert_eq!(app.world().resource::<Vehicles>().save, previous);
    assert!(app.world().resource::<Switches>().get(9000));
    assert!(!app.world().contains_resource::<Pending>());
    assert!(
        app.world()
            .resource::<crate::teleport::PendingTeleport>()
            .0
            .is_none()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_vehicle_motion_numbers_are_rejected_without_mutating_the_session_or_slot() {
    for (case, (from, to)) in [
        ("step_secs:0.13333334", "step_secs:0.0"),
        ("elapsed:0.04", "elapsed:NaN"),
        ("transparency:1", "transparency:8"),
        ("timer:0.0", "timer:inf"),
        ("count:0", "count:24"),
        ("fraction:0.0", "fraction:NaN"),
        ("pixel:Some((-787.2,792.0))", "pixel:Some((NaN,792.0))"),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut app, path) = app(&format!("invalid-motion-{case}"));
        app.world_mut().resource_scope(|world, data: Mut<MapData>| {
            start(&mut world.resource_mut::<Vehicles>(), &data, 0, &[1, 23, 1]);
        });
        let vehicles = app.world().resource::<Vehicles>();
        let encoded = ron::to_string(&vehicles.motion_snapshot()).unwrap();
        assert!(encoded.contains(from), "{case}: {encoded}");
        let state = ron::from_str::<State>(&encoded.replacen(from, to, 1)).unwrap();
        let base = vehicles.save.clone();
        rejects(&mut app, &path, state, base);
    }
}

#[test]
fn invalid_vehicle_pose_rider_and_flight_fields_are_rejected_without_mutation() {
    for case in 0..9 {
        let (mut app, path) = app(&format!("invalid-vehicle-{case}"));
        let vehicles = app.world().resource::<Vehicles>();
        let state = vehicles.motion_snapshot();
        let mut base = vehicles.save.clone();
        assert!(state.valid(&base));
        match case {
            0 => base.vehicles[0].dir = 4,
            1 => base.vehicles[0].frame = 4,
            2 => base.vehicles[0].speed = 0,
            3 => base.vehicles[0].definition.index = 8,
            4 => base.vehicles[0].definition.x = u32::MAX,
            5 => base.riding = Some(3),
            6..=8 => {
                let (from, to) = match case {
                    6 => ("ascent:0", "ascent:257"),
                    7 => ("fraction:0.0", "fraction:NaN"),
                    _ => ("ascent:0,descent:0", "ascent:1,descent:1"),
                };
                let encoded = ron::to_string(&base).unwrap();
                assert!(encoded.contains(from));
                base = ron::from_str::<VehicleSave>(&encoded.replace(from, to)).unwrap();
            }
            _ => unreachable!(),
        }
        rejects(&mut app, &path, state, base);
    }
}

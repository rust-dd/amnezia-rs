use super::*;

#[test]
fn every_vehicle_retains_its_own_stop_count_across_a_real_save_and_load() {
    let (mut app, path) = app("stop-clocks");
    {
        let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
        for (index, motion) in vehicles.motion.iter_mut().enumerate() {
            motion.route.set_stop_maximum(64);
            motion.route.set_stop_count(13 + index as u32);
        }
    }
    let expected = app.world().resource::<Vehicles>().motion_snapshot();
    save_and_load(&mut app);
    assert_eq!(
        app.world().resource::<Vehicles>().motion_snapshot(),
        expected
    );
    for (index, motion) in app.world().resource::<Vehicles>().motion.iter().enumerate() {
        assert_eq!(motion.route.stop_count(), 13 + index as u32);
        assert_eq!(motion.route.stop_maximum(), 64);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn old_vehicle_countdowns_migrate_independently_without_rewriting_the_slot() {
    let (mut app, path) = app("legacy-stop-clocks");
    let vehicles = app.world().resource::<Vehicles>();
    let motion = ron::to_string(&vehicles.motion_snapshot()).unwrap();
    let clock = "stop:Some((count:0,maximum:0)),";
    assert_eq!(motion.matches(clock).count(), 3);
    let mut legacy = motion;
    for seconds in [0.1, 0.25, 0.5] {
        legacy = legacy.replacen(clock, &format!("timer:{seconds},"), 1);
    }
    let original = format!(
        "(format_version:19,map_id:13,x:20,y:20,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,vehicles:{},vehicle_motion:Some({legacy}))",
        ron::to_string(&vehicles.save).unwrap()
    );
    std::fs::write(&path, &original).unwrap();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    let vehicles = app.world().resource::<Vehicles>();
    for (motion, remaining) in vehicles.motion.iter().zip([6, 15, 30]) {
        assert_eq!(motion.route.stop_maximum(), 64);
        assert_eq!(motion.route.stop_count(), 64 - remaining);
        assert!(motion.route.stop_active());
        assert!(!ron::to_string(&motion.route).unwrap().contains("timer:"));
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

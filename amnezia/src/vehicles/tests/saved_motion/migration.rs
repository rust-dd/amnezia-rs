use super::*;

#[test]
fn old_rider_slots_restore_motion_on_the_hero_without_rewriting_the_file() {
    for version in [0, 10, 11, 12, 27] {
        let (mut app, path) = app(&format!("rider-migration-{version}"));
        app.world_mut().resource_scope(|world, data: Mut<MapData>| {
            let mut vehicles = world.resource_mut::<Vehicles>();
            start(&mut vehicles, &data, 2, &[1, 32, 8]);
            vehicles.save.riding = Some(2);
        });
        let vehicles = app.world().resource::<Vehicles>();
        let tile = vehicles.save.vehicles[2].tile();
        let speed = vehicles.save.vehicles[2].speed;
        let expected = vehicles.motion[2].queue.snapshot();
        let base = ron::to_string(&vehicles.save)
            .unwrap()
            .replace("unboarding:false,", "")
            .replace("boarding:false,", "")
            .replace("preboard_speed:4,", "");
        let motion = if version >= 11 {
            format!(
                ",vehicle_motion:Some({})",
                ron::to_string(&vehicles.motion_snapshot()).unwrap()
            )
        } else {
            String::new()
        };
        let original = format!(
            "(format_version:{version},map_id:13,x:{},y:{},dir:2,switches:[],variables:[],party:[1],items:[],gold:0,vehicles:{base}{motion})",
            tile.0, tile.1
        );
        std::fs::write(&path, &original).unwrap();
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        let hero = crate::world::saved::hero::snapshot(app.world_mut()).unwrap();
        assert_eq!(hero.route.speed(), speed);
        assert_eq!(hero.route.pending(), version >= 11);
        assert_eq!(
            hero.motion,
            if version >= 11 {
                expected
            } else {
                MoveQueue::default().snapshot()
            }
        );
        let vehicles = app.world().resource::<Vehicles>();
        assert!(!vehicles.motion[2].route.pending());
        assert_eq!(vehicles.save.preboard_speed, 4);
        assert!(vehicles.aboard());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

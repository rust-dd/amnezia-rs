use super::*;

mod saved_motion;

#[test]
fn vehicle_collision_reads_live_route_through_state() {
    let mut vehicles = Vehicles::default();
    vehicles.set_location(0, 13, 4, 4);
    assert_eq!(
        vehicles.collision_tiles(13).collect::<Vec<_>>(),
        [(0, (4, 4))]
    );
    vehicles.set_route(
        10002,
        crate::world::RouteStepper::from_move_event(&[10002, 8, 0, 0, 36]),
    );
    let motion = &mut vehicles.motion[0];
    drive_route(
        &mut vehicles.save.vehicles[0],
        &mut motion.queue,
        &mut motion.route,
        (0, 0),
        |_, _, _, _, _| true,
    );
    assert_eq!(vehicles.collision_tiles(13).count(), 0);
}

#[test]
fn boarding_and_disembarking_wrap_the_facing_tile() {
    use test_support::{direction, rider_app, ticks, toggle};
    let mut data = crate::world::test_support::water_map(140, 140);
    data.scroll_type = 3;
    let (mut app, hero) = rider_app(data, (0, 2));
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(0, 0, 139, 2);
    direction(&mut app, DIR_LEFT);
    toggle(&mut app);
    assert!(app.world().resource::<Vehicles>().save.boarding);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (139, 2));
    ticks(&mut app, 8);
    direction(&mut app, DIR_RIGHT);
    crate::world::test_support::block_tile(&mut app.world_mut().resource_mut::<MapData>(), (0, 2));
    toggle(&mut app);
    assert!(app.world().resource::<Vehicles>().aboard());
    let mut data = crate::world::test_support::water_map(140, 140);
    data.scroll_type = 3;
    app.insert_resource(data);
    toggle(&mut app);
    assert!(!app.world().resource::<Vehicles>().riding());
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (0, 2));
    assert_eq!(app.world().get::<Player>(hero).unwrap().dir, DIR_RIGHT);
    assert_eq!(
        app.world().resource::<Vehicles>().character(10002),
        Some((139, 2, DIR_LEFT))
    );
}

#[test]
fn airship_boards_on_its_tile_and_lands_at_its_live_position() {
    use test_support::{direction, rider_app, ticks, toggle};
    let mut data = MapData::for_test(100, 110);
    data.map_id = 13;
    let (mut app, hero) = rider_app(data, (54, 100));
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(2, 13, 55, 100);
    toggle(&mut app);
    assert!(!app.world().resource::<Vehicles>().riding());
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(55, 100);
    direction(&mut app, DIR_DOWN);
    toggle(&mut app);
    assert!(app.world().resource::<Vehicles>().aboard());
    ticks(&mut app, 32);
    assert_eq!(
        app.world().resource::<Vehicles>().character(10004),
        Some((55, 100, DIR_LEFT))
    );
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(28, 100);
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(2, 13, 28, 100);
    toggle(&mut app);
    ticks(&mut app, 32);
    assert!(!app.world().resource::<Vehicles>().riding());
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (28, 100));
    assert_eq!(app.world().get::<Player>(hero).unwrap().dir, DIR_DOWN);
    assert_eq!(
        app.world()
            .get::<RouteStepper>(hero)
            .unwrap()
            .direction(app.world().get::<Player>(hero).unwrap()),
        DIR_DOWN
    );
}

#[test]
fn airship_cannot_land_on_a_solid_event() {
    use test_support::{rider_app, ticks, toggle};
    let (mut app, _) = rider_app(MapData::for_test(10, 10), (4, 4));
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(2, 0, 4, 4);
    let mut event =
        crate::world::test_support::event(1, 4, vec![crate::world::test_support::page(vec![])]);
    event.y = 4;
    app.world_mut()
        .resource_mut::<MapEvents>()
        .events
        .push(event);
    toggle(&mut app);
    ticks(&mut app, 32);
    toggle(&mut app);
    ticks(&mut app, 32);
    assert!(app.world().resource::<Vehicles>().riding());
}

#[test]
fn original_airship_move_route_advances_character_and_survives_save() {
    let data = MapData::for_test(100, 110);
    let mut vehicles = Vehicles::default();
    vehicles.set_location(2, 0, 55, 100);
    vehicles.save.riding = Some(2);
    vehicles.set_route(
        10004,
        crate::world::RouteStepper::from_move_event(&[10004, 8, 0, 0, 3, 3, 2]),
    );
    for _ in 0..200 {
        let vehicle = &mut vehicles.save.vehicles[2];
        let motion = &mut vehicles.motion[2];
        drive_route(
            vehicle,
            &mut motion.queue,
            &mut motion.route,
            (55, 100),
            |_, _, _, _, _| true,
        );
        let moving = motion.queue.busy();
        motion.queue.advance(vehicle, &data, 1.0 / 60.0);
        if moving && !motion.queue.busy() {
            motion.route.settle_movement();
        }
        motion.route.advance_stop_clock(moving, true);
    }
    assert!(!vehicles.routes_pending());
    assert_eq!(vehicles.character(10004), Some((53, 101, DIR_DOWN)));
    let saved = ron::to_string(&vehicles.save).unwrap();
    let mut restored = Vehicles::default();
    restored.restore(ron::from_str(&saved).unwrap());
    assert_eq!(restored.save, vehicles.save);
    assert!(restored.riding());
    assert!(!restored.routes_pending());
}

#[test]
fn original_fortress_flight_finishes_before_its_four_second_wait() {
    let data = MapData::for_test(100, 110);
    let mut vehicles = Vehicles::default();
    vehicles.set_location(2, 0, 55, 100);
    let mut route = vec![10004, 8, 0, 0, 29];
    route.extend(std::iter::repeat_n(3, 27));
    vehicles.set_route(10004, crate::world::RouteStepper::from_move_event(&route));
    for _ in 0..240 {
        let vehicle = &mut vehicles.save.vehicles[2];
        let motion = &mut vehicles.motion[2];
        drive_route(
            vehicle,
            &mut motion.queue,
            &mut motion.route,
            (55, 100),
            |_, _, _, _, _| true,
        );
        let moving = motion.queue.busy();
        motion.queue.advance(vehicle, &data, 1.0 / 60.0);
        if moving && !motion.queue.busy() {
            motion.route.settle_movement();
        }
        motion.route.advance_stop_clock(moving, true);
    }
    assert_eq!(vehicles.character(10004), Some((28, 100, DIR_LEFT)));
    assert!(!vehicles.routes_pending());
}

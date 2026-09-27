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
    let mut data = MapData::for_test(140, 140);
    data.scroll_type = 3;
    let mut vehicles = Vehicles::default();
    vehicles.set_location(0, 0, 139, 2);
    assert!(vehicles.toggle(&data, (0, 2, DIR_LEFT), |_, _| false));
    vehicles.motion[0]
        .route
        .set_direction(&mut vehicles.save.vehicles[0], DIR_RIGHT);
    assert!(!vehicles.toggle(&data, (139, 2, DIR_RIGHT), |x, y| (x, y) == (0, 2)));
    assert!(vehicles.toggle(&data, (139, 2, DIR_RIGHT), |_, _| false));
    assert_eq!(
        vehicles.disembark,
        Some(model::DisembarkPose {
            tile: (0, 2),
            direction: DIR_RIGHT,
            facing: DIR_RIGHT
        })
    );
}

#[test]
fn airship_boards_on_its_tile_and_lands_at_its_live_position() {
    let mut vehicles = Vehicles::default();
    let mut data = MapData::for_test(100, 110);
    data.map_id = 13;
    vehicles.set_location(2, 13, 55, 100);
    assert!(!vehicles.toggle(&data, (54, 100, DIR_RIGHT), |_, _| false));
    assert!(vehicles.toggle(&data, (55, 100, DIR_DOWN), |_, _| false));
    for _ in 0..32 {
        vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false);
    }
    assert_eq!(vehicles.character(10004), Some((55, 100, DIR_LEFT)));
    vehicles.set_location(2, 13, 28, 100);
    assert!(vehicles.toggle(&data, (55, 100, DIR_DOWN), |_, _| false));
    for _ in 0..32 {
        vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false);
    }
    assert_eq!(
        vehicles.disembark,
        Some(model::DisembarkPose {
            tile: (28, 100),
            direction: DIR_LEFT,
            facing: DIR_DOWN
        })
    );
    assert!(!vehicles.riding());
}

#[test]
fn airship_cannot_land_on_a_solid_event() {
    let data = MapData::for_test(10, 10);
    let mut vehicles = Vehicles::default();
    vehicles.set_location(2, 0, 4, 4);
    assert!(vehicles.toggle(&data, (4, 4, DIR_DOWN), |_, _| false));
    for _ in 0..32 {
        vehicles.advance_flight(1.0 / 60.0, &data, |_, _| true);
    }
    assert!(vehicles.toggle(&data, (4, 4, DIR_DOWN), |_, _| true));
    for _ in 0..32 {
        vehicles.advance_flight(1.0 / 60.0, &data, |_, _| true);
    }
    assert!(vehicles.riding());
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

use super::*;

fn moving(jumping: bool) -> Vehicles {
    let mut vehicles = Vehicles::default();
    vehicles.set_location(0, 0, 2, 3);
    vehicles.save.vehicles[0].speed = 2;
    vehicles.save.vehicles[0].frame = 3;
    vehicles.set_route(
        10002,
        RouteStepper::from_move_event(&[10002, 4, 1, 0, 1, 32, 9]),
    );
    let motion = &mut vehicles.motion[0];
    motion.alpha = 0.5;
    motion.queue.use_character_motion(2, DIR_RIGHT);
    motion.queue.begin_from(
        &mut vehicles.save.vehicles[0],
        &MapData::for_test(20, 15),
        (2, 3),
        if jumping {
            RouteAction::Jump {
                dx: 2,
                dy: 0,
                face: DIR_RIGHT,
            }
        } else {
            RouteAction::Step {
                dx: 1,
                dy: 0,
                face: DIR_RIGHT,
            }
        },
    );
    vehicles
}

#[test]
fn vehicle_relocation_preserves_routes_speed_animation_and_transparency() {
    let mut vehicles = moving(false);
    let route = vehicles.motion[0].route.clone();
    vehicles.set_location(0, 99, 8, 9);
    assert_eq!(vehicles.motion[0].route, route);
    assert_eq!(vehicles.motion[0].alpha, 0.5);
    assert_eq!(vehicles.save.vehicles[0].speed, 2);
    assert_eq!(vehicles.save.vehicles[0].frame, 3);
    assert!(!vehicles.motion[0].queue.busy());
    assert_eq!(vehicles.motion[0].pixel, None);
}

#[test]
fn vehicle_relocation_keeps_a_zero_remaining_jump_until_its_next_update() {
    let mut vehicles = moving(true);
    vehicles.set_location(0, 99, 8, 9);
    assert!(vehicles.motion[0].queue.jumping());
    let data = MapData::for_test(40, 30);
    let vehicle = &mut vehicles.save.vehicles[0];
    let queue = &mut vehicles.motion[0].queue;
    let point = Vec2::from(data.tile_center(8, 9));
    assert_eq!(queue.render_position(vehicle, &data), point);
    assert_eq!(queue.advance(vehicle, &data, 1.0 / 60.0), Some(point));
    assert!(!queue.busy());
}

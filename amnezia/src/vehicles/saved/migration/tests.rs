use super::*;
use crate::tiles::DIR_RIGHT;
use crate::vehicles::Vehicles;
use crate::world::RouteAction;

#[test]
fn legacy_rider_motion_moves_to_the_hero_once_without_resetting_vehicle_properties() {
    for version in 0..28 {
        let data = MapData::for_test(20, 20);
        let mut vehicles = Vehicles::default();
        vehicles.set_location(2, 0, 5, 5);
        vehicles.save.riding = Some(2);
        vehicles.save.vehicles[2].speed = 2;
        vehicles.set_route(
            10004,
            RouteStepper::from_move_event(&[10004, 8, 0, 0, 1, 32, 7]),
        );
        vehicles.motion[2].queue.use_character_motion(2, DIR_RIGHT);
        vehicles.motion[2].queue.begin_from(
            &mut vehicles.save.vehicles[2],
            &data,
            (5, 5),
            RouteAction::Step {
                dx: 1,
                dy: 0,
                face: DIR_RIGHT,
            },
        );
        vehicles.motion[2]
            .queue
            .advance(&mut vehicles.save.vehicles[2], &data, 1.0 / 60.0);
        let expected = vehicles.motion_snapshot();
        let mut motion = Some(expected.clone());
        let mut hero = Some(HeroState {
            frame: 2,
            motion: MoveQueue::default().snapshot(),
            route: RouteStepper::default().with_speed(3),
        });
        let base = vehicles.save.vehicles.clone();
        migrate_rider(version, &mut vehicles.save, &mut motion, &mut hero);
        assert_eq!(vehicles.save.preboard_speed, 3);
        assert_eq!(vehicles.save.vehicles, base);
        let migrated = hero.as_ref().unwrap();
        assert!(migrated.valid());
        assert_eq!(migrated.frame, 2);
        assert_eq!(migrated.motion, expected.motion[2].queue);
        assert!(migrated.route.pending());
        assert_eq!(migrated.route.speed(), 2);
        assert!(!motion.as_ref().unwrap().motion[2].route.pending());
        assert_eq!(
            motion.as_ref().unwrap().motion[2].queue,
            expected.motion[2].queue
        );
        let hero_before = hero.clone();
        let motion_before = motion.clone();
        migrate_rider(28, &mut vehicles.save, &mut motion, &mut hero);
        assert_eq!(hero, hero_before);
        assert_eq!(motion, motion_before);
    }
}

#[test]
fn legacy_rider_without_vehicle_motion_does_not_invent_a_pending_route() {
    let mut vehicles = crate::vehicles::VehicleSave {
        riding: Some(1),
        ..default()
    };
    vehicles.vehicles[1].speed = 6;
    let mut hero = None;
    migrate_rider(10, &mut vehicles, &mut None, &mut hero);
    let hero = hero.unwrap();
    assert_eq!(vehicles.preboard_speed, 4);
    assert_eq!(hero.route.speed(), 6);
    assert!(!hero.route.pending() && !hero.motion.into_queue().busy());
}

#[test]
fn new_rider_saves_retain_both_owners_and_their_boarding_phase() {
    let mut vehicles = crate::vehicles::VehicleSave {
        riding: Some(0),
        boarding: true,
        preboard_speed: 1,
        ..default()
    };
    let mut hero = Some(HeroState {
        frame: 3,
        motion: MoveQueue::default().snapshot(),
        route: RouteStepper::default().with_speed(1),
    });
    let before = (vehicles.clone(), hero.clone());
    migrate_rider(28, &mut vehicles, &mut None, &mut hero);
    assert_eq!((vehicles, hero), before);
}

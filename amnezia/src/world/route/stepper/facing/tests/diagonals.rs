use super::*;

#[test]
fn a_diagonal_step_retains_its_direction_for_the_following_forward_command() {
    for (direction, dx, dy, facing) in [
        (4, 1, -1, DIR_RIGHT),
        (5, 1, 1, DIR_RIGHT),
        (6, -1, 1, DIR_LEFT),
        (7, -1, -1, DIR_LEFT),
    ] {
        let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, direction, 11]);
        let mut ch = character();
        ch.dir = facing;
        for _ in 0..2 {
            let (action, _) = route
                .advance(&mut ch, (0, 0), &|_, _, _, _, _| true, &mut vec![])
                .unwrap();
            assert_eq!(
                action,
                crate::world::RouteAction::Step {
                    dx,
                    dy,
                    face: facing
                }
            );
            assert_eq!(route.direction(&ch), direction as u32);
        }
    }
}

#[test]
fn restoring_diagonal_facing_keeps_a_compatible_axis_and_reverses_an_incompatible_one() {
    for (direction, vertical, horizontal) in [
        (4, DIR_UP, DIR_RIGHT),
        (5, DIR_DOWN, DIR_RIGHT),
        (6, DIR_DOWN, DIR_LEFT),
        (7, DIR_UP, DIR_LEFT),
    ] {
        for facing in 0..4 {
            let route = RouteStepper {
                direction: Some(direction),
                ..RouteStepper::default()
            };
            let mut ch = character();
            ch.dir = facing;
            route.update_facing(&mut ch);
            let expected = if [vertical, horizontal].contains(&facing) {
                facing
            } else {
                (facing + 2) % 4
            };
            assert_eq!(ch.dir, expected, "direction {direction}, facing {facing}");
            assert_eq!(route.direction(&ch), direction);
        }
    }
}

#[test]
fn saved_diagonal_directions_are_valid_but_diagonal_sprite_locks_are_not() {
    for direction in 0..=8 {
        let route = RouteStepper {
            direction: Some(direction),
            ..RouteStepper::default()
        };
        let mut restored = ron::from_str::<RouteStepper>(&ron::to_string(&route).unwrap()).unwrap();
        assert_eq!(restored.valid(), direction < 8);
        assert_eq!(restored.direction(&character()), direction);
        restored.facing_lock = Some(4);
        assert!(!restored.valid());
    }
}

#[test]
fn forward_keeps_temporary_facing_except_while_retrying_a_blocked_move() {
    for (passable, skippable, expected) in [
        (true, false, DIR_UP),
        (false, true, DIR_UP),
        (false, false, DIR_RIGHT),
    ] {
        let mut route = RouteStepper::default();
        let mut ch = character();
        route.face_toward(&mut ch, (5, 4));
        route.force_route(RouteStepper::from_move_event(&[
            1,
            8,
            0,
            i32::from(skippable),
            11,
        ]));
        route.advance(&mut ch, (0, 0), &|_, _, _, _, _| passable, &mut vec![]);
        assert_eq!((route.direction(&ch), ch.dir), (DIR_RIGHT, expected));
    }
}

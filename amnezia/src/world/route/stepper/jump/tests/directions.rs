use super::*;

fn jump(route: &mut RouteStepper, ch: &mut EventSprite) -> Option<RouteAction> {
    route
        .advance(ch, (0, 0), &|_, _, _, _, _| true, &mut vec![])
        .map(|(action, _)| action)
}

#[test]
fn forward_inside_a_jump_keeps_both_axes_of_a_diagonal_direction() {
    for (direction, dx, dy, face) in [
        (4, 2, -2, DIR_UP),
        (5, 2, 2, DIR_DOWN),
        (6, -2, 2, DIR_DOWN),
        (7, -2, -2, DIR_UP),
    ] {
        let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, 24, direction, 11, 25]);
        assert_eq!(
            jump(&mut route, &mut character()),
            Some(RouteAction::Jump { dx, dy, face })
        );
    }
}

#[test]
fn jump_relative_turns_use_the_logical_direction_and_the_original_modulo_mapping() {
    for (direction, turn, dx, dy, face) in [
        (4, 16, 2, -1, DIR_RIGHT),
        (4, 17, 0, -1, DIR_UP),
        (4, 18, 1, 0, DIR_RIGHT),
        (5, 16, 1, 2, DIR_DOWN),
        (5, 17, 1, 0, DIR_RIGHT),
        (5, 18, 0, 1, DIR_DOWN),
        (6, 16, -2, 1, DIR_LEFT),
        (6, 17, 0, 1, DIR_DOWN),
        (6, 18, -1, 0, DIR_LEFT),
        (7, 16, -1, -2, DIR_UP),
        (7, 17, -1, 0, DIR_LEFT),
        (7, 18, 0, -1, DIR_UP),
    ] {
        let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, 24, direction, turn, 11, 25]);
        assert_eq!(
            jump(&mut route, &mut character()),
            Some(RouteAction::Jump { dx, dy, face })
        );
    }
}

#[test]
fn jumping_in_place_sets_direction_down_without_turning_the_sprite() {
    let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, 24, 25]);
    let mut ch = character();
    ch.dir = DIR_RIGHT;
    assert_eq!(
        jump(&mut route, &mut ch),
        Some(RouteAction::Jump {
            dx: 0,
            dy: 0,
            face: DIR_RIGHT
        })
    );
    assert_eq!((route.direction(&ch), ch.dir), (DIR_DOWN, DIR_RIGHT));
}

#[test]
fn a_nonzero_jump_turns_spinning_characters_but_preserves_page_and_route_facing_locks() {
    for mode in 0..=6 {
        for locked in [false, true] {
            let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, 24, 1, 25]);
            route.animation.mode = mode;
            route.facing_lock = locked.then_some(DIR_DOWN);
            let mut ch = character();
            jump(&mut route, &mut ch).unwrap();
            assert_eq!(route.direction(&ch), DIR_RIGHT);
            assert_eq!(
                ch.dir,
                if locked || (2..=4).contains(&mode) {
                    DIR_DOWN
                } else {
                    DIR_RIGHT
                }
            );
        }
    }
}

#[test]
fn an_unterminated_jump_skips_movement_but_keeps_its_internal_direction_changes() {
    let mut route = RouteStepper::from_move_event(&[1, 8, 0, 0, 24, 4, 16, 11]);
    let mut ch = character();
    assert!(jump(&mut route, &mut ch).is_none());
    assert_eq!((route.direction(&ch), ch.dir), (DIR_RIGHT, DIR_DOWN));
    assert!(!route.pending());
}

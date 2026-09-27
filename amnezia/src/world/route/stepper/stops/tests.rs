use super::super::*;
use crate::player::Player;

fn character() -> Player {
    Player {
        tile_x: 5,
        tile_y: 5,
        dir: 2,
        frame: 1,
        charset: String::new(),
        index: 0,
    }
}

#[test]
fn forced_assignment_compares_frequency_to_the_original_and_cancellation_retains_count() {
    let mut route = RouteStepper::default();
    route.set_stop_maximum(52);
    route.set_stop_count(12);
    route.force_route(RouteStepper::from_move_event(&[0, 3, 0, 0, 23]));
    assert_eq!((route.stop_count(), route.stop_maximum()), (65535, 52));
    route.force_route(RouteStepper::from_move_event(&[0, 2, 0, 0, 23]));
    assert_eq!((route.stop_count(), route.stop_maximum()), (65535, 128));
    route.force_route(RouteStepper::from_move_event(&[0, 3, 0, 0, 23]));
    assert_eq!(route.stop_maximum(), 128);
    route.force_route(RouteStepper::from_move_event(&[0, 3, 0, 0]));
    assert_eq!((route.stop_count(), route.stop_maximum()), (65535, 64));
    assert!(!route.forced());
}

#[test]
fn changing_frequency_alone_does_not_retune_the_current_threshold() {
    let mut route = RouteStepper::default();
    route.set_stop_maximum(52);
    route.set_stop_count(12);
    route.adjust_freq(1);
    assert_eq!(
        (route.frequency(), route.stop_count(), route.stop_maximum()),
        (4, 12, 52)
    );
    route.adjust_freq(-1);
    assert_eq!(
        (route.frequency(), route.stop_count(), route.stop_maximum()),
        (3, 12, 52)
    );
}

#[test]
fn instant_repeat_stops_at_its_original_index_even_after_wrapping() {
    let mut route = RouteStepper::from_move_event(&[0, 8, 1, 0, 12, 40, 13]);
    route.index = 1;
    let mut character = character();
    let mut effects = Vec::new();
    assert!(
        route
            .advance(&mut character, (0, 0), &|_, _, _, _, _| true, &mut effects)
            .is_none()
    );
    assert_eq!(route.index, 1);
    assert_eq!(character.dir, 0);
    assert!(matches!(effects.as_slice(), [StepEffect::Transparency(1)]));
    assert!(route.forced() && route.active());
    assert!(!route.pending());
}

#[test]
fn skipping_a_blocked_jump_preserves_the_previous_threshold() {
    let mut route = RouteStepper::from_move_event(&[0, 7, 1, 1, 24, 1, 25]);
    route.set_stop_count(2);
    route.set_stop_maximum(2);
    let mut character = character();
    assert!(
        route
            .advance(
                &mut character,
                (0, 0),
                &|_, _, _, _, _| false,
                &mut Vec::new()
            )
            .is_none()
    );
    assert_eq!((route.stop_count(), route.stop_maximum()), (2, 2));
    assert_eq!(character.dir, 2);
}

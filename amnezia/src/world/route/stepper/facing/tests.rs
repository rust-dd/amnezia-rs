use super::*;
use crate::player::Player;
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};

fn character() -> Player {
    Player {
        tile_x: 5,
        tile_y: 5,
        dir: DIR_RIGHT,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    }
}

#[test]
fn relative_turns_start_from_temporary_facing_not_logical_direction() {
    let mut route = RouteStepper::default();
    let mut ch = character();
    route.face_toward(&mut ch, (5, 4));
    route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 16]));
    route.advance(&mut ch, (0, 0), &|_, _, _, _, _| true, &mut vec![]);
    assert_eq!((route.direction(&ch), ch.dir), (DIR_RIGHT, DIR_RIGHT));
}

#[test]
fn a_skipped_blocked_move_or_jump_restores_both_independent_directions() {
    for commands in [vec![2], vec![24, 2, 25]] {
        let mut route = RouteStepper::default();
        let mut ch = character();
        route.face_toward(&mut ch, (5, 4));
        let mut params = vec![1, 8, 0, 1];
        params.extend(commands);
        route.force_route(RouteStepper::from_move_event(&params));
        route.advance(&mut ch, (0, 0), &|_, _, _, _, _| false, &mut vec![]);
        assert_eq!((route.direction(&ch), ch.dir), (DIR_RIGHT, DIR_UP));
    }
}

#[test]
fn explicit_route_turns_override_facing_locks_without_unlocking_movement() {
    for mode in 0..=6 {
        let mut route = RouteStepper::default();
        route.animation.mode = mode;
        let mut ch = character();
        route.force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 26, 14]));
        route.advance(&mut ch, (0, 0), &|_, _, _, _, _| true, &mut vec![]);
        assert_eq!((route.direction(&ch), ch.dir), (DIR_DOWN, DIR_DOWN));
        route.set_direction(&mut ch, DIR_LEFT);
        assert_eq!((route.direction(&ch), ch.dir), (DIR_LEFT, DIR_DOWN));
    }
}

#[test]
fn saving_an_npcs_temporary_facing_retains_the_direction_used_when_it_finishes() {
    let mut route = RouteStepper::default();
    let mut ch = character();
    route.face_toward(&mut ch, (5, 4));
    let route = ron::from_str::<RouteStepper>(&ron::to_string(&route).unwrap()).unwrap();
    assert_eq!((route.direction(&ch), ch.dir), (DIR_RIGHT, DIR_UP));
    route.update_facing(&mut ch);
    assert_eq!(ch.dir, DIR_RIGHT);
}

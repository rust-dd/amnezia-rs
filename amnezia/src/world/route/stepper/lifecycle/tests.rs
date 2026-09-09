use super::*;
use crate::assets::{asset_root, load_ron};
use crate::player::Player;
use crate::world::{MapData, MoveQueue, RouteAction, StepEffect, drive_route};
use amnezia_data::Map;

fn player() -> Player {
    Player {
        tile_x: 5,
        tile_y: 5,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    }
}

fn page(codes: &[u32]) -> EventPage {
    let mut page =
        load_ron::<Map>(&format!("{}/maps/map_0001.ron", asset_root())).events[0].pages[0].clone();
    page.move_type = 6;
    page.move_speed = 3;
    page.move_frequency = 5;
    page.move_route = MoveRouteDef {
        commands: codes
            .iter()
            .map(|&code| MoveCommandDef {
                code,
                ..Default::default()
            })
            .collect(),
        repeat: true,
        skippable: false,
    };
    page
}

fn advance(route: &mut RouteStepper) -> Option<RouteAction> {
    route
        .advance(&mut player(), (0, 0), &|_, _, _| true, &mut Vec::new())
        .map(|(action, _)| action)
}

#[test]
fn animation_pause_is_character_state_across_forced_routes_and_page_changes() {
    let mut source = page(&[]);
    source.animation_type = 1;
    let mut route = RouteStepper::from_event_page(Some(&source));
    route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 38]));
    advance(&mut route);
    assert!(route.animation.paused);
    assert!(!route.forced());
    source.animation_type = 3;
    route.refresh_page(Some(&source));
    assert_eq!(route.animation.mode, 3);
    assert!(route.animation.paused);
    route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 39]));
    advance(&mut route);
    assert!(!route.animation.paused);
    assert_eq!(route.animation.mode, 3);
}

#[test]
fn route_unlock_and_turn_commands_cannot_override_a_pages_fixed_or_spinning_facing() {
    for mode in 0..=5 {
        let mut source = page(&[]);
        source.animation_type = mode;
        let mut route = RouteStepper::from_event_page(Some(&source));
        let mut ch = player();
        route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 27, 12]));
        route.advance(&mut ch, (0, 0), &|_, _, _| true, &mut Vec::new());
        assert_eq!(ch.dir, if mode >= 2 { 2 } else { 0 });
        assert_eq!(route.direction(&ch), 0);
    }
}

#[test]
fn forced_route_restores_page_index_and_frequency_but_keeps_live_speed() {
    let mut route = RouteStepper::from_event_page(Some(&page(&[1, 3])));
    assert_eq!(advance(&mut route).unwrap().delta(), (1, 0));
    route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 28, 12]));
    assert!(route.pending());
    assert!(advance(&mut route).is_none());
    assert!(advance(&mut route).is_none());
    assert!(!route.forced());
    assert_eq!((route.frequency(), route.speed()), (5, 4));
    assert_eq!(advance(&mut route).unwrap().delta(), (-1, 0));
}

#[test]
fn replacing_then_cancelling_a_forced_route_resumes_the_original_page_only() {
    let mut route = RouteStepper::from_event_page(Some(&page(&[1, 3])));
    advance(&mut route);
    route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 23]));
    advance(&mut route);
    route.force_route(RouteStepper::from_move_event(&[0, 2, 0, 0, 0, 23]));
    assert_eq!(advance(&mut route).unwrap().delta(), (0, -1));
    route.force_route(RouteStepper::from_move_event(&[0, 1, 0, 0]));
    assert!(!route.forced());
    assert!(!route.pending());
    assert_eq!(route.frequency(), 5);
    assert_eq!(advance(&mut route).unwrap().delta(), (-1, 0));
}

#[test]
fn page_changes_update_the_suspended_program_without_replacing_the_forced_one() {
    for (codes, expected) in [([1, 3], (-1, 0)), ([0, 2], (0, -1))] {
        let mut route = RouteStepper::from_event_page(Some(&page(&[1, 3])));
        advance(&mut route);
        route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 15]));
        let mut next = page(&codes);
        next.move_frequency = 7;
        next.move_speed = 6;
        route.refresh_page(Some(&next));
        assert!(route.forced());
        assert_eq!(route.commands[0].code, 15);
        assert_eq!((route.frequency(), route.speed()), (7, 6));
        advance(&mut route);
        advance(&mut route);
        assert!(!route.forced());
        assert_eq!(route.frequency(), 7);
        assert_eq!(advance(&mut route).unwrap().delta(), expected);
    }
}

#[test]
fn a_parameter_only_page_change_keeps_the_program_counter() {
    let mut first = page(&[1, 32]);
    first.move_route.commands[1].params = vec![7];
    let mut route = RouteStepper::from_event_page(Some(&first));
    advance(&mut route);
    let mut next = first.clone();
    next.move_route.commands[1].params = vec![593];
    route.refresh_page(Some(&next));
    let mut effects = Vec::new();
    route.advance(&mut player(), (0, 0), &|_, _, _| true, &mut effects);
    assert!(matches!(
        effects.as_slice(),
        [StepEffect::Switch(593, true)]
    ));
}

#[test]
fn a_page_disappearing_suspends_but_does_not_discard_its_forced_route() {
    let page = page(&[1, 3]);
    let mut route = RouteStepper::from_event_page(Some(&page));
    route.force_route(RouteStepper::from_move_event(&[0, 8, 0, 0, 23]));
    route.refresh_page(None);
    assert!(!route.active());
    assert!(route.pending());
    route.refresh_page(Some(&page));
    assert!(route.active());
    assert_eq!(route.commands[0].code, 23);
}

#[test]
fn final_move_finishes_on_landing_without_a_low_frequency_delay() {
    let mut route = RouteStepper::default();
    route.force_route(RouteStepper::from_move_event(&[0, 1, 0, 0, 1]));
    let mut player = player();
    let mut queue = MoveQueue::default();
    let data = MapData::for_test(20, 15);
    for frame in 0..12 {
        drive_route(
            &mut player,
            &mut queue,
            &mut route,
            (0, 0),
            1.0 / 60.0,
            |_, _, _| true,
        );
        queue.advance(&mut player, &data, 1.0 / 60.0);
        if frame == 0 {
            assert!(route.pending());
            assert!(queue.busy());
        }
    }
    assert!(!queue.busy());
    assert!(!route.pending());
    assert!(!route.forced());
    assert_eq!(player.tile_x, 6);
}

#[test]
fn a_repeating_forced_route_stops_blocking_after_its_first_lap() {
    let mut route = RouteStepper::default();
    route.force_route(RouteStepper::from_move_event(&[0, 8, 1, 0, 23]));
    assert!(route.pending());
    advance(&mut route);
    advance(&mut route);
    assert!(!route.pending());
    assert!(route.active());
    assert!(route.forced());
}

#[test]
fn an_all_instant_repeating_route_runs_once_per_tick() {
    let mut route = RouteStepper::from_move_event(&[0, 8, 1, 0, 40]);
    let mut effects = Vec::new();
    route.advance(&mut player(), (0, 0), &|_, _, _| true, &mut effects);
    assert!(matches!(effects.as_slice(), [StepEffect::Transparency(1)]));
    assert!(route.active());
    assert!(!route.pending());
}

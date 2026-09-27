use super::*;
use crate::world::test_support::*;

mod lifecycle;
mod ownership;

#[test]
fn a_route_switch_aborts_when_its_page_changes_the_command_codes() {
    let mut app = app(
        vec![event(
            1,
            1,
            vec![
                page(vec![command(32, 7), command(1, 0)]),
                gated(page(vec![command(40, 0)])),
            ],
        )],
        false,
    );
    let npc = entity(&mut app, 1);
    app.update();
    assert!(app.world().resource::<Switches>().get(7));
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
    assert!(!app.world().get::<MoveQueue>(npc).unwrap().busy());
    assert_eq!(app.world().get::<RouteStepper>(npc).unwrap().alpha(), 1.0);
    app.update();
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().alpha(),
        223.0 / 255.0
    );
}

#[test]
fn parameter_only_page_changes_keep_the_old_program_until_the_next_route_update() {
    let mut original = page(vec![command(32, 7), command(32, 8)]);
    original.move_route.repeat = true;
    let mut next = gated(original.clone());
    next.move_route.commands[1].params = vec![9];
    next.move_route.repeat = false;
    let mut app = app(vec![event(1, 1, vec![original, next])], false);
    app.update();
    assert!(app.world().resource::<Switches>().get(8));
    assert!(!app.world().resource::<Switches>().get(9));
    app.update();
    assert!(app.world().resource::<Switches>().get(9));
}

#[test]
fn a_route_switch_refreshes_other_characters_collision_layers_before_the_next_move() {
    for (from, to, expected) in [(0, 1, 1), (1, 0, 2)] {
        let mut old = page(vec![]);
        old.layer = from;
        let mut new = gated(old.clone());
        new.layer = to;
        let mut app = app(
            vec![
                event(1, 1, vec![page(vec![command(32, 7), command(1, 0)])]),
                event(2, 2, vec![old, new]),
            ],
            false,
        );
        let npc = entity(&mut app, 1);
        app.update();
        assert_eq!(
            app.world().get::<EventSprite>(npc).unwrap().tile_x,
            expected
        );
    }
}

#[test]
fn a_forced_route_keeps_its_program_but_uses_its_new_page_speed_immediately() {
    let mut next = gated(page(vec![command(3, 0)]));
    next.move_speed = 6;
    let mut app = app(vec![event(1, 1, vec![page(vec![]), next])], false);
    let npc = entity(&mut app, 1);
    app.world_mut()
        .get_mut::<RouteStepper>(npc)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 32, 7, 1]));
    app.update();
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 2);
    let queue = app.world().get::<MoveQueue>(npc).unwrap();
    let ch = app.world().get::<EventSprite>(npc).unwrap();
    let data = app.world().resource::<MapData>();
    let start = data.tile_center(1, 1).0;
    assert_eq!(queue.render_position(ch, data).x, start + 8.0);
}

#[test]
fn a_route_page_refresh_can_delay_the_next_command_with_its_new_stop_threshold() {
    let original = page(vec![command(32, 7), command(1, 0)]);
    let mut next = gated(original.clone());
    next.move_frequency = 1;
    let mut app = app(vec![event(1, 1, vec![original, next])], false);
    let npc = entity(&mut app, 1);
    for expected_count in 1..=128 {
        app.update();
        let route = app.world().get::<RouteStepper>(npc).unwrap();
        assert_eq!(
            (route.stop_count(), route.stop_maximum()),
            (expected_count, 128)
        );
        assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
    }
    app.update();
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 2);
}

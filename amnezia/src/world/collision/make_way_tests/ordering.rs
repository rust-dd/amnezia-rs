use super::*;

fn trace(digit: i32) -> Vec<EventCommand> {
    [vec![0, 1, 1, 3, 0, 5], vec![0, 1, 1, 1, 0, digit]]
        .into_iter()
        .map(|params| EventCommand {
            params,
            ..counter()
        })
        .collect()
}

#[test]
fn diagonal_legs_visit_vertical_then_horizontal_with_short_circuiting() {
    let mut vertical = page(vec![]);
    vertical.trigger = 4;
    vertical.layer = 2;
    vertical.commands = trace(2);
    let mut destination = vertical.clone();
    destination.layer = 1;
    destination.commands = trace(3);
    let mut horizontal = vertical.clone();
    horizontal.commands = trace(4);
    let mut events = vec![
        event(1, 1, vec![page(vec![command(5, 0)])]),
        event(2, 1, vec![vertical]),
        event(3, 2, vec![destination]),
        event(4, 2, vec![horizontal]),
    ];
    events[1].y = 2;
    events[2].y = 2;
    let mut app = app(events, false);
    app.update();
    assert_eq!(
        app.world().resource::<crate::state::Variables>().get(1),
        43_569
    );
    for id in 1..=4 {
        let npc = entity(&mut app, id);
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
            1
        );
    }
}

#[test]
fn a_through_mover_skips_callbacks_but_a_through_obstacle_is_still_updated() {
    for mover_through in [false, true] {
        let mut other = page(vec![command(36, 0)]);
        other.trigger = 4;
        other.commands = vec![counter()];
        let commands = if mover_through {
            vec![command(36, 0), command(1, 0)]
        } else {
            vec![command(1, 0)]
        };
        let mut app = app(
            vec![event(1, 1, vec![page(commands)]), event(2, 2, vec![other])],
            false,
        );
        app.update();
        assert_eq!(
            app.world().resource::<crate::state::Variables>().get(1),
            if mover_through { 1 } else { 2 }
        );
        let npc = entity(&mut app, 1);
        assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 2);
    }
}

#[test]
fn collision_visits_consume_parallel_wait_ticks_even_after_character_processing() {
    let mut other = page(vec![]);
    other.trigger = 4;
    other.commands = vec![
        EventCommand {
            code: 11410,
            params: vec![1],
            ..counter()
        },
        counter(),
    ];
    let mut app = app(
        vec![
            event(1, 1, vec![page(vec![command(1, 0)])]),
            event(2, 2, vec![other]),
            event(3, 3, vec![page(vec![command(3, 0)])]),
        ],
        false,
    );
    for _ in 0..2 {
        app.update();
        assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 0);
    }
    app.update();
    assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 1);
    let npc = entity(&mut app, 2);
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        3
    );
}

#[test]
fn autonomous_and_scripted_movers_share_the_same_make_way_updates() {
    let mut horizontal = page(vec![]);
    horizontal.move_type = 3;
    let mut app = app(
        vec![
            event(1, 1, vec![horizontal.clone()]),
            event(2, 2, vec![page(vec![command(1, 0)])]),
            event(3, 3, vec![horizontal]),
        ],
        false,
    );
    app.update();
    for id in 1..=3 {
        let npc = entity(&mut app, id);
        assert_eq!(
            app.world().get::<EventSprite>(npc).unwrap().tile_x,
            id as i32 + 1
        );
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
            0
        );
    }
}

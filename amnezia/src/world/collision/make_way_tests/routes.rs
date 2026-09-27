use super::*;

fn script(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        params,
        ..counter()
    }
}

fn parallel(commands: Vec<EventCommand>) -> amnezia_data::EventPage {
    let mut page = page(vec![]);
    page.trigger = 4;
    page.commands = commands;
    page
}

fn once(mut commands: Vec<EventCommand>) -> Vec<amnezia_data::EventPage> {
    commands.push(script(10210, vec![0, 7, 7, 0]));
    vec![parallel(commands), gated(page(vec![]))]
}

#[test]
fn a_callback_relocation_does_not_replace_the_captured_move_origin() {
    for jumping in [false, true] {
        let commands = if jumping {
            vec![command(24, 0), command(1, 0), command(25, 0)]
        } else {
            vec![command(1, 0)]
        };
        let mut app = app(
            vec![
                event(1, 1, vec![page(commands)]),
                event(
                    2,
                    2,
                    once(vec![
                        script(10860, vec![1, 0, 7, 7]),
                        script(10860, vec![2, 0, 8, 1]),
                    ]),
                ),
            ],
            false,
        );
        app.update();
        let npc = entity(&mut app, 1);
        let character = app.world().get::<EventSprite>(npc).unwrap();
        assert_eq!(character.tile(), (2, 1));
        let queue = app.world().get::<MoveQueue>(npc).unwrap();
        let data = app.world().resource::<MapData>();
        let elapsed = if jumping { 16.0 / 11.0 } else { 2.0 };
        let pixels = queue.ground_position(character, data).x - data.tile_center(1, 1).0;
        assert!((pixels - elapsed).abs() < 0.00001);
        assert_eq!(queue.jumping(), jumping);
    }
}

#[test]
fn an_interrupted_page_route_advances_its_own_cursor_not_the_new_forced_route() {
    let mut app = app(
        vec![
            event(1, 1, vec![page(vec![command(1, 0), command(32, 9)])]),
            event(
                2,
                2,
                once(vec![
                    script(11330, vec![1, 8, 0, 0, 32, 8, 23]),
                    script(10860, vec![2, 0, 8, 1]),
                ]),
            ),
        ],
        false,
    );
    app.update();
    for _ in 0..8 {
        app.update();
    }
    assert!(app.world().resource::<crate::state::Switches>().get(8));
    assert!(!app.world().resource::<crate::state::Switches>().get(9));
}

#[test]
fn replacing_a_forced_program_during_collision_uses_its_live_cursor_and_skip_flag() {
    let mut app = app(
        vec![
            event(1, 1, vec![page(vec![])]),
            event(2, 2, once(vec![script(11330, vec![1, 8, 0, 1, 23, 32, 8])])),
        ],
        false,
    );
    let npc = entity(&mut app, 1);
    app.world_mut()
        .get_mut::<RouteStepper>(npc)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 1, 32, 9]));
    app.update();
    assert!(app.world().resource::<crate::state::Switches>().get(8));
    assert!(!app.world().resource::<crate::state::Switches>().get(9));
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
}

#[test]
fn a_skipped_failed_walk_restores_its_pose_after_scheduling_the_collision_event() {
    let mut page = page(vec![command(1, 0)]);
    page.direction = 0;
    page.move_route.skippable = true;
    page.trigger = 2;
    page.commands = vec![script(11410, vec![10])];
    let mut event = event(1, 4, vec![page]);
    event.y = 5;
    let mut app = app(vec![event], false);
    app.update();
    let npc = entity(&mut app, 1);
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().dir, 0);
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        1
    );
    assert!(
        app.world()
            .resource::<crate::interpreter::RunningEvent>()
            .active()
    );
}

#[test]
fn callback_page_changes_apply_live_collision_speed_and_stopping_dependent_poses() {
    for jumping in [false, true] {
        let commands = if jumping {
            vec![command(24, 0), command(1, 0), command(25, 0)]
        } else {
            vec![command(1, 0)]
        };
        let original = page(commands);
        let mut next = gated(original.clone());
        next.direction = 0;
        next.move_speed = 3;
        next.layer = 2;
        next.graphic_index = 1;
        let mut app = app(
            vec![event(1, 1, vec![original, next]), event(2, 2, once(vec![]))],
            false,
        );
        app.update();
        let npc = entity(&mut app, 1);
        let character = app.world().get::<EventSprite>(npc).unwrap();
        assert_eq!(character.tile(), (2, 1));
        assert_eq!((character.layer, character.index), (2, 1));
        assert_eq!(character.dir, if jumping { 1 } else { 0 });
        let queue = app.world().get::<MoveQueue>(npc).unwrap();
        let data = app.world().resource::<MapData>();
        let pixels = queue.ground_position(character, data).x - data.tile_center(1, 1).0;
        assert!((pixels - 1.0).abs() < 0.00001);
    }
}

#[test]
fn a_page_replacement_during_a_skipped_move_keeps_the_original_calls_skip_and_commands() {
    let mut original = page(vec![command(1, 0), command(32, 9)]);
    original.move_route.skippable = true;
    let next = gated(page(vec![command(40, 0), command(32, 8)]));
    let mut app = app(
        vec![event(1, 1, vec![original, next]), event(2, 2, once(vec![]))],
        false,
    );
    app.update();
    assert!(app.world().resource::<crate::state::Switches>().get(9));
    assert!(!app.world().resource::<crate::state::Switches>().get(8));
    let npc = entity(&mut app, 1);
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
}

#[test]
fn cancelling_a_forced_call_mid_move_does_not_advance_the_restored_page_cursor() {
    let mut app = app(
        vec![
            event(1, 1, vec![page(vec![command(32, 9)])]),
            event(
                2,
                2,
                once(vec![
                    script(11330, vec![1, 8, 0, 0]),
                    script(10860, vec![2, 0, 8, 1]),
                ]),
            ),
        ],
        false,
    );
    let npc = entity(&mut app, 1);
    app.world_mut()
        .get_mut::<RouteStepper>(npc)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[1, 8, 0, 0, 1]));
    app.update();
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 2);
    for _ in 0..8 {
        app.update();
    }
    assert!(app.world().resource::<crate::state::Switches>().get(9));
}

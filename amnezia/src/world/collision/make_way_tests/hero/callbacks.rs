use super::*;

fn script(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        params,
        ..counter()
    }
}

fn callback(commands: Vec<EventCommand>) -> amnezia_data::Event {
    let mut parallel = page(vec![]);
    parallel.trigger = 4;
    parallel.commands = commands;
    parallel.commands.push(script(10210, vec![0, 7, 7, 0]));
    let mut event = at(2, 6, 5, parallel);
    event.pages.push(gated(page(vec![])));
    event
}

#[test]
fn a_hero_collision_callback_replaces_the_live_forced_cursor_and_skip_flag() {
    let mut app = app(
        vec![
            at(1, 4, 5, page(vec![command(1, 0)])),
            callback(vec![script(11330, vec![10001, 8, 0, 1, 23, 32, 8])]),
        ],
        false,
    );
    let hero = hero_entity(&mut app);
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 1, 32, 9]));
    app.update();
    let switches = app.world().resource::<crate::state::Switches>();
    assert!(switches.get(8));
    assert!(!switches.get(9));
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (5, 5));
}

#[test]
fn cancelling_the_hero_route_in_a_callback_keeps_the_move_without_replaying_its_tail() {
    let mut app = app(
        vec![
            at(1, 4, 5, page(vec![command(1, 0)])),
            callback(vec![
                script(11330, vec![10001, 8, 0, 0]),
                script(10860, vec![2, 0, 9, 1]),
            ]),
        ],
        false,
    );
    let hero = hero_entity(&mut app);
    app.world_mut()
        .get_mut::<RouteStepper>(hero)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 1, 32, 9]));
    app.update();
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
    assert!(!app.world().get::<RouteStepper>(hero).unwrap().forced());
    for _ in 0..10 {
        app.update();
    }
    assert!(!app.world().resource::<crate::state::Switches>().get(9));
}

#[test]
fn a_later_parallel_scene_replaces_an_early_heroes_menu_request() {
    let mut parallel = gated(page(vec![]));
    parallel.trigger = 4;
    parallel.commands = vec![script(11910, vec![]), script(11410, vec![100])];
    let mut observer = at(2, 9, 1, page(vec![]));
    observer.pages.push(parallel);
    let mut app = app(
        vec![at(1, 4, 5, page(vec![command(1, 0)])), observer],
        false,
    );
    crate::menu::register_map_input(&mut app);
    app.add_plugins((
        crate::timing::TimingPlugin,
        crate::transitions::TransitionPlugin,
    ));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(app.world().resource::<crate::menu::Calling>().pending());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut()
        .resource_mut::<crate::state::Switches>()
        .set(7, true);
    app.update();
    assert!(app.world().resource::<crate::save::EventSaveRequest>().0);
    assert!(!app.world().resource::<crate::menu::SceneFlow>().active());
}

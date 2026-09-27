use super::*;

fn vehicle_app() -> App {
    let mut app = app();
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    app
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    *input = default();
    for &key in keys {
        input.press(key);
    }
    app.update();
}

fn start_two_frame_step(app: &mut App) {
    let world = app.world_mut();
    queue_hero_step(world, crate::tiles::DIR_RIGHT, 6);
    app.update();
    let world = app.world_mut();
    assert!(world.query::<&MoveQueue>().single(world).unwrap().busy());
    assert_eq!(hero_x(app), 6);
}

fn put_action_event(app: &mut App) {
    crate::dialogue::testing::register_actions(app);
    let mut event = map_event(1, 0, vec![switch_cmd(1, 0, 0), cmd(11410, 0, vec![100])]);
    event.x = 7;
    event.y = 5;
    event.pages[0].layer = 1;
    app.insert_resource(MapEvents {
        events: vec![event],
    });
}

#[test]
fn a_boat_moves_before_processing_a_simultaneous_disembark_button() {
    let mut app = vehicle_app();
    {
        let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
        vehicles.set_location(0, 0, 5, 5);
        vehicles.save.riding = Some(0);
    }
    press(&mut app, &[KeyCode::ArrowRight, KeyCode::Enter]);
    let vehicles = app.world().resource::<crate::vehicles::Vehicles>();
    assert!(vehicles.riding());
    assert_eq!(
        vehicles.character(10002),
        Some((6, 5, crate::tiles::DIR_RIGHT))
    );
    assert_eq!(hero_x(&mut app), 6);
}

#[test]
fn boarding_waits_until_the_hero_was_already_stopped_at_update_start() {
    let mut app = vehicle_app();
    app.world_mut()
        .resource_mut::<crate::vehicles::Vehicles>()
        .set_location(0, 0, 7, 5);
    start_two_frame_step(&mut app);
    press(&mut app, &[KeyCode::Enter]);
    assert!(!app.world().resource::<crate::vehicles::Vehicles>().riding());
    assert_eq!(hero_x(&mut app), 6);
    press(&mut app, &[KeyCode::Enter]);
    assert!(app.world().resource::<crate::vehicles::Vehicles>().riding());
}

#[test]
fn a_facing_action_cannot_reuse_the_last_walking_update() {
    let mut app = app();
    put_action_event(&mut app);
    start_two_frame_step(&mut app);
    press(&mut app, &[KeyCode::Enter]);
    assert!(!switch_on(&app, 1));
    press(&mut app, &[KeyCode::Enter]);
    assert!(switch_on(&app, 1));
}

#[test]
fn an_idle_forced_route_still_owns_action_and_boarding_input() {
    for boarding in [false, true] {
        let mut app = vehicle_app();
        if boarding {
            app.world_mut()
                .resource_mut::<crate::vehicles::Vehicles>()
                .set_location(0, 0, 5, 6);
        } else {
            put_action_event(&mut app);
            let world = app.world_mut();
            let mut player = world.query::<&mut Player>().single_mut(world).unwrap();
            player.tile_x = 6;
            player.dir = crate::tiles::DIR_RIGHT;
        }
        let world = app.world_mut();
        *world
            .query::<&mut RouteStepper>()
            .single_mut(world)
            .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 23, 1]);
        press(&mut app, &[KeyCode::Enter]);
        assert!(!switch_on(&app, 1));
        assert!(!app.world().resource::<crate::vehicles::Vehicles>().riding());
    }
}

#[test]
fn a_finished_instant_route_releases_input_in_the_same_update() {
    let mut app = vehicle_app();
    app.world_mut()
        .resource_mut::<crate::vehicles::Vehicles>()
        .set_location(0, 0, 5, 6);
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 36]);
    press(&mut app, &[KeyCode::Enter]);
    assert!(app.world().resource::<crate::vehicles::Vehicles>().riding());
}

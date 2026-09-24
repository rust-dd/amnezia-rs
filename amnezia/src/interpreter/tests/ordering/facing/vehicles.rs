use super::*;
use crate::vehicles::{VehiclePlugin, Vehicles};

fn boat_app() -> App {
    let mut app = app();
    crate::dialogue::testing::register_actions(&mut app);
    app.add_plugins(VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    let world = app.world_mut();
    let (mut hero, mut route) = world
        .query::<(&mut Player, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.set_direction(&mut *hero, DIR_LEFT);
    let mut vehicles = world.resource_mut::<Vehicles>();
    vehicles.set_location(0, 0, 5, 5);
    vehicles.save.riding = Some(0);
    vehicles.save.vehicles[0].dir = DIR_DOWN;
    app
}

fn hero_directions(app: &mut App) -> (u32, u32) {
    let world = app.world_mut();
    let (hero, route) = world
        .query::<(&Player, &RouteStepper)>()
        .single(world)
        .unwrap();
    (route.direction(hero), hero.dir)
}

fn action(app: &mut App, x: u32, y: u32) {
    let mut event = map_event(1, 0, vec![switch_cmd(10, 0, 0), cmd(11410, 0, vec![100])]);
    event.x = x;
    event.y = y;
    event.pages[0].layer = 1;
    app.insert_resource(MapEvents {
        events: vec![event],
    });
}

#[test]
fn boarding_uses_the_heros_direction_even_when_its_facing_is_locked() {
    let mut app = boat_app();
    let world = app.world_mut();
    let (mut hero, mut route, mut queue) = world
        .query::<(&mut Player, &mut RouteStepper, &mut MoveQueue)>()
        .single_mut(world)
        .unwrap();
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 26]));
    crate::world::drive_route(
        &mut *hero,
        &mut queue,
        &mut route,
        (0, 0),
        1.0 / 60.0,
        |_, _, _, _, _| true,
    );
    route.set_direction(&mut *hero, DIR_DOWN);
    let mut vehicles = world.resource_mut::<Vehicles>();
    vehicles.save.riding = None;
    vehicles.set_location(0, 0, 5, 6);
    talk(&mut app);
    assert!(app.world().resource::<Vehicles>().riding());
}

#[test]
fn a_rider_talks_in_the_boats_live_direction_not_its_preboarding_direction() {
    let mut app = boat_app();
    action(&mut app, 5, 6);
    app.update();
    talk(&mut app);
    assert!(switch_on(&app, 10));
    assert!(app.world().resource::<Vehicles>().riding());
    assert_eq!(hero_directions(&mut app), (DIR_DOWN, DIR_DOWN));
}

#[test]
fn a_fixed_facing_boat_uses_its_direction_for_disembarking_and_talking() {
    let mut app = boat_app();
    action(&mut app, 6, 5);
    app.world_mut().resource_mut::<Vehicles>().set_route(
        10002,
        RouteStepper::from_move_event(&[10002, 8, 0, 0, 26, 13]),
    );
    app.update();
    app.update();
    app.world_mut().resource_mut::<Vehicles>().save.vehicles[0].dir = DIR_DOWN;
    talk(&mut app);
    assert!(app.world().resource::<Vehicles>().riding());
    assert!(switch_on(&app, 10));
    assert_eq!(hero_directions(&mut app), (DIR_RIGHT, DIR_DOWN));
}

#[test]
fn disembarking_updates_the_heros_direction_before_its_next_action() {
    let mut app = boat_app();
    action(&mut app, 7, 5);
    app.world_mut().resource_mut::<Vehicles>().save.vehicles[0].dir = DIR_RIGHT;
    talk(&mut app);
    assert!(!app.world().resource::<Vehicles>().riding());
    assert_eq!(hero_x(&mut app), 6);
    talk(&mut app);
    assert!(switch_on(&app, 10));
    assert!(!app.world().resource::<Vehicles>().riding());
    assert_eq!(hero_directions(&mut app), (DIR_RIGHT, DIR_RIGHT));
}

#[test]
fn manual_rider_movement_replaces_a_finished_routes_direction() {
    let mut app = boat_app();
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_route(10002, RouteStepper::from_move_event(&[10002, 8, 0, 0, 13]));
    app.update();
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    app.update();
    assert_eq!(hero_directions(&mut app), (DIR_DOWN, DIR_DOWN));
}

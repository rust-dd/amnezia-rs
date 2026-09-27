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
    route.set_direction(&mut *hero, DIR_DOWN);
    let mut vehicles = world.resource_mut::<Vehicles>();
    vehicles.set_location(0, 0, 5, 5);
    vehicles.save.riding = Some(0);
    vehicles.save.vehicles[0].dir = DIR_LEFT;
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
    route.set_direction(&mut *hero, DIR_LEFT);
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 26]));
    crate::world::drive_route(
        &mut *hero,
        &mut queue,
        &mut route,
        (0, 0),
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
fn a_rider_talks_in_the_heros_live_direction_and_the_boat_copies_it() {
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
    force_hero(&mut app, &[26, 13]);
    app.update();
    app.update();
    let world = app.world_mut();
    world.query::<&mut Player>().single_mut(world).unwrap().dir = DIR_DOWN;
    talk(&mut app);
    assert!(app.world().resource::<Vehicles>().riding());
    assert!(switch_on(&app, 10));
    assert_eq!(hero_directions(&mut app), (DIR_RIGHT, DIR_DOWN));
}

#[test]
fn disembarking_updates_the_heros_direction_before_its_next_action() {
    let mut app = boat_app();
    action(&mut app, 7, 5);
    crate::vehicles::test_support::direction(&mut app, DIR_RIGHT);
    talk(&mut app);
    assert!(!app.world().resource::<Vehicles>().riding());
    assert_eq!(hero_x(&mut app), 6);
    crate::vehicles::test_support::ticks(&mut app, 7);
    talk(&mut app);
    assert!(switch_on(&app, 10));
    assert!(!app.world().resource::<Vehicles>().riding());
    assert_eq!(hero_directions(&mut app), (DIR_RIGHT, DIR_RIGHT));
}

#[test]
fn manual_rider_movement_replaces_a_finished_routes_direction() {
    let mut app = boat_app();
    force_hero(&mut app, &[13]);
    app.update();
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    app.update();
    assert_eq!(hero_directions(&mut app), (DIR_DOWN, DIR_DOWN));
}

pub(super) fn force_hero(app: &mut App, commands: &[i32]) {
    let mut params = vec![10001, 8, 0, 0];
    params.extend_from_slice(commands);
    let world = app.world_mut();
    world
        .query_filtered::<&mut RouteStepper, With<Player>>()
        .single_mut(world)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&params));
}

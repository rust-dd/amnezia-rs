use super::*;

fn set_hero(app: &mut App, direction: u32, facing: u32) {
    let world = app.world_mut();
    let (mut hero, mut route) = world
        .query::<(&mut Player, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    route.set_direction(&mut *hero, direction);
    hero.dir = facing;
}

#[test]
fn decision_normalizes_a_diagonal_direction_before_looking_for_events_without_a_vehicle() {
    let (mut app, _) = setup(0);
    set_hero(&mut app, 5, DIR_DOWN);
    talk(&mut app);
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
    let world = app.world_mut();
    let (hero, route) = world
        .query::<(&Player, &RouteStepper)>()
        .single(world)
        .unwrap();
    assert_eq!(route.direction(hero), DIR_DOWN);
}

#[test]
fn boarding_normalizes_a_diagonal_direction_to_the_heros_facing_first() {
    let mut app = app();
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    set_hero(&mut app, 4, DIR_UP);
    app.world_mut()
        .resource_mut::<crate::vehicles::Vehicles>()
        .set_location(0, 0, 5, 4);
    talk(&mut app);
    let vehicles = app.world().resource::<crate::vehicles::Vehicles>();
    assert!(vehicles.riding());
    assert!(vehicles.save.boarding);
    assert_eq!(hero_direction(&mut app), DIR_UP);
}

#[test]
fn a_diagonal_boat_route_is_normalized_before_disembarking() {
    let mut app = app();
    app.insert_resource(crate::world::test_support::water_map(10, 10));
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
    vehicles.set_location(0, 0, 5, 5);
    vehicles.save.riding = Some(0);
    vehicles.save.vehicles[0].dir = DIR_DOWN;
    set_hero(&mut app, DIR_DOWN, DIR_DOWN);
    vehicles::force_hero(&mut app, &[5]);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(hero_direction(&mut app), 5);
    talk(&mut app);
    assert!(!app.world().resource::<crate::vehicles::Vehicles>().riding());
    let world = app.world_mut();
    let (hero, route) = world
        .query::<(&Player, &RouteStepper)>()
        .single(world)
        .unwrap();
    assert_eq!(hero.tile(), (6, 7));
    assert_eq!(route.direction(hero), DIR_DOWN);
}

#[test]
fn scripted_boarding_uses_cardinal_direction_or_normalizes_a_diagonal_to_facing() {
    for (direction, x, y) in [(DIR_RIGHT, 6, 5), (4, 5, 4)] {
        let mut app = app();
        app.add_plugins(crate::vehicles::VehiclePlugin)
            .init_resource::<crate::audio::CurrentBgm>();
        set_hero(&mut app, direction, DIR_UP);
        app.world_mut()
            .resource_mut::<crate::vehicles::Vehicles>()
            .set_location(0, 0, x, y);
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(0, vec![cmd(10840, 0, vec![])]);
        app.update();
        let vehicles = app.world().resource::<crate::vehicles::Vehicles>();
        assert!(vehicles.riding(), "direction {direction}");
        assert_eq!(
            hero_direction(&mut app),
            if direction < 4 { direction } else { DIR_UP }
        );
    }
}

fn hero_direction(app: &mut App) -> u32 {
    let world = app.world_mut();
    let (hero, route) = world
        .query::<(&Player, &RouteStepper)>()
        .single(world)
        .unwrap();
    route.direction(hero)
}

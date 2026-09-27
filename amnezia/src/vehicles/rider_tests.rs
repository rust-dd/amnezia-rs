use super::*;
use crate::world::test_support::water_map;

mod boarding;
mod interpreter;
mod logical;
mod relocation;
mod synchronization;

fn app() -> (App, Entity) {
    test_support::rider_app(water_map(20, 20), (5, 5))
}

fn script(app: &mut App, target: i32, commands: &[i32]) {
    let mut params = vec![target, 8, 0, 0];
    params.extend_from_slice(commands);
    app.world_mut()
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(
            0,
            vec![amnezia_data::EventCommand {
                code: 11330,
                indent: 0,
                string: String::new(),
                params,
            }],
        );
}

fn rider(app: &mut App, index: usize) {
    let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
    vehicles.set_location(index, 0, 5, 5);
    vehicles.save.riding = Some(index);
}

#[test]
fn rider_routes_target_the_hero_for_both_hero_and_occupied_vehicle_references() {
    for target in [10001, 10004] {
        let (mut app, hero) = app();
        rider(&mut app, 2);
        script(&mut app, target, &[1]);
        app.update();
        assert!(app.world().get::<RouteStepper>(hero).unwrap().pending());
        assert!(!app.world().resource::<Vehicles>().routes_pending());
    }
}

#[test]
fn rider_routes_forward_only_the_graphic_and_keep_other_modifiers_on_the_hero() {
    let (mut app, hero) = app();
    rider(&mut app, 2);
    let original_graphic = app.world().get::<Player>(hero).unwrap().charset.clone();
    script(
        &mut app,
        10004,
        &[29, 36, 40, 34, 6, 67, 104, 97, 114, 97, 52, 2, 13],
    );
    app.update();
    app.update();
    let route = app.world().get::<RouteStepper>(hero).unwrap();
    assert_eq!(route.speed(), 3);
    assert!(route.through());
    assert_eq!(route.alpha(), crate::tiles::character_alpha(1));
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(vehicles.save.vehicles[2].speed, 5);
    assert!(!vehicles.motion[2].route.through());
    assert_eq!(vehicles.motion[2].alpha, 1.0);
    assert_eq!(vehicles.save.vehicles[2].charset(), "Chara4");
    assert_eq!(vehicles.save.vehicles[2].index(), 2);
    assert_eq!(
        app.world().get::<Player>(hero).unwrap().charset(),
        original_graphic
    );
}

#[test]
fn rider_manual_movement_keeps_the_heros_live_queue_and_mirrors_its_position() {
    let (mut app, hero) = app();
    rider(&mut app, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    let player = app.world().get::<Player>(hero).unwrap();
    let queue = app.world().get::<MoveQueue>(hero).unwrap();
    assert!(queue.busy());
    assert_eq!(player.tile(), (6, 5));
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(vehicles.save.vehicles[0].tile(), player.tile());
    assert_eq!(
        vehicles.motion[0].queue.ground_position(
            &vehicles.save.vehicles[0],
            app.world().resource::<MapData>()
        ),
        queue.ground_position(player, app.world().resource::<MapData>())
    );
}

#[test]
fn rider_boarding_uses_a_hero_step_at_the_preboarding_speed() {
    let (mut app, hero) = app();
    {
        let world = app.world_mut();
        let (mut player, mut route) = world
            .query::<(&mut Player, &mut RouteStepper)>()
            .get_mut(world, hero)
            .unwrap();
        route.set_direction(&mut *player, DIR_RIGHT);
        route.set_speed(2);
    }
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(0, 0, 6, 5);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<Vehicles>().save.riding, Some(0));
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
    assert_eq!(app.world().get::<RouteStepper>(hero).unwrap().speed(), 2);
    let data = app.world().resource::<MapData>();
    let point = app
        .world()
        .get::<MoveQueue>(hero)
        .unwrap()
        .ground_position(app.world().get::<Player>(hero).unwrap(), data);
    assert_eq!(point, Vec2::from(data.tile_center(5, 5)));
}

#[test]
fn rider_boarding_checks_the_source_tiles_outgoing_edge() {
    let (mut app, hero) = app();
    {
        let world = app.world_mut();
        let (mut player, mut route) = world
            .query::<(&mut Player, &mut RouteStepper)>()
            .get_mut(world, hero)
            .unwrap();
        route.set_direction(&mut *player, DIR_RIGHT);
        let mut data = world.resource_mut::<MapData>();
        crate::world::test_support::block_tile(&mut data, (5, 5));
    }
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_location(0, 0, 6, 5);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert!(!app.world().resource::<Vehicles>().riding());
}

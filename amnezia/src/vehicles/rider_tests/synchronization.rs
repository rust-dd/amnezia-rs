use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn rider_sync_preserves_vehicle_clocks_speed_and_graphic_and_cancels_its_old_route() {
    let (mut app, hero) = app();
    rider(&mut app, 2);
    app.world_mut().resource_mut::<Vehicles>().set_route(
        10004,
        RouteStepper::from_move_event(&[10004, 8, 0, 0, 32, 9, 1]),
    );
    let before = app.world().resource::<Vehicles>().motion[2]
        .route
        .stop_count();
    app.update();
    assert!(!app.world().resource::<Switches>().get(9));
    assert!(!app.world().resource::<Vehicles>().routes_pending());
    assert_eq!(
        app.world().resource::<Vehicles>().motion[2]
            .route
            .stop_count(),
        before
    );
    let hero_state = crate::world::saved::hero::snapshot(app.world_mut()).unwrap();
    for _ in 0..15 {
        app.world_mut()
            .run_system_once(super::super::rider::sync)
            .unwrap();
    }
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(vehicles.save.vehicles[2].frame, 2);
    assert_eq!(vehicles.save.vehicles[2].speed, 5);
    assert_eq!(vehicles.motion[2].route.stop_count(), before);
    assert_eq!(
        crate::world::saved::hero::snapshot(app.world_mut()).unwrap(),
        hero_state
    );
    assert!(!app.world().get::<MoveQueue>(hero).unwrap().busy());
}

#[test]
fn a_rider_jump_keeps_the_vehicle_flat_and_uses_the_vehicle_passage_rule() {
    let (mut app, hero) = app();
    rider(&mut app, 0);
    script(&mut app, 10002, &[24, 1, 1, 25]);
    app.update();
    app.update();
    let queue = app.world().get::<MoveQueue>(hero).unwrap();
    assert!(queue.jumping());
    let player = app.world().get::<Player>(hero).unwrap();
    let vehicles = app.world().resource::<Vehicles>();
    assert!(!vehicles.motion[0].queue.jumping());
    assert_eq!(vehicles.save.vehicles[0].tile(), (7, 5));
    let data = app.world().resource::<MapData>();
    let boat = vehicles.motion[0]
        .queue
        .render_position(&vehicles.save.vehicles[0], data);
    assert_eq!(boat.y, data.tile_center(7, 5).1);
    assert!(queue.render_position(player, data).y > boat.y);
    assert!(boat.x > queue.ground_position(player, data).x);
}

#[test]
fn occupied_boat_collision_updates_destination_events_before_the_hero_moves() {
    use crate::world::test_support::{command, entity, event, page};
    let mut target = event(1, 6, vec![page(vec![command(1, 0)])]);
    target.y = 5;
    let mut app = crate::world::test_support::app(vec![target], false);
    app.insert_resource(water_map(20, 20))
        .init_resource::<CurrentBgm>()
        .init_resource::<crate::system_bgm::SystemBgm>()
        .add_plugins(VehiclePlugin);
    rider(&mut app, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    let npc = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<crate::world::EventSprite>(npc)
            .unwrap()
            .tile(),
        (7, 5)
    );
    assert_eq!(
        app.world().resource::<Vehicles>().save.vehicles[0].tile(),
        (6, 5)
    );
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(
        vehicles.motion[0].queue.ground_position(
            &vehicles.save.vehicles[0],
            app.world().resource::<MapData>()
        ),
        Vec2::from(app.world().resource::<MapData>().tile_center(5, 5)) + Vec2::X * 2.0
    );
}

#[test]
fn an_occupied_vehicle_can_update_before_its_rider_then_sync_back_without_rechecking_the_npc() {
    use crate::world::test_support::{command, entity, event, page};
    let mut app =
        crate::world::test_support::app(vec![event(1, 1, vec![page(vec![command(1, 0)])])], false);
    app.insert_resource(water_map(20, 20))
        .init_resource::<CurrentBgm>()
        .init_resource::<crate::system_bgm::SystemBgm>()
        .add_plugins(VehiclePlugin);
    let world = app.world_mut();
    world
        .query::<&mut Player>()
        .single_mut(world)
        .unwrap()
        .set_tile(2, 1);
    let mut vehicles = world.resource_mut::<Vehicles>();
    vehicles.set_location(0, 0, 2, 1);
    vehicles.save.riding = Some(0);
    vehicles.set_route(
        10002,
        RouteStepper::from_move_event(&[10002, 8, 0, 0, 32, 9, 1]),
    );
    app.update();
    assert!(app.world().resource::<Switches>().get(9));
    let npc = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<crate::world::EventSprite>(npc)
            .unwrap()
            .tile(),
        (2, 1)
    );
    let vehicles = app.world().resource::<Vehicles>();
    assert_eq!(vehicles.save.vehicles[0].tile(), (2, 1));
    assert!(!vehicles.motion[0].route.pending() && !vehicles.motion[0].queue.busy());
    assert_eq!(vehicles.motion[0].route.stop_count(), 0);
}

#[test]
fn hero_route_through_does_not_replace_the_occupied_vehicles_collision_flag() {
    let (mut app, hero) = app();
    rider(&mut app, 0);
    crate::world::test_support::block_tile(&mut app.world_mut().resource_mut::<MapData>(), (6, 5));
    script(&mut app, 10001, &[36, 1]);
    app.update();
    app.update();
    assert!(app.world().get::<RouteStepper>(hero).unwrap().through());
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (5, 5));
    let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
    let vehicles = &mut *vehicles;
    let motion = &mut vehicles.motion[0];
    motion
        .route
        .force_route(RouteStepper::from_move_event(&[10002, 8, 0, 0, 36]));
    drive_route(
        &mut vehicles.save.vehicles[0],
        &mut motion.queue,
        &mut motion.route,
        (5, 5),
        |_, _, _, _, _| true,
    );
    app.update();
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
}

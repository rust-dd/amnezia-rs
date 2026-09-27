use super::*;
use crate::player::Player;
use crate::tiles::{ABOVE_HERO_BIT, PASS_ALL};
use crate::vehicles::{Vehicles, test_support::passage_app as app};
use crate::world::Character;
use bevy::prelude::*;

mod geometry;
mod original;

fn water(app: &mut App) {
    let mut map = app.world_mut().resource_mut::<MapData>();
    map.terrains[0].boat_pass = true;
    map.terrains[0].ship_pass = true;
    map.passages_up[0] = ABOVE_HERO_BIT | PASS_ALL;
}

fn assert_step(app: &mut App, index: usize, allowed: bool) {
    app.update();
    assert_eq!(
        app.world().resource::<Vehicles>().save.vehicles[index].tile(),
        if allowed { (5, 4) } else { (4, 4) }
    );
}

#[test]
fn boat_and_ship_manual_and_forced_steps_obey_their_own_terrain_permission() {
    for index in 0..2 {
        for manual in [false, true] {
            let mut app = app(index, manual);
            app.world_mut().resource_mut::<MapData>().passages_up[0] = ABOVE_HERO_BIT | PASS_ALL;
            assert_step(&mut app, index, false);
            let mut map = app.world_mut().resource_mut::<MapData>();
            map.terrains[0].boat_pass = index == 0;
            map.terrains[0].ship_pass = index == 1;
            assert_step(&mut app, index, true);
        }
    }
}

#[test]
fn water_vehicles_need_an_above_upper_tile_even_when_it_is_walkable() {
    for index in 0..2 {
        for manual in [false, true] {
            let mut app = app(index, manual);
            water(&mut app);
            app.world_mut().resource_mut::<MapData>().upper[45] = 10001;
            assert_step(&mut app, index, false);
            app.world_mut().resource_mut::<MapData>().passages_up[1] = ABOVE_HERO_BIT;
            assert_step(&mut app, index, true);
        }
    }
}

#[test]
fn boats_and_ships_collide_with_same_layer_events_but_airships_fly_over_them() {
    for index in 0..3 {
        for manual in [false, true] {
            let mut app = app(index, manual);
            water(&mut app);
            let mut event = crate::world::test_support::event(
                1,
                5,
                vec![crate::world::test_support::page(vec![])],
            );
            event.y = 4;
            app.world_mut()
                .resource_mut::<MapEvents>()
                .events
                .push(event);
            assert_step(&mut app, index, index == 2);
        }
    }
}

#[test]
fn an_unoccupied_vehicle_seeks_the_actual_hero_instead_of_its_own_tile() {
    let mut app = app(2, false);
    app.world_mut()
        .resource_mut::<Vehicles>()
        .set_route(10004, RouteStepper::from_move_event(&[10004, 8, 0, 0, 9]));
    let world = app.world_mut();
    world
        .query::<&mut Player>()
        .single_mut(world)
        .unwrap()
        .set_tile(8, 4);
    assert_step(&mut app, 2, true);
}

#[test]
fn vehicle_seek_and_flee_choose_the_shortest_wrapped_distance_to_the_hero() {
    for (command, expected) in [(9, 0), (10, 8)] {
        let mut app = app(2, false);
        app.world_mut().resource_mut::<MapData>().scroll_type = 2;
        let world = app.world_mut();
        world
            .query::<&mut Player>()
            .single_mut(world)
            .unwrap()
            .set_tile(0, 4);
        let mut vehicles = world.resource_mut::<Vehicles>();
        vehicles.set_location(2, 0, 9, 4);
        vehicles.set_route(
            10004,
            RouteStepper::from_move_event(&[10004, 8, 0, 0, command]),
        );
        app.update();
        assert_eq!(
            app.world().resource::<Vehicles>().save.vehicles[2].tile(),
            (expected, 4)
        );
    }
}

use super::*;
use crate::world::test_support::{event, page};

fn obstacle(id: u32, layer: u32, tile: Option<u32>) -> amnezia_data::Event {
    let mut page = page(vec![]);
    page.layer = layer;
    page.overlap_forbidden = true;
    if let Some(tile) = tile {
        page.graphic_name.clear();
        page.graphic_index = tile;
    }
    let mut event = event(id, 5, vec![page]);
    event.y = 4;
    event
}

#[test]
fn vehicles_ignore_source_passage_and_water_vehicles_ignore_lower_geometry() {
    for index in 0..3 {
        for manual in [false, true] {
            let mut app = app(index, manual);
            water(&mut app);
            let mut map = app.world_mut().resource_mut::<MapData>();
            map.upper[44] = 10001;
            map.passages_up[1] = 0;
            map.passages_up[0] = ABOVE_HERO_BIT;
            map.passages_down.fill(0);
            assert_step(&mut app, index, true);
        }
    }
}

#[test]
fn water_vehicles_use_the_highest_active_below_tile_without_event_overlap_flags() {
    for index in 0..2 {
        for winning in [None, Some(0), Some(1), Some(2)] {
            let mut app = app(index, false);
            water(&mut app);
            app.world_mut().resource_mut::<MapData>().passages_up[2] = ABOVE_HERO_BIT;
            let mut events = app.world_mut().resource_mut::<MapEvents>();
            events.events.push(obstacle(2, 0, Some(1)));
            events.events.push(obstacle(9, 0, winning));
            assert_step(&mut app, index, matches!(winning, Some(0 | 2)));
        }
        let mut app = app(index, false);
        water(&mut app);
        app.world_mut()
            .resource_mut::<MapEvents>()
            .events
            .push(obstacle(1, 0, None));
        assert_step(&mut app, index, true);
    }
}

#[test]
fn missing_vehicle_terrain_is_blocked_but_live_route_through_bypasses_it() {
    for index in 0..3 {
        let mut app = app(index, false);
        water(&mut app);
        app.world_mut().resource_mut::<MapData>().terrains.clear();
        assert_step(&mut app, index, false);
        app.world_mut().resource_mut::<Vehicles>().set_route(
            10002 + index as i32,
            RouteStepper::from_move_event(&[10002 + index as i32, 8, 0, 0, 36, 1]),
        );
        app.world_mut()
            .resource_mut::<MapEvents>()
            .events
            .push(obstacle(1, 1, None));
        assert_step(&mut app, index, true);
    }
}

#[test]
fn vehicle_through_still_obeys_bounds_and_wraps_looping_destinations() {
    for index in 0..3 {
        for looping in [false, true] {
            let mut app = app(index, false);
            app.world_mut().resource_mut::<MapData>().scroll_type = if looping { 2 } else { 0 };
            let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
            vehicles.set_location(index, 0, 9, 4);
            vehicles.set_route(
                10002 + index as i32,
                RouteStepper::from_move_event(&[10002 + index as i32, 8, 0, 0, 36, 1]),
            );
            app.update();
            assert_eq!(
                app.world().resource::<Vehicles>().save.vehicles[index].tile(),
                (if looping { 0 } else { 9 }, 4)
            );
        }
    }
}

#[test]
fn only_water_vehicles_collide_with_the_unmounted_hero_and_other_vehicles() {
    for index in 0..3 {
        for other in 0..4 {
            if other == index {
                continue;
            }
            let mut app = app(index, false);
            water(&mut app);
            if other == 3 {
                let world = app.world_mut();
                world
                    .query::<&mut Player>()
                    .single_mut(world)
                    .unwrap()
                    .set_tile(5, 4);
            } else {
                app.world_mut()
                    .resource_mut::<Vehicles>()
                    .set_location(other, 0, 5, 4);
            }
            assert_step(&mut app, index, index == 2);
        }
    }
}

#[test]
fn diagonal_water_routes_need_a_complete_orthogonal_path_but_jumps_check_only_landing() {
    for index in 0..2 {
        for jumping in [false, true] {
            let mut app = app(index, false);
            water(&mut app);
            let world = app.world_mut();
            world
                .query::<&mut Player>()
                .single_mut(world)
                .unwrap()
                .set_tile(8, 8);
            let mut map = world.resource_mut::<MapData>();
            map.upper[45] = 10001;
            map.upper[54] = 10001;
            let mut route = vec![10002 + index as i32, 8, 0, 0];
            route.extend(if jumping { vec![24, 5, 25] } else { vec![5] });
            app.world_mut()
                .resource_mut::<Vehicles>()
                .set_route(10002 + index as i32, RouteStepper::from_move_event(&route));
            app.update();
            assert_eq!(
                app.world().resource::<Vehicles>().save.vehicles[index].tile(),
                if jumping { (5, 5) } else { (4, 4) }
            );
        }
    }
}

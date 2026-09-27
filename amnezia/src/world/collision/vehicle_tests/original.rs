use super::*;
use crate::assets::{asset_root, load_ron};
use amnezia_data::{Chipset, Map};

#[test]
fn original_world_coast_rejects_the_boat_while_beach_allows_both_water_vehicles() {
    let original = load_ron::<Map>(&format!("{}/maps/map_0013.ron", asset_root()));
    let chipset = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()))
        .into_iter()
        .find(|chipset| chipset.id == original.chipset_id)
        .unwrap();
    for (tile, terrain, permissions) in [
        ((0, 0), 10, [false, true, true]),
        ((129, 4), 9, [true, true, true]),
        ((11, 7), 1, [false, false, true]),
    ] {
        for (index, allowed) in permissions.into_iter().enumerate() {
            let mut app = app(index, false);
            let mut map = MapData::for_test(original.width as i32, original.height as i32);
            map.map_id = 13;
            map.scroll_type = original.scroll_type;
            map.lower = original.lower.clone();
            map.upper = original.upper.clone();
            map.passages_down = chipset.passages_down.clone();
            map.passages_up = chipset.passages_up.clone();
            map.terrain_data = chipset.terrain_data.clone();
            map.terrains = load_ron(&format!("{}/terrains.ron", asset_root()));
            assert_eq!(map.terrain_at(tile.0, tile.1).unwrap().id, terrain);
            let origin = map.normalize_tile(tile.0 - 1, tile.1);
            app.insert_resource(map);
            let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
            vehicles.set_location(index, 13, origin.0 as u32, origin.1 as u32);
            vehicles.set_route(
                10002 + index as i32,
                RouteStepper::from_move_event(&[10002 + index as i32, 8, 0, 0, 1]),
            );
            app.update();
            assert_eq!(
                app.world().resource::<Vehicles>().save.vehicles[index].tile(),
                if allowed { tile } else { origin },
                "vehicle {index}, terrain {terrain}"
            );
        }
    }
}

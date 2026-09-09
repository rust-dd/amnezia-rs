use super::*;
use crate::assets::{asset_root, load_ron};
use amnezia_data::{Chipset, Map};

fn original_map(id: u32) -> MapData {
    let map = load_ron::<Map>(&format!("{}/maps/map_{id:04}.ron", asset_root()));
    let chips = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()));
    let chip = chips
        .into_iter()
        .find(|chip| chip.id == map.chipset_id)
        .unwrap();
    let mut data = MapData::for_test(map.width as i32, map.height as i32);
    data.map_id = id;
    data.scroll_type = map.scroll_type;
    data.lower = map.lower;
    data.upper = map.upper;
    data.passages_down = chip.passages_down;
    data.passages_up = chip.passages_up;
    data.terrain_data = chip.terrain_data;
    data.terrains = load_ron(&format!("{}/terrains.ron", asset_root()));
    data
}

#[test]
fn original_world_terrains_allow_flight_but_restrict_landing() {
    for (id, samples) in [
        (
            13,
            vec![
                (0, 0, 10, false),
                (129, 4, 9, false),
                (11, 7, 1, true),
                (11, 8, 2, false),
                (91, 59, 3, true),
            ],
        ),
        (220, vec![(0, 0, 2, false), (1, 0, 1, true)]),
        (222, vec![(0, 0, 1, true), (7, 0, 2, false)]),
    ] {
        let data = original_map(id);
        for (x, y, terrain_id, land) in samples {
            assert_eq!(data.terrain_at(x, y).unwrap().id, terrain_id);
            assert!(data.airship_passable(x, y));
            assert_eq!(
                data.airship_landing_tile(x, y),
                land,
                "map {id}, ({x}, {y})"
            );
        }
    }
}

#[test]
fn terrain_lookup_uses_lower_tile_families_and_wraps_before_indexing() {
    let mut data = MapData::for_test(5, 1);
    data.terrain_data = (1..=162).collect();
    data.terrains = (1..=162)
        .map(|id| TerrainDef {
            id,
            ..Default::default()
        })
        .collect();
    data.lower = vec![2000, 3100, 4100, 5000, 5143];
    data.upper = vec![10143; 5];
    for (x, tag) in [3, 6, 9, 19, 162].into_iter().enumerate() {
        assert_eq!(data.terrain_at(x as i32, 0).unwrap().id, tag);
    }
    assert_eq!(data.terrain_at(-1, 0).unwrap().id, 1);
    data.scroll_type = 2;
    assert_eq!(data.terrain_at(-1, 0).unwrap().id, 162);
    data.terrain_data.clear();
    assert_eq!(data.terrain_at(0, 0).unwrap().id, 1);
}

#[test]
fn landing_checks_both_geometry_layers_instead_of_upper_tile_override() {
    let mut data = MapData::for_test(2, 2);
    data.passages_down[0] = 0;
    data.upper[0] = 10001;
    assert!(data.passable(0, 0));
    assert!(!data.airship_landing_tile(0, 0));
    data.passages_down[0] = PASS_ALL;
    assert!(data.airship_landing_tile(0, 0));
    data.passages_up[1] = 0;
    assert!(!data.airship_landing_tile(0, 0));
    data.upper[0] = 10000;
    data.passages_up[0] = 0;
    assert!(!data.airship_landing_tile(0, 0));
}

#[test]
fn missing_terrain_and_explicit_flight_bans_are_not_treated_as_grass() {
    let mut data = MapData::for_test(2, 2);
    data.terrain_data = vec![11; 162];
    assert!(data.terrain_at(0, 0).is_none());
    assert!(!data.airship_passable(0, 0));
    assert!(!data.airship_landing_tile(0, 0));
    data.terrains.push(TerrainDef {
        id: 11,
        airship_pass: false,
        ..Default::default()
    });
    assert!(!data.airship_passable(0, 0));
    assert!(data.airship_landing_tile(0, 0));
    assert!(!data.airship_passable(-1, 0));
    assert!(!data.airship_landing_tile(-1, 0));
}

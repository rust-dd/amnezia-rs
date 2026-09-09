use super::RouteStepper;
use crate::assets::{asset_root, load_ron};
use crate::player::Player;
use amnezia_data::Map;
use std::path::Path;

#[test]
fn original_custom_routes_keep_both_repeating_and_one_shot_flags() {
    let mut repeating = 0;
    let mut one_shot = 0;
    for entry in std::fs::read_dir(Path::new(asset_root()).join("maps")).unwrap() {
        let path = entry.unwrap().path();
        let map = load_ron::<Map>(path.to_str().unwrap());
        for event in map.events {
            for page in event.pages {
                if page.move_type == 6 && !page.move_route.is_empty() {
                    if page.move_route.repeat {
                        repeating += 1;
                    } else {
                        one_shot += 1;
                    }
                }
            }
        }
    }
    assert_eq!((repeating, one_shot), (35, 38));
}

#[test]
fn original_starting_village_dog_keeps_pacing_after_first_lap() {
    let map = load_ron::<Map>(&format!("{}/maps/map_0001.ron", asset_root()));
    let event = map.events.iter().find(|e| e.id == 48).unwrap();
    for page in &event.pages[..2] {
        let mut route =
            RouteStepper::from_page(&page.move_route, page.move_speed, page.move_frequency);
        let mut player = Player {
            tile_x: event.x as i32,
            tile_y: event.y as i32,
            dir: page.direction,
            frame: page.pattern,
            charset: page.graphic_name.clone(),
            index: page.graphic_index,
        };
        let mut effects = Vec::new();
        for _ in 0..3 {
            for dx in [1, -1] {
                let (action, _) = route
                    .advance(&mut player, (0, 0), &|_, _, _| true, &mut effects)
                    .unwrap();
                assert_eq!(action.delta(), (dx, 0));
            }
        }
        assert!(route.active());
        assert!(!route.forced());
    }
}

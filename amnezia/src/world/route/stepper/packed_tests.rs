use super::{RouteStepper, StepEffect};
use crate::assets::{asset_root, load_ron};
use crate::player::Player;
use amnezia_data::Map;
use std::path::Path;

fn campaign_route(map: u32, event: u32, page: usize, tail: &[i32]) -> RouteStepper {
    let map = load_ron::<Map>(&format!("{}/maps/map_{map:04}.ron", asset_root()));
    let event = map.events.iter().find(|e| e.id == event).unwrap();
    let command = event.pages[page - 1]
        .commands
        .iter()
        .find(|c| c.code == 11330 && c.params.ends_with(tail))
        .unwrap();
    RouteStepper::from_move_event(&command.params)
}

fn finish(route: &mut RouteStepper) -> Vec<StepEffect> {
    let mut player = Player {
        tile_x: 0,
        tile_y: 0,
        dir: 2,
        frame: 1,
        charset: String::new(),
        index: 0,
    };
    let mut effects = Vec::new();
    for _ in 0..=route.commands.len() {
        route.advance(&mut player, (0, 0), &|_, _, _| true, &mut effects);
    }
    assert!(!route.active());
    effects
}

#[test]
fn original_cutscene_routes_toggle_full_switch_ids() {
    for (map, event, tail, expected) in [(66, 16, [32, 129, 42], 170), (46, 59, [32, 132, 81], 593)]
    {
        let mut route = campaign_route(map, event, 1, &tail);
        assert!(route.commands.iter().all(|c| c.code <= 41));
        assert!(
            matches!(finish(&mut route).as_slice(), [StepEffect::Switch(id, true)] if *id == expected)
        );
    }
}

#[test]
fn original_movement_sound_keeps_tempo_balance_and_following_graphic() {
    let mut route = campaign_route(23, 8, 2, &[34, 5, 84, 111, 114, 99, 104, 1, 1]);
    assert_eq!(route.commands.len(), 6);
    assert_eq!(route.commands[4].string, "Torch");
    assert_eq!(route.commands[4].params, [1]);
    assert!(
        matches!(finish(&mut route).as_slice(), [StepEffect::Sound { name, params }] if name == "Movement" && *params == [80, 140, 50])
    );
}

#[test]
fn packed_switch_off_and_large_numbers_preserve_command_boundaries() {
    let route = RouteStepper::from_move_event(&[0, 8, 0, 0, 33, 129, 128, 0, 23]);
    assert_eq!(route.commands.len(), 2);
    assert_eq!(route.commands[0].params, [16384]);
    assert_eq!(route.commands[1].code, 23);
}

#[test]
fn packed_names_use_cp1250_and_variable_length_lengths() {
    let route = RouteStepper::from_move_event(&[0, 8, 0, 0, 34, 3, 193, 114, 237, 129, 0]);
    assert_eq!(route.commands.len(), 1);
    assert_eq!(route.commands[0].string, "Árí");
    assert_eq!(route.commands[0].params, [128]);

    let mut params = vec![0, 8, 0, 0, 34, 129, 0];
    params.extend(std::iter::repeat_n(65, 128));
    params.extend([7, 23]);
    let route = RouteStepper::from_move_event(&params);
    assert_eq!(route.commands.len(), 2);
    assert_eq!(route.commands[0].string, "A".repeat(128));
    assert_eq!(route.commands[0].params, [7]);
    assert_eq!(route.commands[1].code, 23);
}

#[test]
fn malformed_packed_routes_do_not_panic_or_execute_a_partial_route() {
    for tail in [
        vec![32],
        vec![32, 129],
        vec![34],
        vec![34, 5, 65],
        vec![35, 0, 80, 100],
        vec![32, 256],
        vec![34, 1, -1, 0],
        vec![32, 255, 255, 255, 255, 127],
        vec![32, 129, 128, 128, 128, 128, 0],
    ] {
        let mut params = vec![0, 8, 0, 0, 32, 1];
        params.extend(tail);
        let route = RouteStepper::from_move_event(&params);
        assert!(route.commands.is_empty(), "{params:?}");
        assert!(!route.active());
    }
}

fn packed_int(value: i32) -> Vec<i32> {
    let mut value = value as u32;
    let mut bytes = vec![(value & 127) as i32];
    value >>= 7;
    while value != 0 {
        bytes.push(((value & 127) | 128) as i32);
        value >>= 7;
    }
    bytes.reverse();
    bytes
}

#[test]
fn all_campaign_forced_routes_round_trip_without_phantom_commands() {
    let mut count = 0;
    for entry in std::fs::read_dir(Path::new(asset_root()).join("maps")).unwrap() {
        let path = entry.unwrap().path();
        let map = load_ron::<Map>(path.to_str().unwrap());
        for event in map.events {
            for page in event.pages {
                for command in page.commands.iter().filter(|c| c.code == 11330) {
                    count += 1;
                    let route = RouteStepper::from_move_event(&command.params);
                    let mut packed = Vec::new();
                    for step in route.commands {
                        assert!(step.code <= 41, "{} / event {}", path.display(), event.id);
                        packed.push(step.code as i32);
                        if matches!(step.code, 34 | 35) {
                            let (bytes, _, errors) = encoding_rs::WINDOWS_1250.encode(&step.string);
                            assert!(!errors);
                            packed.extend(packed_int(bytes.len() as i32));
                            packed.extend(bytes.iter().map(|&b| i32::from(b)));
                        }
                        for arg in step.params {
                            packed.extend(packed_int(arg));
                        }
                    }
                    assert_eq!(
                        packed,
                        command.params[4..],
                        "{} / event {}",
                        path.display(),
                        event.id
                    );
                }
            }
        }
    }
    assert_eq!(count, 5818);
}

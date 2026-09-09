use crate::assets::{asset_root, load_ron, resolve_png};
use amnezia_data::{Chipset, CommonEvent, EventCommand, Map, Start, TroopDef};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod enemy_actions;
mod state_resistance;
mod troop_events;

fn maps() -> BTreeMap<u32, Map> {
    std::fs::read_dir(Path::new(asset_root()).join("maps"))
        .unwrap()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let id = path
                .file_stem()?
                .to_str()?
                .strip_prefix("map_")?
                .parse::<u32>()
                .ok()?;
            Some((id, load_ron(path.to_str().unwrap())))
        })
        .collect()
}

fn graphic_exists(category: &str, name: &str, context: &str) {
    if name.is_empty() {
        return;
    }
    let path = Path::new(asset_root()).join(resolve_png(category, name));
    assert!(path.is_file(), "{context}: missing {}", path.display());
}

#[test]
fn all_campaign_maps_and_page_graphics_are_available() {
    let maps = maps();
    assert_eq!(maps.len(), 276);
    let chipsets = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()));
    for (id, map) in &maps {
        let context = format!("map {id}");
        assert_eq!(
            map.lower.len(),
            (map.width * map.height) as usize,
            "{context}"
        );
        assert_eq!(map.upper.len(), map.lower.len(), "{context}");
        let chipset = chipsets
            .iter()
            .find(|c| c.id == map.chipset_id)
            .expect(&context);
        graphic_exists("ChipSet", &chipset.graphic, &context);
        if let Some(panorama) = &map.panorama {
            graphic_exists("Panorama", &panorama.name, &context);
        }
        for event in &map.events {
            for page in &event.pages {
                graphic_exists("CharSet", &page.graphic_name, &context);
            }
        }
    }
    let start = load_ron::<Start>(&format!("{}/start.ron", asset_root()));
    destination(
        &maps,
        start.map_id as i32,
        start.x as i32,
        start.y as i32,
        "new game",
    );
}

fn destination(maps: &BTreeMap<u32, Map>, id: i32, x: i32, y: i32, context: &str) {
    let map = maps
        .get(&(id as u32))
        .unwrap_or_else(|| panic!("{context}: missing map {id}"));
    assert!(
        x >= 0 && y >= 0 && x < map.width as i32 && y < map.height as i32,
        "{context}: destination ({id}, {x}, {y}) outside {} x {}",
        map.width,
        map.height,
    );
}

#[test]
fn campaign_commands_have_known_opcodes_and_valid_static_references() {
    let maps = maps();
    let common = load_ron::<Vec<CommonEvent>>(&format!("{}/common_events.ron", asset_root()));
    let troops = load_ron::<Vec<TroopDef>>(&format!("{}/troops.ron", asset_root()));
    let known = include_str!("interpreter/opcodes.rs")
        .lines()
        .filter(|line| line.starts_with("pub(super) const "))
        .filter_map(|line| {
            line.split('=')
                .nth(1)?
                .trim()
                .trim_end_matches(';')
                .parse::<u32>()
                .ok()
        })
        .chain([0])
        .collect::<BTreeSet<_>>();
    let mut checked = 0;
    let mut check = |command: &EventCommand, context: &str| {
        checked += 1;
        assert!(
            known.contains(&command.code),
            "{context}: unknown opcode {}",
            command.code
        );
        match command.code {
            10810 => {
                let [map, x, y, ..] = command.params.as_slice() else {
                    panic!("{context}: incomplete teleport");
                };
                destination(&maps, *map, *x, *y, context);
            }
            10710 if command.params.first() == Some(&0) => {
                let id = command.params[1] as u32;
                assert!(
                    troops.iter().any(|t| t.id == id),
                    "{context}: missing troop {id}"
                );
            }
            10130 => graphic_exists("FaceSet", &command.string, context),
            10630 => graphic_exists("CharSet", &command.string, context),
            11110 => graphic_exists("Picture", &command.string, context),
            11720 => graphic_exists("Panorama", &command.string, context),
            _ => {}
        }
    };
    for (id, map) in &maps {
        for event in &map.events {
            for (page, definition) in event.pages.iter().enumerate() {
                let context = format!("map {id}, event {}, page {}", event.id, page + 1);
                for command in &definition.commands {
                    check(command, &context);
                }
            }
        }
    }
    for event in &common {
        for command in &event.commands {
            check(command, &format!("common event {}", event.id));
        }
    }
    assert!(checked > 10_000, "campaign scripts were not loaded");
}

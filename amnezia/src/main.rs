//! The Amnézia game: renders a converted map with its chipset. Depends only on
//! `amnezia-data` + Bevy; it never touches the legacy RPG Maker formats.

mod tiles;

use amnezia_data::{Chipset, Map};
use bevy::prelude::*;
use std::path::Path;

/// Absolute path to the converted assets, resolved at compile time so the game
/// runs regardless of the current working directory.
const ASSET_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

fn main() -> AppExit {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: ASSET_ROOT.to_string(),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .run()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let map: Map = load_ron(&format!("{ASSET_ROOT}/maps/map_0001.ron"));
    let chipsets: Vec<Chipset> = load_ron(&format!("{ASSET_ROOT}/chipsets.ron"));
    let graphic = chipsets
        .iter()
        .find(|c| c.id == map.chipset_id)
        .map(|c| c.graphic.clone())
        .unwrap_or_default();
    let chipset = asset_server.load(resolve_chipset_png(&graphic));

    let width = map.width as i32;
    let offset_x = map.width as f32 * tiles::TILE / 2.0;
    let offset_y = map.height as f32 * tiles::TILE / 2.0;

    for (index, &id) in map.lower.iter().enumerate() {
        let (source_x, source_y) = tiles::lower_source(id);
        spawn_tile(
            &mut commands, &chipset, source_x, source_y, index as i32, width, offset_x, offset_y, 0.0,
        );
    }
    for (index, &id) in map.upper.iter().enumerate() {
        if let Some((source_x, source_y)) = tiles::upper_source(id) {
            spawn_tile(
                &mut commands, &chipset, source_x, source_y, index as i32, width, offset_x, offset_y,
                1.0,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_tile(
    commands: &mut Commands,
    chipset: &Handle<Image>,
    source_x: f32,
    source_y: f32,
    index: i32,
    width: i32,
    offset_x: f32,
    offset_y: f32,
    z: f32,
) {
    let tile_x = index % width;
    let tile_y = index / width;
    let world_x = tile_x as f32 * tiles::TILE - offset_x + tiles::TILE / 2.0;
    let world_y = offset_y - tile_y as f32 * tiles::TILE - tiles::TILE / 2.0;
    commands.spawn((
        Sprite {
            image: chipset.clone(),
            rect: Some(Rect::new(
                source_x,
                source_y,
                source_x + tiles::TILE,
                source_y + tiles::TILE,
            )),
            custom_size: Some(Vec2::splat(tiles::TILE)),
            ..default()
        },
        Transform::from_xyz(world_x, world_y, z),
    ));
}

fn load_ron<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    ron::from_str(&text).unwrap_or_else(|e| panic!("parsing {path}: {e}"))
}

/// Resolve a chipset graphic name to its PNG path relative to the asset root,
/// matching the on-disk filename case-insensitively.
fn resolve_chipset_png(graphic: &str) -> String {
    let dir = format!("{ASSET_ROOT}/graphics/ChipSet");
    let target = format!("{}.png", graphic.to_lowercase());
    if let Ok(entries) = std::fs::read_dir(Path::new(&dir)) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.to_lowercase() == target {
                return format!("graphics/ChipSet/{name}");
            }
        }
    }
    format!("graphics/ChipSet/{graphic}.png")
}

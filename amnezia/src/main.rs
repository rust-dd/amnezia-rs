//! The Amnézia game: renders a converted map with its chipset and lets the
//! player walk the hero around it, blocked by tile passability. Depends only on
//! `amnezia-data` + Bevy; it never touches the legacy RPG Maker formats.

mod tiles;

use amnezia_data::{Chipset, Map};
use bevy::prelude::*;
use std::path::Path;

/// Absolute path to the converted assets, resolved at compile time so the game
/// runs regardless of the current working directory.
const ASSET_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

const PLAYER_CHARSET: &str = "Chara1";
const PLAYER_INDEX: u32 = 0;
const DIR_UP: u32 = 0;
const DIR_RIGHT: u32 = 1;
const DIR_DOWN: u32 = 2;
const DIR_LEFT: u32 = 3;

/// Vertical offset so the 24×32 character's feet sit on the tile it occupies
/// (RM2000 aligns the sprite's bottom with the tile's bottom).
const PLAYER_Y_OFFSET: f32 = (tiles::CHAR_H - tiles::TILE) / 2.0;

#[derive(Component)]
struct Player {
    tile_x: i32,
    tile_y: i32,
    dir: u32,
    frame: u32,
}

#[derive(Resource)]
struct MapData {
    width: i32,
    height: i32,
    offset_x: f32,
    offset_y: f32,
    lower: Vec<u16>,
    upper: Vec<u16>,
    passages_down: Vec<u8>,
    passages_up: Vec<u8>,
}

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
        .add_systems(Update, (move_player, update_player_sprite, camera_follow).chain())
        .run()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let map: Map = load_ron(&format!("{ASSET_ROOT}/maps/map_0001.ron"));
    let chipsets: Vec<Chipset> = load_ron(&format!("{ASSET_ROOT}/chipsets.ron"));
    let entry = chipsets.into_iter().find(|c| c.id == map.chipset_id);
    let (graphic, passages_down, passages_up) = match entry {
        Some(c) => (c.graphic, c.passages_down, c.passages_up),
        None => (String::new(), vec![0x0F; 162], vec![0x0F; 144]),
    };
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

    let start_x = width / 2;
    let start_y = map.height as i32 / 2;
    let player_charset = asset_server.load(format!("graphics/CharSet/{PLAYER_CHARSET}.png"));
    let (source_x, source_y) = tiles::charset_source(PLAYER_INDEX, DIR_DOWN, 1);
    let world_x = start_x as f32 * tiles::TILE - offset_x + tiles::TILE / 2.0;
    let world_y = offset_y - start_y as f32 * tiles::TILE - tiles::TILE / 2.0 + PLAYER_Y_OFFSET;
    commands.spawn((
        Player { tile_x: start_x, tile_y: start_y, dir: DIR_DOWN, frame: 1 },
        Sprite {
            image: player_charset,
            rect: Some(Rect::new(
                source_x,
                source_y,
                source_x + tiles::CHAR_W,
                source_y + tiles::CHAR_H,
            )),
            custom_size: Some(Vec2::new(tiles::CHAR_W, tiles::CHAR_H)),
            ..default()
        },
        Transform::from_xyz(world_x, world_y, 2.0),
    ));

    commands.insert_resource(MapData {
        width,
        height: map.height as i32,
        offset_x,
        offset_y,
        lower: map.lower,
        upper: map.upper,
        passages_down,
        passages_up,
    });
}

fn tile_center(tile_x: i32, tile_y: i32, data: &MapData) -> (f32, f32) {
    let x = tile_x as f32 * tiles::TILE - data.offset_x + tiles::TILE / 2.0;
    let y = data.offset_y - tile_y as f32 * tiles::TILE - tiles::TILE / 2.0;
    (x, y)
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

fn move_player(keys: Res<ButtonInput<KeyCode>>, data: Res<MapData>, mut players: Query<&mut Player>) {
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    let step = if keys.just_pressed(KeyCode::ArrowUp) {
        Some((0, -1, DIR_UP))
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        Some((0, 1, DIR_DOWN))
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        Some((-1, 0, DIR_LEFT))
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some((1, 0, DIR_RIGHT))
    } else {
        None
    };
    if let Some((dx, dy, dir)) = step {
        player.dir = dir;
        let (nx, ny) = (player.tile_x + dx, player.tile_y + dy);
        if nx >= 0 && ny >= 0 && nx < data.width && ny < data.height {
            let idx = (ny * data.width + nx) as usize;
            if tiles::passable(data.lower[idx], data.upper[idx], &data.passages_down, &data.passages_up) {
                player.tile_x = nx;
                player.tile_y = ny;
                player.frame = (player.frame + 1) % 3;
            }
        }
    }
}

fn update_player_sprite(
    data: Res<MapData>,
    mut players: Query<(&Player, &mut Sprite, &mut Transform), Changed<Player>>,
) {
    for (player, mut sprite, mut transform) in &mut players {
        let (source_x, source_y) = tiles::charset_source(PLAYER_INDEX, player.dir, player.frame);
        sprite.rect = Some(Rect::new(
            source_x,
            source_y,
            source_x + tiles::CHAR_W,
            source_y + tiles::CHAR_H,
        ));
        let (world_x, world_y) = tile_center(player.tile_x, player.tile_y, &data);
        transform.translation.x = world_x;
        transform.translation.y = world_y + PLAYER_Y_OFFSET;
    }
}

fn camera_follow(
    players: Query<&Transform, With<Player>>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    camera.translation.x = player.translation.x;
    camera.translation.y = player.translation.y;
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

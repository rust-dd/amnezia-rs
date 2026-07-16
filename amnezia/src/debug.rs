//! A developer HUD and passability overlay for diagnosing movement and events.
//! The HUD (top-left) always shows the current map, the hero's tile and facing,
//! and whether an event is running; pressing `P` toggles a red overlay marking
//! every impassable tile of the current map (re-toggle after a map change).

use crate::font::GameFont;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::tiles;
use crate::world::MapData;
use bevy::prelude::*;
use bevy::text::FontSource;

#[derive(Component)]
struct DebugHud;

#[derive(Component)]
struct PassOverlay;

#[derive(Resource, Default)]
struct OverlayShown(bool);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlayShown>()
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, (update_hud, toggle_overlay));
    }
}

fn spawn_hud(mut commands: Commands, font: Res<GameFont>) {
    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font: FontSource::Handle(font.0.clone()),
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(6.0),
            top: Val::Px(6.0),
            ..default()
        },
        GlobalZIndex(2000),
        DebugHud,
    ));
}

fn update_hud(
    data: Option<Res<MapData>>,
    running: Res<RunningEvent>,
    players: Query<&Player>,
    mut hud: Query<&mut Text, With<DebugHud>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let map = data.map(|d| d.map_id).unwrap_or(0);
    let (px, py, dir) = match players.single() {
        Ok(p) => (p.tile_x, p.tile_y, p.dir),
        Err(_) => (0, 0, 0),
    };
    let run = match running.debug_id() {
        Some(id) => format!("event {id}"),
        None => "idle".to_string(),
    };
    **text = format!("map {map}  tile ({px},{py}) dir {dir}  run: {run}  [P: walls]");
}

fn toggle_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut shown: ResMut<OverlayShown>,
    mut commands: Commands,
    data: Option<Res<MapData>>,
    existing: Query<Entity, With<PassOverlay>>,
) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }
    shown.0 = !shown.0;
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if !shown.0 {
        return;
    }
    let Some(data) = data else {
        return;
    };
    for y in 0..data.height {
        for x in 0..data.width {
            if data.passable(x, y) {
                continue;
            }
            let (wx, wy) = data.tile_center(x, y);
            commands.spawn((
                Sprite {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.35),
                    custom_size: Some(Vec2::splat(tiles::TILE)),
                    ..default()
                },
                Transform::from_xyz(wx, wy, 5.0),
                PassOverlay,
            ));
        }
    }
}

//! Opt-in developer diagnostics, excluded from release gameplay.

use crate::font::GameFont;
use crate::interpreter::{ParallelPool, RunningEvent};
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

pub(crate) fn tools_enabled() -> bool {
    cfg!(test) || (cfg!(debug_assertions) && std::env::args().any(|arg| arg == "--debug-tools"))
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if !cfg!(debug_assertions) {
            return;
        }
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
        Visibility::Hidden,
    ));
}

fn update_hud(
    keys: Res<ButtonInput<KeyCode>>,
    data: Option<Res<MapData>>,
    running: Res<RunningEvent>,
    parallel: Res<ParallelPool>,
    players: Query<&Player>,
    mut hud: Query<(&mut Text, &mut Visibility), With<DebugHud>>,
) {
    let Ok((mut text, mut visibility)) = hud.single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::F3) {
        *visibility = if *visibility == Visibility::Hidden {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if *visibility == Visibility::Hidden {
        return;
    }
    let map = data.map(|d| d.map_id).unwrap_or(0);
    let (px, py, dir) = match players.single() {
        Ok(p) => (p.tile_x, p.tile_y, p.dir),
        Err(_) => (0, 0, 0),
    };
    let run = match running.debug_id() {
        Some(id) => format!("event {id}"),
        None => "idle".to_string(),
    };
    let par = parallel.count();
    **text = format!("map {map}  tile ({px},{py}) dir {dir}  run: {run}  par: {par}  [P: walls]");
}

fn toggle_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut shown: ResMut<OverlayShown>,
    mut commands: Commands,
    data: Option<Res<MapData>>,
    existing: Query<Entity, With<PassOverlay>>,
) {
    if !tools_enabled() || !keys.just_pressed(KeyCode::KeyP) {
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
                // Above every world draw band (tiles and characters) so the
                // impassable-tile overlay stays visible on top.
                Transform::from_xyz(wx, wy, 100.0),
                PassOverlay,
            ));
        }
    }
}

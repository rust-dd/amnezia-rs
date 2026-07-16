//! Map transfer with a fade: a teleport request fades the screen to black,
//! swaps the map at the black peak (despawn scene, load destination, reposition
//! the persistent hero), then fades back in. Movement and interaction are
//! suppressed while a fade is in progress.

use crate::player::Player;
use crate::state::{Switches, Variables};
use crate::tiles::CHAR_Y_OFFSET;
use crate::world::{load_map, MapData, MapEvents, MapScene};
use bevy::prelude::*;

/// Screen fades per second (a full fade-out or fade-in takes 1/this seconds).
const FADE_SPEED: f32 = 4.0;

/// A pending teleport `(map_id, x, y)`, set by an interaction or a touch, and
/// picked up by the fade. At most one is queued at a time.
#[derive(Resource, Default)]
pub struct PendingTeleport(pub Option<(u32, u32, u32)>);

#[derive(PartialEq, Eq, Clone, Copy)]
enum Phase {
    Idle,
    Out,
    In,
}

/// The teleport fade state machine.
#[derive(Resource)]
pub struct Fade {
    phase: Phase,
    alpha: f32,
    target: Option<(u32, u32, u32)>,
}

impl Fade {
    /// Whether a teleport fade is in progress (movement/interaction paused).
    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }
}

impl Default for Fade {
    fn default() -> Self {
        Self { phase: Phase::Idle, alpha: 0.0, target: None }
    }
}

#[derive(Component)]
struct FadeOverlay;

pub struct TeleportPlugin;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingTeleport>()
            .init_resource::<Fade>()
            .add_systems(Startup, spawn_overlay)
            .add_systems(Update, drive_fade);
    }
}

/// A full-screen black overlay whose alpha the fade drives; on top of everything.
fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        GlobalZIndex(1000),
        FadeOverlay,
    ));
}

#[allow(clippy::too_many_arguments)]
fn drive_fade(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    mut pending: ResMut<PendingTeleport>,
    mut fade: ResMut<Fade>,
    mut map_data: ResMut<MapData>,
    mut map_events: ResMut<MapEvents>,
    scene: Query<Entity, With<MapScene>>,
    mut players: Query<(&mut Player, &mut Transform)>,
    mut overlay: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    if fade.phase == Phase::Idle
        && let Some(target) = pending.0.take()
    {
        fade.target = Some(target);
        fade.phase = Phase::Out;
    }
    let step = FADE_SPEED * time.delta_secs();
    match fade.phase {
        Phase::Out => {
            fade.alpha += step;
            if fade.alpha >= 1.0 {
                fade.alpha = 1.0;
                if let Some((map_id, x, y)) = fade.target.take() {
                    swap_map(
                        &mut commands, &asset_server, &switches, &variables, &mut map_data,
                        &mut map_events, &scene, &mut players, map_id, x, y,
                    );
                }
                fade.phase = Phase::In;
            }
        }
        Phase::In => {
            fade.alpha -= step;
            if fade.alpha <= 0.0 {
                fade.alpha = 0.0;
                fade.phase = Phase::Idle;
            }
        }
        Phase::Idle => {}
    }
    if let Ok(mut background) = overlay.single_mut() {
        background.0 = Color::srgba(0.0, 0.0, 0.0, fade.alpha);
    }
}

#[allow(clippy::too_many_arguments)]
fn swap_map(
    commands: &mut Commands,
    asset_server: &AssetServer,
    switches: &Switches,
    variables: &Variables,
    map_data: &mut MapData,
    map_events: &mut MapEvents,
    scene: &Query<Entity, With<MapScene>>,
    players: &mut Query<(&mut Player, &mut Transform)>,
    map_id: u32,
    x: u32,
    y: u32,
) {
    for entity in scene {
        commands.entity(entity).despawn();
    }
    let (data, events) = load_map(commands, asset_server, switches, variables, map_id);
    let (tile_x, tile_y) = (x as i32, y as i32);
    if let Ok((mut player, mut transform)) = players.single_mut() {
        player.tile_x = tile_x;
        player.tile_y = tile_y;
        let (world_x, world_y) = data.tile_center(tile_x, tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
    }
    *map_data = data;
    *map_events = events;
}

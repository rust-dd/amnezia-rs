//! Map transfer with a fade: a teleport request fades the screen to black,
//! swaps the map at the black peak (despawn scene, load destination, reposition
//! the persistent hero), then fades back in. Movement and interaction are
//! suppressed while a fade is in progress.

use crate::player::{CameraPan, Player};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::tiles::CHAR_Y_OFFSET;
use crate::world::{MapChanged, MapData, MapEvents, MapScene, MoveQueue, RouteStepper, load_map};
use bevy::prelude::*;

/// Frames (at 60 fps) each fade phase runs — RM2000's map-transfer transition
/// default (EasyRPG `Transition::GetDefaultFrames`), matching `screenfx::fade`'s
/// 35-frame screen fade. The previous 0.25 s fade was ~2.3× too fast.
const FADE_FRAMES: f32 = 35.0;

/// Overlay alpha added per second, so a full fade-out (or fade-in) spans
/// [`FADE_FRAMES`] frames.
const FADE_SPEED: f32 = 60.0 / FADE_FRAMES;

/// A pending teleport `(map_id, x, y)`, set by an interaction or a touch, and
/// picked up by the fade. At most one is queued at a time.
#[derive(Resource, Default)]
pub struct PendingTeleport(pub Option<(u32, u32, u32)>, bool);

impl PendingTeleport {
    /// Rebuild the destination even when a save or new game uses the current map.
    pub fn reload(&mut self, map_id: u32, x: u32, y: u32) {
        self.0 = Some((map_id, x, y));
        self.1 = true;
    }
}

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
    reload: bool,
}

impl Fade {
    /// Whether a teleport fade is in progress (movement/interaction paused).
    pub fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }
}

impl Default for Fade {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            alpha: 0.0,
            target: None,
            reload: false,
        }
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
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut pending: ResMut<PendingTeleport>,
    mut fade: ResMut<Fade>,
    mut map_data: ResMut<MapData>,
    mut map_events: ResMut<MapEvents>,
    mut pan: ResMut<CameraPan>,
    mut map_changed: MessageWriter<MapChanged>,
    scene: Query<Entity, With<MapScene>>,
    mut players: Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
    mut overlay: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    if fade.phase == Phase::Idle
        && let Some(target) = pending.0.take()
    {
        fade.target = Some(target);
        fade.reload = std::mem::take(&mut pending.1);
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
                        &mut commands,
                        &asset_server,
                        &switches,
                        &variables,
                        &party,
                        &inventory,
                        &mut map_data,
                        &mut map_events,
                        &mut pan,
                        &scene,
                        &mut players,
                        map_id,
                        x,
                        y,
                        fade.reload,
                    );
                    map_changed.write(MapChanged);
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
    party: &Party,
    inventory: &Inventory,
    map_data: &mut MapData,
    map_events: &mut MapEvents,
    pan: &mut CameraPan,
    scene: &Query<Entity, With<MapScene>>,
    players: &mut Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
    map_id: u32,
    x: u32,
    y: u32,
    reload: bool,
) {
    let (tile_x, tile_y) = (x as i32, y as i32);
    // A teleport whose destination is the current map (RM2000 same-map transfer)
    // keeps the loaded map, its events, and their state — only the hero moves.
    // Rebuilding the scene would reset every event's position and page state.
    if map_id == map_data.map_id && !reload {
        reposition_hero(players, map_data, tile_x, tile_y);
        pan.recenter(false);
        return;
    }
    for entity in scene {
        commands.entity(entity).despawn();
    }
    let (data, events) = load_map(
        commands,
        asset_server,
        switches,
        variables,
        party,
        inventory,
        map_id,
    );
    reposition_hero(players, &data, tile_x, tile_y);
    *map_data = data;
    *map_events = events;
    pan.recenter(true);
}

/// Move the persistent hero to tile `(tile_x, tile_y)` on `data`: update its
/// logical tile and snap its transform to the tile center. Facing is retained —
/// the teleport target carries no direction, matching RM2000's "retain heading".
fn reposition_hero(
    players: &mut Query<(
        &mut Player,
        &mut Transform,
        &mut MoveQueue,
        &mut RouteStepper,
    )>,
    data: &MapData,
    tile_x: i32,
    tile_y: i32,
) {
    if let Ok((mut player, mut transform, mut queue, mut route)) = players.single_mut() {
        *queue = MoveQueue::default();
        *route = RouteStepper::default();
        player.tile_x = tile_x;
        player.tile_y = tile_y;
        let (world_x, world_y) = data.tile_center(tile_x, tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
        transform.translation.z = crate::tiles::character_z(tile_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reposition the single hero to tile (4, 6) — the same-map teleport path.
    fn run_reposition(
        data: Res<MapData>,
        mut players: Query<(
            &mut Player,
            &mut Transform,
            &mut MoveQueue,
            &mut RouteStepper,
        )>,
    ) {
        reposition_hero(&mut players, &data, 4, 6);
    }

    /// A same-map teleport repositions the hero without tearing down the scene:
    /// `reposition_hero` moves the hero's tile and transform and never despawns
    /// the `MapScene` entities (it holds no `Commands`), so the map's generation
    /// is unchanged.
    #[test]
    fn same_map_reposition_moves_hero_and_keeps_scene() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(MapData::for_test(10, 10));
        // Three scene entities that a full reload would despawn.
        let scene: Vec<Entity> = (0..3)
            .map(|_| app.world_mut().spawn(MapScene).id())
            .collect();
        let hero = app
            .world_mut()
            .spawn((
                Player {
                    tile_x: 1,
                    tile_y: 1,
                    dir: crate::tiles::DIR_DOWN,
                    frame: 1,
                    charset: "Chara1".into(),
                    index: 0,
                },
                Transform::default(),
                MoveQueue::default(),
                RouteStepper::default(),
            ))
            .id();

        app.add_systems(Update, run_reposition);
        app.update();

        // The hero moved to the target tile and its transform snapped there.
        let player = app.world().entity(hero).get::<Player>().unwrap();
        assert_eq!((player.tile_x, player.tile_y), (4, 6));
        let (cx, cy) = app.world().resource::<MapData>().tile_center(4, 6);
        let transform = app.world().entity(hero).get::<Transform>().unwrap();
        assert_eq!(transform.translation.x, cx);
        assert_eq!(transform.translation.y, cy + CHAR_Y_OFFSET);
        // No scene entity was despawned — the same map is still standing.
        for entity in scene {
            assert!(app.world().get_entity(entity).is_ok());
        }
    }

    #[test]
    fn fade_phase_matches_the_thirty_five_frame_transition() {
        // Each fade phase spans 35 frames at 60 fps ≈ 0.583 s: FADE_SPEED alpha
        // per second fills 0→1 in exactly that time.
        assert!((1.0 / FADE_SPEED - FADE_FRAMES / 60.0).abs() < 1e-6);
    }
}

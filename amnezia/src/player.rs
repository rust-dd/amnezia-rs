//! The player hero: spawning, grid movement (blocked by passability and solid
//! events), player-touch event triggers, walk animation, and camera follow.

use crate::assets::resolve_png;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::{
    Character, MapData, MapEvents, MoveQueue, RouteAction, RouteStepper, ScenePause, walk,
};
use amnezia_data::EventPage;
use bevy::prelude::*;

mod camera;
pub(crate) use camera::CameraFollow;
pub use camera::CameraPan;

const PLAYER_CHARSET: &str = "Chara1";
const PLAYER_INDEX: u32 = 0;

/// The hero the player controls, tracked in tile coordinates. `charset`/`index`
/// hold the current graphic so a `MoveEvent` `ChangeGraphic` (the intro's sleep
/// pose) can swap it, like an event NPC's.
#[derive(Component)]
pub struct Player {
    pub tile_x: i32,
    pub tile_y: i32,
    pub dir: u32,
    pub frame: u32,
    pub charset: String,
    pub index: u32,
}

impl Character for Player {
    fn tile(&self) -> (i32, i32) {
        (self.tile_x, self.tile_y)
    }
    fn set_tile(&mut self, x: i32, y: i32) {
        self.tile_x = x;
        self.tile_y = y;
    }
    fn dir(&self) -> u32 {
        self.dir
    }
    fn set_dir(&mut self, dir: u32) {
        self.dir = dir;
    }
    fn frame(&self) -> u32 {
        self.frame
    }
    fn set_frame(&mut self, frame: u32) {
        self.frame = frame;
    }
    fn index(&self) -> u32 {
        self.index
    }
    fn charset(&self) -> &str {
        &self.charset
    }
    fn set_graphic(&mut self, name: String, index: u32) {
        self.charset = name;
        self.index = index;
    }
}

/// Scripted visibility is independent of move-route transparency.
#[derive(Resource, Default)]
pub struct HeroHidden(pub bool);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraPan>()
            .init_resource::<HeroHidden>()
            .add_systems(
                Update,
                (move_player, walk::<Player>, update_player_sprite).chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    camera::camera_follow
                        .in_set(CameraFollow)
                        .after(crate::vehicles::VehicleSync)
                        .before(crate::screenfx::ScreenShakeSet)
                        .before(bevy::transform::TransformSystems::Propagate),
                    update_hero_hidden
                        .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
                ),
            );
    }
}

/// Spawn the hero at `start` (tile) positioned via the map's geometry.
pub fn spawn_player(
    commands: &mut Commands,
    asset_server: &AssetServer,
    start: (i32, i32),
    data: &MapData,
) {
    let image = asset_server.load(resolve_png("CharSet", PLAYER_CHARSET));
    let (sx, sy) = tiles::charset_source(PLAYER_INDEX, DIR_DOWN, 1);
    let (world_x, world_y) = data.tile_center(start.0, start.1);
    commands.spawn((
        Player {
            tile_x: start.0,
            tile_y: start.1,
            dir: DIR_DOWN,
            frame: 1,
            charset: PLAYER_CHARSET.to_string(),
            index: PLAYER_INDEX,
        },
        MoveQueue::default(),
        RouteStepper::default(),
        Sprite {
            image,
            rect: Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H)),
            custom_size: Some(Vec2::new(tiles::CHAR_W, tiles::CHAR_H)),
            ..default()
        },
        Transform::from_xyz(
            world_x,
            world_y + CHAR_Y_OFFSET,
            tiles::character_z(start.1),
        ),
    ));
}

/// The tile directly in front of the player, given its facing direction.
pub fn facing_tile(player: &Player) -> (i32, i32) {
    let (dx, dy) = match player.dir {
        DIR_UP => (0, -1),
        DIR_RIGHT => (1, 0),
        DIR_DOWN => (0, 1),
        _ => (-1, 0),
    };
    (player.tile_x + dx, player.tile_y + dy)
}

#[allow(clippy::too_many_arguments)]
fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    prompts: crate::dialogue::InputPrompts,
    data: Res<MapData>,
    dialogue: Res<Dialogue>,
    scene: ScenePause,
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut running: ResMut<RunningEvent>,
    mut players: Query<(&mut Player, &mut MoveQueue, &mut RouteStepper)>,
    mut arrived: Local<Option<(u32, i32, i32)>>,
) {
    let Ok((mut player, mut queue, mut stepper)) = players.single_mut() else {
        return;
    };
    let position = (data.map_id, player.tile_x, player.tile_y);
    if data.is_changed() {
        *arrived = Some(position);
    }
    if dialogue.active
        || prompts.active()
        || running.active()
        || scene.paused()
        || scene.riding()
        || stepper.active()
    {
        *arrived = Some(position);
        return;
    }
    if queue.busy() {
        return;
    }
    if arrived
        .replace(position)
        .is_some_and(|previous| previous.0 == position.0 && previous != position)
        && let Some((id, page)) = touch_page_at(
            &map_events,
            &switches,
            &variables,
            &party,
            &inventory,
            player.tile_x,
            player.tile_y,
            false,
        )
    {
        running.start(id, page.commands.clone());
        return;
    }
    let step = if keys.pressed(KeyCode::ArrowUp) {
        Some((0, -1, DIR_UP))
    } else if keys.pressed(KeyCode::ArrowDown) {
        Some((0, 1, DIR_DOWN))
    } else if keys.pressed(KeyCode::ArrowLeft) {
        Some((-1, 0, DIR_LEFT))
    } else if keys.pressed(KeyCode::ArrowRight) {
        Some((1, 0, DIR_RIGHT))
    } else {
        None
    };
    let Some((dx, dy, dir)) = step else {
        return;
    };
    stepper.set_direction(&mut *player, dir);
    let (nx, ny) = (player.tile_x + dx, player.tile_y + dy);
    if !data.contains_tile(nx, ny) {
        return;
    }
    let (tx, ty) = data.normalize_tile(nx, ny);
    let blocked = !stepper.through()
        && (!data.can_move(player.tile_x, player.tile_y, nx, ny)
            || event_blocks_at(
                &map_events,
                &switches,
                &variables,
                &party,
                &inventory,
                tx,
                ty,
            ));
    if !blocked {
        queue.set_step_secs(crate::world::step_secs_for_speed(stepper.speed()));
        queue.push_step(RouteAction::Step {
            dx,
            dy,
            face: player.dir,
        });
    }
    if blocked
        && let Some((id, page)) = touch_page_at(
            &map_events,
            &switches,
            &variables,
            &party,
            &inventory,
            tx,
            ty,
            true,
        )
    {
        running.start(id, page.commands.clone());
    }
}

/// Whether a same-layer event occupies tile `(x, y)` and blocks the player.
/// RM2000 events with `layer == 1` are solid (graphic or not); other layers
/// don't block. The active page (per current switches/variables) decides.
#[allow(clippy::too_many_arguments)]
fn event_blocks_at(
    map_events: &MapEvents,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
    x: i32,
    y: i32,
) -> bool {
    map_events.events.iter().any(|e| {
        e.x as i32 == x
            && e.y as i32 == y
            && active_page(e, switches, variables, party, inventory).is_some_and(|p| p.layer == 1)
    })
}

/// The active page (with its event id) of a player-touch event (trigger 1 or 2)
/// on tile `(x, y)`, if any — the command list the interpreter should run on
/// contact.
#[allow(clippy::too_many_arguments)]
fn touch_page_at<'a>(
    map_events: &'a MapEvents,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
    x: i32,
    y: i32,
    same_layer: bool,
) -> Option<(u32, &'a EventPage)> {
    map_events
        .events
        .iter()
        .filter(|e| e.x as i32 == x && e.y as i32 == y)
        .find_map(|e| {
            active_page(e, switches, variables, party, inventory)
                .filter(|p| (p.trigger == 1 || p.trigger == 2) && (p.layer == 1) == same_layer)
                .map(|p| (e.id, p))
        })
}

fn update_player_sprite(
    data: Res<MapData>,
    asset_server: Res<AssetServer>,
    mut players: Query<(&Player, &MoveQueue, &mut Sprite, &mut Transform), Changed<Player>>,
) {
    for (player, queue, mut sprite, mut transform) in &mut players {
        // While a step tweens, `walk` owns the sprite; here we only render the
        // hero at rest (keyboard turns, teleport arrival, settled routes).
        if queue.busy() {
            continue;
        }
        sprite.image = asset_server.load(resolve_png("CharSet", &player.charset));
        let (sx, sy) = tiles::charset_source(player.index, player.dir, player.frame);
        sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
        let (world_x, world_y) = data.tile_center(player.tile_x, player.tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
        transform.translation.z = tiles::character_z(player.tile_y);
    }
}

fn update_hero_hidden(
    hidden: Res<HeroHidden>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut players: Query<&mut Visibility, With<Player>>,
) {
    let invisible = hidden.0 || vehicles.as_ref().is_some_and(|v| v.riding());
    for mut visibility in &mut players {
        *visibility = if invisible {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[cfg(test)]
mod tests;

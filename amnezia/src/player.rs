//! The player hero: spawning, grid movement (blocked by passability and solid
//! events), player-touch event triggers, walk animation, and camera follow.

use crate::assets::resolve_png;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::EventSprite;
use crate::world::{
    Character, EventTriggers, MapData, MoveQueue, RouteAction, RouteStepper, ScenePause, walk,
};
use bevy::prelude::*;

mod arrival;
mod camera;
mod input;
use input::move_player;
pub(crate) mod update;
pub use camera::CameraPan;
pub(crate) use camera::saved as saved_camera;
pub(crate) use camera::{BackgroundScroll, CameraFollow};

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

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PlayerStep;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PlayerInput;

#[derive(Resource, Default)]
pub(crate) struct InputPhase {
    pub(crate) blocked: bool,
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroHidden>()
            .add_systems(
                Update,
                update_player_sprite
                    .after(PlayerStep)
                    .after(crate::appearance::ActorGraphics),
            )
            .add_systems(
                PostUpdate,
                update_hero_hidden
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
        update::post(app, || {
            camera::camera_follow
                .in_set(CameraFollow)
                .after(PlayerStep)
                .after(crate::vehicles::VehicleSync)
                .before(crate::screenfx::ScreenShakeSet)
                .before(crate::dialogue::MessageUpdate)
        });
        register_movement(app);
    }
}

fn register_movement(app: &mut App) {
    update::register(app);
    app.init_resource::<CameraPan>()
        .init_resource::<InputPhase>();
    update::character(app, || {
        move_player
            .in_set(PlayerInput)
            .after(crate::interpreter::ParallelStep)
            .after(crate::world::update::HeroRouteStep)
            .after(crate::menu::MenuInput)
            .after(crate::world::saved::RestoreCharacters)
            .before(PlayerStep)
    });
    arrival::register(app);
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

pub(crate) fn update_player_sprite(
    data: Res<MapData>,
    asset_server: Res<AssetServer>,
    mut players: Query<(&Player, &MoveQueue, &mut Sprite, &mut Transform), Changed<Player>>,
) {
    for (player, queue, mut sprite, mut transform) in &mut players {
        if player.charset.is_empty() {
            continue;
        }
        sprite.image = asset_server.load(resolve_png("CharSet", &player.charset));
        let (sx, sy) = tiles::charset_source(player.index, player.dir, player.frame);
        sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
        if queue.busy() {
            continue;
        }
        let (world_x, world_y) = data.tile_center(player.tile_x, player.tile_y);
        transform.translation.x = world_x;
        transform.translation.y = world_y + CHAR_Y_OFFSET;
        transform.translation.z = tiles::character_z(player.tile_y);
    }
}

fn update_hero_hidden(
    hidden: Res<HeroHidden>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut players: Query<(&Player, &mut Visibility)>,
) {
    let invisible = hidden.0 || vehicles.as_ref().is_some_and(|v| v.aboard());
    for (player, mut visibility) in &mut players {
        *visibility = if invisible || player.charset.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[cfg(test)]
mod tests;

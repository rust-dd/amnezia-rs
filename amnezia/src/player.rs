//! The player hero: spawning, grid movement (blocked by passability and solid
//! events), player-touch event triggers, walk animation, and camera follow.

use crate::assets::resolve_png;
use crate::battle::BattleActive;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::menu::MenuOpen;
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::teleport::Fade;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::title::TitleActive;
use crate::world::{Character, MapData, MapEvents, MoveQueue, RouteAction, walk};
use amnezia_data::EventPage;
use bevy::prelude::*;

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

/// A camera offset for cutscene panning (`PanScreen` 11060). [`camera_follow`]
/// adds `offset` after clamping to the map, so a pan can scroll past the map
/// edge; [`ease_camera_pan`] slides `offset` toward `target` at `speed` world
/// units per second. The interpreter's PanScreen arm sets `target`/`speed` and
/// zeroes `target` to return.
#[derive(Resource, Default)]
pub struct CameraPan {
    pub offset: Vec2,
    pub target: Vec2,
    pub speed: f32,
}

/// The hero's transparency on the RM2000 0..7 scale (`PlayerTransparency`
/// 11310): 0 is opaque, 7 the most see-through. [`update_hero_transparency`]
/// maps it to the sprite's alpha.
#[derive(Resource, Default)]
pub struct HeroTransparency(pub u8);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraPan>()
            .init_resource::<HeroTransparency>()
            .add_systems(
                Update,
                (
                    move_player,
                    walk::<Player>,
                    update_player_sprite,
                    update_hero_transparency,
                    ease_camera_pan,
                    camera_follow,
                )
                    .chain(),
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
    data: Res<MapData>,
    dialogue: Res<Dialogue>,
    fade: Res<Fade>,
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    battle: Res<BattleActive>,
    title: Res<TitleActive>,
    mut running: ResMut<RunningEvent>,
    mut players: Query<(&mut Player, &mut MoveQueue)>,
) {
    if dialogue.active || fade.busy() || running.active() || menu.0 || shop.0 || battle.0 || title.0
    {
        return;
    }
    let Ok((mut player, mut queue)) = players.single_mut() else {
        return;
    };
    // Held (not tapped) so the hero keeps walking; the queue paces it one tile at
    // a time via a smooth tween rather than an instant snap.
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
        // Settle to the standing frame once the hero comes to rest.
        if !queue.busy() && player.frame != 1 {
            player.frame = 1;
        }
        return;
    };
    // Finish the tile in flight before deciding the next one, so a step is one
    // whole tile and facing doesn't flip mid-stride.
    if queue.busy() {
        return;
    }
    player.dir = dir;
    let (nx, ny) = (player.tile_x + dx, player.tile_y + dy);
    if nx < 0 || ny < 0 || nx >= data.width || ny >= data.height {
        return;
    }
    let blocked = !data.passable(nx, ny)
        || event_blocks_at(
            &map_events,
            &switches,
            &variables,
            &party,
            &inventory,
            nx,
            ny,
        );
    if !blocked {
        queue.push_step(RouteAction::Step { dx, dy, face: dir });
    }
    // A player-touch event fires on the attempt to enter its tile — after a
    // passable step onto it, or in place at a solid one (RM2000 doors/exits are
    // solid). It fires only on input, never on the interpreter's own actions, so
    // there's no re-trigger loop.
    if let Some((id, page)) = touch_page_at(
        &map_events,
        &switches,
        &variables,
        &party,
        &inventory,
        nx,
        ny,
    ) {
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
) -> Option<(u32, &'a EventPage)> {
    map_events
        .events
        .iter()
        .filter(|e| e.x as i32 == x && e.y as i32 == y)
        .find_map(|e| {
            active_page(e, switches, variables, party, inventory)
                .filter(|p| p.trigger == 1 || p.trigger == 2)
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

/// Fade the hero sprite to match [`HeroTransparency`]: alpha `1 - t/8` on the
/// RM2000 0..7 scale. Runs on change (and once at startup), independent of the
/// per-tile sprite refresh so it holds while the hero walks or stands.
fn update_hero_transparency(
    transparency: Res<HeroTransparency>,
    mut players: Query<&mut Sprite, With<Player>>,
) {
    if !transparency.is_changed() {
        return;
    }
    let alpha = 1.0 - transparency.0 as f32 / 8.0;
    for mut sprite in &mut players {
        sprite.color = sprite.color.with_alpha(alpha);
    }
}

#[allow(clippy::type_complexity)]
fn camera_follow(
    data: Res<MapData>,
    pan: Res<CameraPan>,
    players: Query<&Transform, With<Player>>,
    mut cameras: Query<(&mut Transform, &Projection), (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let Ok((mut camera, projection)) = cameras.single_mut() else {
        return;
    };
    // The world-space viewport is the fixed 320×240 (not the window pixels), so
    // the clamp stops the camera at the map edge for that view, not the window.
    let Projection::Orthographic(view) = projection else {
        return;
    };
    let half_map_w = data.width as f32 * tiles::TILE / 2.0;
    let half_map_h = data.height as f32 * tiles::TILE / 2.0;
    // The pan offset is added after the clamp so a cutscene can deliberately
    // scroll past the map edge; a zero offset leaves the normal follow untouched.
    camera.translation.x =
        clamp_to_map(player.translation.x, half_map_w, view.area.width() / 2.0) + pan.offset.x;
    camera.translation.y =
        clamp_to_map(player.translation.y, half_map_h, view.area.height() / 2.0) + pan.offset.y;
}

/// Follow `target` but keep the camera inside the map: never scroll past the
/// edge (which would reveal the empty area beyond the map). When the map is
/// smaller than the viewport on an axis, it is centered (returns 0). The map is
/// centered on the origin, so its extent on each axis is `±half_map`.
fn clamp_to_map(target: f32, half_map: f32, half_view: f32) -> f32 {
    if half_view >= half_map {
        0.0
    } else {
        target.clamp(-half_map + half_view, half_map - half_view)
    }
}

/// Ease the camera pan: slide `offset` toward `target` at `speed` world units
/// per second, snapping the last fraction of a step so it settles exactly.
fn ease_camera_pan(time: Res<Time>, mut pan: ResMut<CameraPan>) {
    pan.offset = ease_toward(pan.offset, pan.target, pan.speed * time.delta_secs());
}

/// Step `offset` toward `target` by at most `step`; snap to `target` once within
/// one step (or already there), which also avoids normalising a zero vector.
fn ease_toward(offset: Vec2, target: Vec2, step: f32) -> Vec2 {
    let delta = target - offset;
    if delta.length() <= step {
        target
    } else {
        offset + delta.normalize() * step
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_to_map, ease_toward};
    use bevy::math::Vec2;

    #[test]
    fn pan_eases_toward_target_then_snaps() {
        // A full step moves exactly `step` along the direction to the target.
        assert_eq!(
            ease_toward(Vec2::ZERO, Vec2::new(10.0, 0.0), 2.0),
            Vec2::new(2.0, 0.0)
        );
        // Within one step of the target: snap onto it, no overshoot.
        assert_eq!(
            ease_toward(Vec2::new(9.0, 0.0), Vec2::new(10.0, 0.0), 5.0),
            Vec2::new(10.0, 0.0)
        );
        // Already at the target: stay put (and don't normalise a zero delta).
        assert_eq!(
            ease_toward(Vec2::splat(4.0), Vec2::splat(4.0), 5.0),
            Vec2::splat(4.0)
        );
    }

    #[test]
    fn camera_clamps_to_map_edges() {
        // map half-extent 320, viewport half 160: the camera stops at ±160
        assert_eq!(clamp_to_map(1000.0, 320.0, 160.0), 160.0);
        assert_eq!(clamp_to_map(-1000.0, 320.0, 160.0), -160.0);
        // well inside the map: follows the target exactly
        assert_eq!(clamp_to_map(50.0, 320.0, 160.0), 50.0);
        // map narrower than the viewport: centered, no gray edge
        assert_eq!(clamp_to_map(1000.0, 100.0, 160.0), 0.0);
    }
}

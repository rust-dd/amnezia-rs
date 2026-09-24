//! Shared tile-to-tile movement for the hero and event NPCs: a per-character
//! queue that tweens one tile per step, and the generic [`walk`] system that
//! drives any [`Character`].
//!
//! Forced routes (from the route stepper, driving `MoveEvent` and `move_type` 6),
//! autonomous NPC steps, and the hero's own keyboard steps all share this queue,
//! so every kind of movement animates smoothly across a tile instead of
//! teleport-snapping. The stepper applies a route's facing/graphic changes
//! directly to the character and paces it via its own timer, so the queue itself
//! carries walking steps and complete jumps.

use super::MapData;
use crate::assets::resolve_png;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::collections::VecDeque;

pub(super) mod saved;

/// Seconds a character spends tweening across one tile at RM2000 move speed 4
/// (the hero's pace, and every scripted route's). Autonomous event movement
/// scales this per the event's move speed via [`step_secs_for_speed`].
const STEP_DURATION: f32 = 8.0 / 60.0;

/// Tween seconds for one tile at RM2000 move `speed` (1 slowest … 6 fastest).
/// Each tile is 256 subpixels; movement advances `1 << (1 + speed)` per 60 Hz frame.
pub(crate) fn step_secs_for_speed(speed: u32) -> f32 {
    STEP_DURATION * 2f32.powi(4 - speed.clamp(1, 6) as i32)
}

/// A queued move and its final facing; a jump crosses its full delta in one tween.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum RouteAction {
    Step { dx: i32, dy: i32, face: u32 },
    Jump { dx: i32, dy: i32, face: u32 },
}

impl RouteAction {
    pub(crate) fn delta(&self) -> (i32, i32) {
        match *self {
            Self::Step { dx, dy, .. } | Self::Jump { dx, dy, .. } => (dx, dy),
        }
    }
}

/// A movable map character (the hero or an event NPC). Lets the shared movement
/// code read and write a character's tile, facing, walk frame, and graphic
/// without knowing its concrete component type.
pub trait Character {
    fn tile(&self) -> (i32, i32);
    fn set_tile(&mut self, x: i32, y: i32);
    fn dir(&self) -> u32;
    fn set_dir(&mut self, dir: u32);
    fn frame(&self) -> u32;
    fn set_frame(&mut self, frame: u32);
    fn index(&self) -> u32;
    fn charset(&self) -> &str;
    fn set_graphic(&mut self, name: String, index: u32);
    fn y_offset(&self) -> f32 {
        CHAR_Y_OFFSET
    }
    /// The world draw-Z for this character at tile row `tile_y`. The hero uses
    /// the default "same as hero" band; an event NPC overrides it to sit in the
    /// band for its page layer (below/same/above the hero).
    fn draw_z(&self, tile_y: i32) -> f32 {
        tiles::character_z(tile_y)
    }
}

/// An in-progress walk or jump between tile centers.
#[derive(Clone)]
struct Tween {
    from: Vec2,
    to: Vec2,
    elapsed: f32,
    jumping: bool,
}

impl Tween {
    fn position(&self, duration: f32) -> Vec2 {
        let progress = (self.elapsed / duration).clamp(0.0, 1.0);
        let mut pos = self.from.lerp(self.to, progress);
        if self.jumping {
            pos.y += jump_height(progress);
        }
        pos
    }
}

/// A character's pending steps and current tween. Animation timing belongs to
/// the character, independently of individual steps and route boundaries.
#[derive(Component, Clone)]
pub struct MoveQueue {
    steps: VecDeque<RouteAction>,
    active: Option<Tween>,
    step_secs: f32,
}

impl Default for MoveQueue {
    fn default() -> Self {
        Self {
            steps: VecDeque::new(),
            active: None,
            step_secs: STEP_DURATION,
        }
    }
}

impl MoveQueue {
    /// Append the movements of a scripted route.
    pub fn enqueue_route(&mut self, actions: impl IntoIterator<Item = RouteAction>) {
        self.steps.extend(actions);
    }

    /// Queue a single keyboard step.
    pub fn push_step(&mut self, action: RouteAction) {
        self.steps.push_back(action);
    }

    /// Set the per-tile tween duration for the steps that follow (RM2000 move
    /// speed, via [`step_secs_for_speed`]); the autonomous mover sets this from
    /// the event's speed before enqueuing its step.
    pub fn set_step_secs(&mut self, secs: f32) {
        self.step_secs = secs;
    }

    /// Whether a step is tweening or pending; the interpreter waits on this and
    /// the sprite systems yield rendering to [`walk`] while it holds.
    pub fn busy(&self) -> bool {
        self.active.is_some() || !self.steps.is_empty()
    }

    pub(crate) fn render_position<C: Character>(&self, ch: &C, data: &MapData) -> Vec2 {
        self.active.as_ref().map_or_else(
            || center(data, ch.tile().0, ch.tile().1),
            |tween| tween.position(self.step_secs),
        )
    }

    pub(crate) fn ground_position<C: Character>(&self, ch: &C, data: &MapData) -> Vec2 {
        self.active.as_ref().map_or_else(
            || center(data, ch.tile().0, ch.tile().1),
            |tween| {
                tween
                    .from
                    .lerp(tween.to, (tween.elapsed / self.step_secs).clamp(0.0, 1.0))
            },
        )
    }

    pub(crate) fn jumping(&self) -> bool {
        self.active.as_ref().is_some_and(|t| t.jumping)
    }

    /// Advance the current step by `dt`, applying instant actions in order and
    /// starting the next tween. Returns the world-space tile-center position to
    /// render at this frame, or `None` when the character is idle.
    pub(crate) fn advance<C: Character>(
        &mut self,
        ch: &mut C,
        data: &MapData,
        dt: f32,
    ) -> Option<Vec2> {
        let step = self.step_secs;
        loop {
            if let Some(tween) = self.active.as_mut() {
                tween.elapsed += dt;
                if tween.elapsed < step {
                    return Some(tween.position(step));
                }
                let end = tween.to;
                self.active = None;
                return Some(end);
            }
            let action = self.steps.pop_front()?;
            self.begin_step(ch, data, action);
        }
    }

    /// Commit a move to its destination: update the logical tile immediately
    /// (so y-sorting and lookups use the destination), face the move, and start
    /// the pixel tween from the old center to the new one.
    fn begin_step<C: Character>(&mut self, ch: &mut C, data: &MapData, action: RouteAction) {
        let (dx, dy) = action.delta();
        let jumping = matches!(action, RouteAction::Jump { .. });
        let (RouteAction::Step { face, .. } | RouteAction::Jump { face, .. }) = action;
        let (x, y) = ch.tile();
        let from = center(data, x, y);
        let (nx, ny) = (x + dx, y + dy);
        let (tile_x, tile_y) = data.normalize_tile(nx, ny);
        ch.set_tile(tile_x, tile_y);
        ch.set_dir(face);
        self.active = Some(Tween {
            from,
            to: center(data, nx, ny),
            elapsed: 0.0,
            jumping,
        });
    }
}

fn jump_height(progress: f32) -> f32 {
    let height = (progress.min(1.0 - progress) * 32.0).floor().max(0.0);
    if height < 5.0 {
        height * 2.0
    } else if height < 13.0 {
        height + 4.0
    } else {
        16.0
    }
}

/// World-space center of a tile as a [`Vec2`].
fn center(data: &MapData, x: i32, y: i32) -> Vec2 {
    let (wx, wy) = data.tile_center(x, y);
    Vec2::new(wx, wy)
}

/// The tile delta for a cardinal or diagonal movement direction.
pub(crate) fn dir_delta(dir: u32) -> (i32, i32) {
    match dir {
        DIR_UP => (0, -1),
        DIR_RIGHT => (1, 0),
        DIR_DOWN => (0, 1),
        DIR_LEFT => (-1, 0),
        4 => (1, -1),
        5 => (1, 1),
        6 => (-1, 1),
        7 => (-1, -1),
        _ => (0, 0),
    }
}

/// Drive every [`Character`]'s move queue: advance the tween, then reflect the
/// interpolated position and time-driven animation onto its sprite, including
/// continuous and spinning animation while the character stands still.
#[allow(clippy::type_complexity)]
pub fn walk<C: Character + Component<Mutability = Mutable>>(world: &mut World) {
    world
        .run_system_cached_with(walk_selected::<C>, None)
        .unwrap();
}

#[allow(clippy::type_complexity)]
pub(super) fn walk_selected<C: Character + Component<Mutability = Mutable>>(
    In(target): In<Option<Entity>>,
    time: Res<Time>,
    data: Res<MapData>,
    asset_server: Res<AssetServer>,
    scene: super::ScenePause,
    mut movers: Query<(
        Entity,
        &mut C,
        &mut MoveQueue,
        &mut Transform,
        &mut Sprite,
        Option<&mut super::RouteStepper>,
    )>,
) {
    if scene.paused() {
        return;
    }
    let dt = time.delta_secs();
    for (entity, mut ch, mut queue, mut transform, mut sprite, route) in &mut movers {
        if target.is_some_and(|target| entity != target) {
            continue;
        }
        let moving = queue.busy();
        let facing = ch.dir();
        let position = if moving {
            queue.advance(&mut *ch, &data, dt)
        } else {
            None
        };
        if let Some(mut route) = route {
            if moving && !queue.busy() {
                route.settle_movement();
            }
            if route.animation.keeps_facing() && ch.dir() != facing {
                ch.set_dir(facing);
            }
            let speed = route.speed();
            let previous = (ch.frame(), ch.dir());
            route.animation.advance(
                ch.bypass_change_detection(),
                speed,
                moving,
                queue.jumping(),
                dt,
            );
            if previous != (ch.frame(), ch.dir()) {
                ch.set_changed();
            }
        }
        if (position.is_some() || ch.is_changed()) && !ch.charset().is_empty() {
            let (sx, sy) = tiles::charset_source(ch.index(), ch.dir(), ch.frame());
            sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
            sprite.image = asset_server.load(resolve_png("CharSet", ch.charset()));
        }
        if let Some(pos) = position {
            let z = ch.draw_z(ch.tile().1);
            transform.translation = Vec3::new(pos.x, pos.y + ch.y_offset(), z);
        }
    }
}

impl Character for super::EventSprite {
    fn y_offset(&self) -> f32 {
        if self.charset.is_empty() {
            0.0
        } else {
            CHAR_Y_OFFSET
        }
    }
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
    fn draw_z(&self, tile_y: i32) -> f32 {
        tiles::character_z_layer(tile_y, self.layer)
    }
}

#[cfg(test)]
mod tests;

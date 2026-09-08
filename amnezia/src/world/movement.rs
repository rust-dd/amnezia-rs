//! Shared tile-to-tile movement for the hero and event NPCs: a per-character
//! queue that tweens one tile per step, and the generic [`walk`] system that
//! drives any [`Character`].
//!
//! Forced routes (from the route stepper, driving `MoveEvent` and `move_type` 6),
//! autonomous NPC steps, and the hero's own keyboard steps all share this queue,
//! so every kind of movement animates smoothly across a tile instead of
//! teleport-snapping. The stepper applies a route's facing/graphic changes
//! directly to the character and paces it via its own timer, so the queue itself
//! only ever carries tile [`RouteAction::Step`]s.

use super::MapData;
use crate::assets::resolve_png;
use crate::tiles::{self, CHAR_Y_OFFSET, DIR_DOWN, DIR_RIGHT, DIR_UP};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::collections::VecDeque;

/// Seconds a character spends tweening across one tile at RM2000 move speed 4
/// (the hero's pace, and every scripted route's). Autonomous event movement
/// scales this per the event's move speed via [`step_secs_for_speed`].
const STEP_DURATION: f32 = 0.18;

/// Tween seconds for one tile at RM2000 move `speed` (1 slowest … 6 fastest).
/// EasyRPG advances a move by `1 << (1 + speed)` per frame, so each speed step
/// halves the time; anchoring speed 4 at [`STEP_DURATION`] keeps the hero's feel
/// while scaling the rest by that same power of two (speed 3 = 0.36s, speed 5 =
/// 0.09s). The speed is clamped to the valid 1–6 range.
pub(super) fn step_secs_for_speed(speed: u32) -> f32 {
    STEP_DURATION * 2f32.powi(4 - speed.clamp(1, 6) as i32)
}

/// One queued tile step: its `(dx, dy)` delta and the facing it leaves the
/// character in. This is the only action the queue tweens — a route's turns,
/// graphic swaps, and waits are handled by the stepper, not queued here.
#[derive(Clone, PartialEq, Debug)]
pub enum RouteAction {
    Step { dx: i32, dy: i32, face: u32 },
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

/// An in-progress single-tile tween between two tile centers (equal for `Wait`).
struct Tween {
    from: Vec2,
    to: Vec2,
    elapsed: f32,
}

/// A character's pending route steps plus the tween of the step in flight. The
/// interpreter enqueues scripted routes; the player system pushes keyboard
/// steps. `route` marks a scripted run so the character settles to its standing
/// frame when the route drains (keyboard idling settles separately).
#[derive(Component)]
pub struct MoveQueue {
    steps: VecDeque<RouteAction>,
    active: Option<Tween>,
    route: bool,
    step_secs: f32,
}

impl Default for MoveQueue {
    fn default() -> Self {
        Self {
            steps: VecDeque::new(),
            active: None,
            route: false,
            step_secs: STEP_DURATION,
        }
    }
}

impl MoveQueue {
    /// Append a scripted route and mark it as one, so the character settles to
    /// its standing frame once the route finishes.
    pub fn enqueue_route(&mut self, actions: impl IntoIterator<Item = RouteAction>) {
        self.steps.extend(actions);
        self.route = true;
    }

    /// Queue a single keyboard step (no scripted-settle).
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

    /// Whether [`walk`] still has something to do (busy, or a scripted route to
    /// settle). Keyboard idling stays out of the movement system.
    fn has_work(&self) -> bool {
        self.busy() || self.route
    }

    /// Advance the current step by `dt`, applying instant actions in order and
    /// starting the next tween. Returns the world-space tile-center position to
    /// render at this frame, or `None` when the character is idle.
    fn advance<C: Character>(&mut self, ch: &mut C, data: &MapData, dt: f32) -> Option<Vec2> {
        let step = self.step_secs;
        loop {
            if let Some(tween) = self.active.as_mut() {
                tween.elapsed += dt;
                if tween.elapsed < step {
                    return Some(tween.from.lerp(tween.to, tween.elapsed / step));
                }
                let end = tween.to;
                self.active = None;
                return Some(end);
            }
            match self.steps.pop_front() {
                None => {
                    if self.route {
                        self.route = false;
                        if ch.frame() != 1 {
                            ch.set_frame(1);
                        }
                    }
                    return None;
                }
                Some(RouteAction::Step { dx, dy, face }) => self.begin_step(ch, data, dx, dy, face),
            }
        }
    }

    /// Commit a move to the adjacent tile: update the logical tile immediately
    /// (so y-sorting and lookups use the destination), face and advance the walk
    /// frame, and start the pixel tween from the old center to the new one.
    fn begin_step<C: Character>(
        &mut self,
        ch: &mut C,
        data: &MapData,
        dx: i32,
        dy: i32,
        face: u32,
    ) {
        let (x, y) = ch.tile();
        let from = center(data, x, y);
        let (nx, ny) = (x + dx, y + dy);
        ch.set_tile(nx, ny);
        ch.set_dir(face);
        ch.set_frame((ch.frame() + 1) % 3);
        self.active = Some(Tween {
            from,
            to: center(data, nx, ny),
            elapsed: 0.0,
        });
    }
}

/// World-space center of a tile as a [`Vec2`].
fn center(data: &MapData, x: i32, y: i32) -> Vec2 {
    let (wx, wy) = data.tile_center(x, y);
    Vec2::new(wx, wy)
}

/// The tile delta for a facing direction.
pub(super) fn dir_delta(dir: u32) -> (i32, i32) {
    match dir {
        DIR_UP => (0, -1),
        DIR_RIGHT => (1, 0),
        DIR_DOWN => (0, 1),
        _ => (-1, 0),
    }
}

/// Drive every [`Character`]'s move queue: advance the tween, then reflect the
/// interpolated position, walk frame, facing, and graphic onto its sprite. Runs
/// only while a queue has work, leaving idle characters to their own change-driven
/// sprite update.
pub fn walk<C: Character + Component<Mutability = Mutable>>(
    time: Res<Time>,
    data: Res<MapData>,
    asset_server: Res<AssetServer>,
    mut movers: Query<(&mut C, &mut MoveQueue, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (mut ch, mut queue, mut transform, mut sprite) in &mut movers {
        if !queue.has_work() {
            continue;
        }
        if let Some(pos) = queue.advance(&mut *ch, &data, dt) {
            if !ch.charset().is_empty() {
                let (sx, sy) = tiles::charset_source(ch.index(), ch.dir(), ch.frame());
                sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
                sprite.image = asset_server.load(resolve_png("CharSet", ch.charset()));
            }
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
mod tests {
    use super::*;

    #[test]
    fn queue_tracks_busy_and_work() {
        let mut q = MoveQueue::default();
        assert!(!q.busy() && !q.has_work());
        q.enqueue_route([RouteAction::Step {
            dx: 0,
            dy: 1,
            face: DIR_DOWN,
        }]);
        assert!(q.busy() && q.has_work());
    }

    struct FakeChar {
        x: i32,
        y: i32,
        dir: u32,
        frame: u32,
        charset: String,
    }

    impl Character for FakeChar {
        fn tile(&self) -> (i32, i32) {
            (self.x, self.y)
        }
        fn set_tile(&mut self, x: i32, y: i32) {
            self.x = x;
            self.y = y;
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
            0
        }
        fn charset(&self) -> &str {
            &self.charset
        }
        fn set_graphic(&mut self, name: String, _index: u32) {
            self.charset = name;
        }
    }

    fn test_map() -> MapData {
        MapData {
            map_id: 0,
            width: 5,
            height: 5,
            offset_x: 40.0,
            offset_y: 40.0,
            lower: vec![0; 25],
            upper: vec![10000; 25],
            passages_down: vec![0x0F; 162],
            passages_up: vec![0x0F; 144],
        }
    }

    #[test]
    fn advance_tweens_one_tile_then_settles() {
        let data = test_map();
        let mut ch = FakeChar {
            x: 2,
            y: 2,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
        };
        let mut q = MoveQueue::default();
        q.enqueue_route([RouteAction::Step {
            dx: -1,
            dy: 0,
            face: 3,
        }]);
        // The first tick commits the logical tile and faces the move, rendering
        // still at the old tile's center.
        let start = q.advance(&mut ch, &data, 0.0).unwrap();
        assert_eq!(ch.tile(), (1, 2));
        assert_eq!(ch.dir(), 3);
        // Halfway through, the sprite sits between the two tile centers.
        let mid = q.advance(&mut ch, &data, STEP_DURATION / 2.0).unwrap();
        assert!(mid.x < start.x);
        assert!(q.busy());
        // Completing the step drains the route and settles to the standing frame.
        q.advance(&mut ch, &data, STEP_DURATION).unwrap();
        assert!(q.advance(&mut ch, &data, 0.0).is_none());
        assert!(!q.busy());
        assert_eq!(ch.frame(), 1);
    }
}

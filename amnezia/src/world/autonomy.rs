//! Autonomous event movement: the RM2000 `move_type` behaviours (random, the
//! two pacing cycles, and toward/away the hero) that let NPCs wander on their
//! own, paced by the page's `move_frequency` and tweened at its `move_speed`.
//!
//! Each moving NPC carries an [`AutoMove`] holding its move fields and a
//! frequency-derived countdown. When the countdown elapses and the NPC is
//! standing still (and no message/fade/menu/battle/event is holding the map),
//! [`autonomous_movement`] picks one tile step via [`decide`], updates both the
//! logical [`MapEvents`] tile and the sprite's [`MoveQueue`] so they stay in
//! sync, and re-arms the countdown. Scripted routes, the `Move Event` opcode,
//! and custom routes (`move_type` 6) are handled elsewhere or deferred.

use super::movement::{dir_delta, step_secs_for_speed};
use super::route::RouteStepper;
use super::{EventSprite, MapData, MapEvents, MoveQueue, RouteAction};
use crate::battle::BattleActive;
use crate::dialogue::Dialogue;
use crate::gameover::GameOverActive;
use crate::interpreter::RunningEvent;
use crate::menu::MenuOpen;
use crate::player::Player;
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::Fade;
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::title::TitleActive;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Logical frames per second the RM2000 stop-count delays are measured in.
const FPS: f32 = 60.0;

/// The resources that pause autonomous movement — the same set that stops the
/// hero (message, teleport fade, running event, menu, shop, battle, title) plus
/// game-over. Bundled so [`autonomous_movement`] stays within the system-param
/// count.
#[derive(SystemParam)]
pub(crate) struct MoveGuards<'w> {
    dialogue: Res<'w, Dialogue>,
    fade: Res<'w, Fade>,
    running: Res<'w, RunningEvent>,
    menu: Res<'w, MenuOpen>,
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
    title: Res<'w, TitleActive>,
    gameover: Res<'w, GameOverActive>,
}

impl MoveGuards<'_> {
    /// Whether any pause condition is active, freezing every NPC this frame.
    pub(crate) fn paused(&self) -> bool {
        self.dialogue.active
            || self.fade.busy()
            || self.running.active()
            || self.menu.0
            || self.shop.0
            || self.battle.0
            || self.title.0
            || self.gameover.0
    }

    /// The pauses that freeze even a *forced* move route (a `MoveEvent` on the hero
    /// or an NPC): a scene that owns the screen (menu, shop, battle, title,
    /// game-over) or a teleport fade. Unlike [`MoveGuards::paused`], this omits the
    /// running event and the message — RM2000 advances an overwritten move route
    /// every frame regardless of both (see `Game_Character::Update`, where
    /// `IsMoveRouteOverwritten` short-circuits the interpreter/message stop gate), so
    /// cutscene movement (the intro walking the hero in) plays while the event runs.
    pub(crate) fn forced_route_paused(&self) -> bool {
        self.fade.busy()
            || self.menu.0
            || self.shop.0
            || self.battle.0
            || self.title.0
            || self.gameover.0
    }
}

/// An event's autonomous-movement state: the active page's move fields plus a
/// countdown to the next step and a per-event RNG. Built by [`AutoMove::new`]
/// from the page the NPC spawned with.
#[derive(Component)]
pub struct AutoMove {
    move_type: u32,
    frequency: u32,
    speed: u32,
    timer: f32,
    rng: u32,
}

impl AutoMove {
    /// Build from an active page's `move_type`/`move_frequency`/`move_speed`,
    /// seeding the RNG from the event id (so same-map random movers don't step
    /// in lockstep) and arming the first step one full frequency delay out.
    pub fn new(move_type: u32, frequency: u32, speed: u32, event_id: u32) -> Self {
        Self {
            move_type,
            frequency,
            speed,
            timer: stop_frames(frequency) as f32 / FPS,
            rng: event_id.wrapping_mul(2_654_435_761) | 1,
        }
    }

    /// Seconds until the next step attempt. A random mover gets RM2000's
    /// `SetMaxStopCountForRandom` jitter (`* (3..=6) / 5`) so its wandering
    /// doesn't tick like a metronome; the rest use the flat frequency delay.
    fn next_delay(&mut self) -> f32 {
        let base = stop_frames(self.frequency) as f32 / FPS;
        if self.move_type == 1 {
            base * (next_rand(&mut self.rng) % 4 + 3) as f32 / 5.0
        } else {
            base
        }
    }
}

/// RM2000 stop-count frames between steps for a move `frequency` (1 slowest … 8
/// fastest): `GetMaxStopCountForStep`, `freq >= 8 ? 0 : 1 << (9 - freq)`. Higher
/// frequency ⇒ fewer frames ⇒ more frequent steps.
fn stop_frames(frequency: u32) -> u32 {
    let f = frequency.clamp(1, 8);
    if f >= 8 { 0 } else { 1 << (9 - f) }
}

/// xorshift32 — cheap deterministic per-event randomness, no `rand` dependency.
fn next_rand(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// The opposite of a facing (Up↔Down, Left↔Right).
fn reverse(dir: u32) -> u32 {
    (dir + 2) % 4
}

/// One event's decided action for a movement tick.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Decision {
    /// Step one tile in this direction.
    Step(u32),
    /// Blocked or turning in place: face this direction without moving.
    Face(u32),
    /// Nothing to do (stationary, or no candidate direction).
    Idle,
}

/// Decide an event's next action for `move_type`, given its `facing`, its tile
/// `(ex, ey)`, the player's tile `(px, py)`, a pre-drawn random direction (used
/// only by the random type), and a `passable` test for a candidate direction.
#[allow(clippy::too_many_arguments)]
fn decide(
    move_type: u32,
    facing: u32,
    ex: i32,
    ey: i32,
    px: i32,
    py: i32,
    rand_dir: u32,
    passable: impl Fn(u32) -> bool,
) -> Decision {
    match move_type {
        1 => {
            if passable(rand_dir) {
                Decision::Step(rand_dir)
            } else {
                Decision::Face(rand_dir)
            }
        }
        2 => cycle(facing, DIR_DOWN, passable),
        3 => cycle(facing, DIR_RIGHT, passable),
        4 => seek(&toward_candidates(px - ex, py - ey), passable),
        5 => seek(&away_candidates(px - ex, py - ey), passable),
        _ => Decision::Idle,
    }
}

/// Pace along `default_dir`↔its reverse: keep going the way the event faces, and
/// on a block reverse — turning even when boxed in. Mirrors `MoveTypeCycle`.
fn cycle(facing: u32, default_dir: u32, passable: impl Fn(u32) -> bool) -> Decision {
    let primary = if facing == reverse(default_dir) {
        reverse(default_dir)
    } else {
        default_dir
    };
    if passable(primary) {
        return Decision::Step(primary);
    }
    let back = reverse(primary);
    if passable(back) {
        Decision::Step(back)
    } else {
        Decision::Face(back)
    }
}

/// Step the first passable candidate; if none is passable, face the preferred
/// one and idle. Shared by the toward/away movers.
fn seek(candidates: &[u32], passable: impl Fn(u32) -> bool) -> Decision {
    for &dir in candidates {
        if passable(dir) {
            return Decision::Step(dir);
        }
    }
    match candidates.first() {
        Some(&dir) => Decision::Face(dir),
        None => Decision::Idle,
    }
}

/// Directions stepping toward `(dx, dy)` = player minus event, dominant axis
/// first, skipping an axis the event is already aligned on. Mirrors EasyRPG's
/// `GetDirectionToCharacter`, whose ties favour the vertical axis.
fn toward_candidates(dx: i32, dy: i32) -> Vec<u32> {
    use std::cmp::Ordering::{Equal, Greater, Less};
    let horiz = match dx.cmp(&0) {
        Greater => Some(DIR_RIGHT),
        Less => Some(DIR_LEFT),
        Equal => None,
    };
    let vert = match dy.cmp(&0) {
        Greater => Some(DIR_DOWN),
        Less => Some(DIR_UP),
        Equal => None,
    };
    let (first, second) = if dx.abs() > dy.abs() {
        (horiz, vert)
    } else {
        (vert, horiz)
    };
    first.into_iter().chain(second).collect()
}

/// Directions stepping away from the player: the reverse of each toward
/// direction, in the same dominant-axis order.
fn away_candidates(dx: i32, dy: i32) -> Vec<u32> {
    toward_candidates(dx, dy).into_iter().map(reverse).collect()
}

/// Drive every NPC's autonomous movement: on its frequency countdown, and only
/// while standing still and unblocked by a message/fade/menu/battle/event, pick
/// one passable tile step for its `move_type`, keep the logical [`MapEvents`]
/// tile in step with the sprite's [`MoveQueue`], face the move, and re-arm.
#[allow(clippy::too_many_arguments)]
pub(crate) fn autonomous_movement(
    time: Res<Time>,
    data: Res<MapData>,
    mut map_events: ResMut<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    guards: MoveGuards,
    mut touches: Option<ResMut<super::TouchEvents>>,
    players: Query<&Player>,
    mut movers: Query<(
        &mut EventSprite,
        &mut MoveQueue,
        &mut AutoMove,
        Option<&RouteStepper>,
    )>,
) {
    if guards.paused() {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let (px, py) = (player.tile_x, player.tile_y);
    let dt = time.delta_secs();
    for (mut sprite, mut queue, mut auto, stepper) in &mut movers {
        // Stationary (0) and custom-route (6) events never move here; a busy queue
        // means the previous step is still tweening; and a live forced route (a
        // MoveEvent override) takes over, as RM2000's move_route_overwritten
        // suppresses the autonomous move_type. Like EasyRPG's stop_count, the delay
        // only counts down while the NPC stands still.
        if auto.move_type == 0
            || auto.move_type == 6
            || queue.busy()
            || stepper.is_some_and(RouteStepper::active)
        {
            continue;
        }
        auto.timer -= dt;
        if auto.timer > 0.0 {
            continue;
        }

        let (ex, ey) = (sprite.tile_x, sprite.tile_y);
        let self_id = sprite.id;
        let rand_dir = next_rand(&mut auto.rng) % 4;
        let touched = std::cell::Cell::new(false);
        let decision = {
            let passable = |dir: u32| {
                let (dx, dy) = dir_delta(dir);
                let (nx, ny) = (ex + dx, ey + dy);
                if sprite.layer == 1 && (nx, ny) == (px, py) {
                    touched.set(true);
                }
                nx >= 0
                    && ny >= 0
                    && nx < data.width
                    && ny < data.height
                    && data.can_move(ex, ey, nx, ny)
                    && !(sprite.layer == 1 && nx == px && ny == py)
                    && !super::collision::event_blocks_at(
                        &map_events,
                        (&switches, &variables, &party, &inventory),
                        (self_id, sprite.layer),
                        (nx, ny),
                    )
            };
            decide(
                auto.move_type,
                sprite.dir,
                ex,
                ey,
                px,
                py,
                rand_dir,
                passable,
            )
        };

        if touched.get()
            && let Some(touches) = touches.as_mut()
        {
            touches.0.push(self_id);
        }

        match decision {
            Decision::Step(dir) => {
                let (dx, dy) = dir_delta(dir);
                let (nx, ny) = (ex + dx, ey + dy);
                if let Some(event) = map_events.events.iter_mut().find(|e| e.id == self_id) {
                    event.x = nx as u32;
                    event.y = ny as u32;
                }
                sprite.dir = dir;
                queue.set_step_secs(step_secs_for_speed(auto.speed));
                queue.enqueue_route([RouteAction::Step { dx, dy, face: dir }]);
            }
            Decision::Face(dir) => sprite.dir = dir,
            Decision::Idle => {}
        }
        auto.timer = auto.next_delay();
    }
}

#[cfg(test)]
mod tests;

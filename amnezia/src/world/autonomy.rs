//! Autonomous movement decisions and shared map-scene pause conditions.

mod driver;
pub(super) use driver::advance_event;

use super::collision::{CollisionBodies, MapCollision, Mover};
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

/// Shared movement gates for scenes, foreground events and input prompts.
#[derive(SystemParam)]
pub(crate) struct MoveGuards<'w> {
    transition: Option<Res<'w, crate::transitions::Transition>>,
    frame: Option<Res<'w, crate::timing::SceneWait>>,
    menu_flow: Option<Res<'w, crate::menu::SceneFlow>>,
    shop_flow: Option<Res<'w, crate::shop::SceneFlow>>,
    prompts: crate::dialogue::InputPrompts<'w>,
    message_options: Option<Res<'w, crate::dialogue::MessageOptions>>,
    dialogue: Res<'w, Dialogue>,
    fade: Res<'w, Fade>,
    running: Res<'w, RunningEvent>,
    menu: Res<'w, MenuOpen>,
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
    title: Res<'w, TitleActive>,
    gameover: Res<'w, GameOverActive>,
    save: Option<Res<'w, crate::save::EventSaveRequest>>,
    files: Option<Res<'w, crate::menu::save_files::SaveFiles>>,
    input: Option<Res<'w, crate::player::InputPhase>>,
}

impl MoveGuards<'_> {
    pub(crate) fn autonomous_paused(&self, event_id: u32) -> bool {
        self.forced_route_paused()
            || self.running.event_paused(event_id)
            || (!self
                .message_options
                .as_ref()
                .is_some_and(|o| o.continue_events)
                && self.running.active())
    }

    /// Whether manual movement input is paused.
    pub(crate) fn paused(&self) -> bool {
        self.dialogue.active
            || self.prompts.active()
            || self.running.active()
            || self
                .input
                .as_ref()
                .map_or_else(|| self.running.waiting(), |input| input.blocked)
            || self.forced_route_paused()
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
            || self.frame.as_ref().is_some_and(|wait| wait.0)
            || self
                .menu_flow
                .as_ref()
                .is_some_and(|flow| flow.blocks_map())
            || self.shop_flow.as_ref().is_some_and(|flow| flow.active())
            || self.transition.as_ref().is_some_and(|v| v.busy())
            || self.menu.0
            || self.shop.0
            || self.battle.0
            || self.title.0
            || self.gameover.0
            || self.save.as_ref().is_some_and(|v| v.0)
            || self.files.as_ref().is_some_and(|v| v.active())
    }
}

/// Page movement settings and per-event randomness; the route owns the shared clock.
#[derive(Component, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutoMove {
    move_type: u32,
    frequency: u32,
    speed: u32,
    #[serde(
        default,
        rename = "timer",
        skip_serializing_if = "super::stop_clock::legacy_empty"
    )]
    legacy_timer: f32,
    rng: u32,
}

impl AutoMove {
    pub(super) fn valid(&self) -> bool {
        self.move_type <= 6
            && (1..=8).contains(&self.frequency)
            && (1..=6).contains(&self.speed)
            && super::stop_clock::legacy_valid(self.legacy_timer)
    }

    /// Build from an active page's `move_type`/`move_frequency`/`move_speed`,
    /// seeding the RNG from the event id (so same-map random movers don't step
    /// in lockstep).
    pub fn new(move_type: u32, frequency: u32, speed: u32, event_id: u32) -> Self {
        Self {
            move_type,
            frequency,
            speed,
            legacy_timer: 0.0,
            rng: event_id.wrapping_mul(2_654_435_761) | 1,
        }
    }

    pub(super) fn set_stop_maximum(&mut self, route: &mut RouteStepper) {
        let base = super::stop_clock::step(route.frequency());
        let maximum = if self.move_type == 1 {
            base * (next_rand(&mut self.rng) % 4 + 3) / 5
        } else {
            base
        };
        route.set_stop_maximum(maximum);
    }

    pub(super) fn refresh(
        &mut self,
        page: Option<&amnezia_data::EventPage>,
        route: &mut RouteStepper,
    ) {
        self.move_type = page.map_or(0, |page| page.move_type);
        self.frequency = route.frequency();
        self.speed = route.speed();
        if self.move_type == 1 {
            self.set_stop_maximum(route);
        }
    }

    pub(super) fn restore_clock(&mut self, route: &mut RouteStepper) {
        let delay =
            (!route.forced() && matches!(self.move_type, 1..=5)).then_some(self.legacy_timer);
        route.restore_stop_clock(delay);
        self.legacy_timer = 0.0;
    }
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

#[cfg(test)]
pub(crate) fn autonomous_movement(world: &mut World) {
    world
        .run_system_cached_with(driver::advance_event, None)
        .unwrap();
}

#[cfg(test)]
mod tests;

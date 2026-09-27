//! Autonomous movement decisions and shared map-scene pause conditions.

mod decisions;
mod driver;
pub(super) use driver::advance_event;

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
#[cfg(test)]
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
    continuation: Option<Res<'w, crate::interpreter::continuation::Continuation>>,
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
        let waiting = self.frame.as_ref().is_some_and(|wait| wait.0);
        self.fade.busy()
            || self
                .continuation
                .as_ref()
                .map_or(waiting, |state| state.characters_paused(waiting))
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
        self.set_stop_maximum_for(self.move_type, route);
    }

    fn set_stop_maximum_for(&mut self, move_type: u32, route: &mut RouteStepper) {
        let base = super::stop_clock::step(route.frequency());
        let maximum = if move_type == 1 {
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

#[cfg(test)]
pub(crate) fn autonomous_movement(world: &mut World) {
    driver::advance_event(world, None);
}

#[cfg(test)]
mod tests;

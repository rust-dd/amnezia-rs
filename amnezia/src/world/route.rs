//! Move-route execution and shared side effects for map characters.

mod events;
mod hero;
mod stepper;
pub(super) use events::route_event;
pub(super) use hero::route as route_hero;

pub use stepper::RouteStepper;

#[cfg(test)]
use super::collision::{CollisionBodies, MapCollision, Mover};
use super::{Character, EventSprite, MapData, MapEvents, MoveQueue};
use crate::audio::AudioRequest;
use crate::player::Player;
use crate::state::Switches;
#[cfg(test)]
use crate::state::{Inventory, Party, Variables};
use bevy::prelude::*;
pub(crate) use stepper::StepEffect;
pub(crate) use stepper::Turn;

/// One frame of a character's stepper: the tile delta of the step it enqueued
/// this tick (for the caller to sync the logical event tile), plus the side
/// effects to apply.
pub(crate) struct Driven {
    pub(crate) moved: Option<(i32, i32)>,
    pub(crate) effects: Vec<StepEffect>,
}

impl Driven {
    /// Nothing happened this tick (inactive, mid-step, or still in its delay).
    fn idle() -> Self {
        Self {
            moved: None,
            effects: Vec::new(),
        }
    }
}

/// Drive one character's stepper for a frame: yield (nothing) while it is
/// inactive, mid-step (queue busy), or below its stop threshold; otherwise
/// advance the route, enqueue any resulting step, and return its delta and side
/// effects for the caller to apply. The effects are returned rather than applied
/// here so `can_step` — which borrows the switches and events — is dropped before
/// the caller mutates them.
#[cfg(test)]
pub(crate) fn drive<C: Character>(
    ch: &mut C,
    queue: &mut MoveQueue,
    stepper: &mut RouteStepper,
    hero: (i32, i32),
    can_step: impl Fn(&C, i32, i32, bool, bool) -> bool,
) -> Driven {
    if queue.busy() {
        return Driven::idle();
    }
    if stepper.settle_movement() || !stepper.active() {
        return Driven::idle();
    }
    if stepper.stop_active() {
        return Driven::idle();
    }
    let mut effects = Vec::new();
    let moved = stepper
        .advance(ch, hero, &can_step, &mut effects)
        .map(|(action, secs)| {
            let delta = action.delta();
            queue.set_step_secs(secs);
            queue.enqueue_route([action]);
            delta
        });
    Driven { moved, effects }
}

pub(crate) fn drive_part<C: Character>(
    ch: &mut C,
    queue: &mut MoveQueue,
    stepper: &mut RouteStepper,
    hero: (i32, i32),
    can_step: impl Fn(&C, i32, i32, bool, bool) -> bool,
    turn: Option<Turn>,
) -> (Driven, Option<Turn>) {
    if queue.busy()
        || (turn.is_none() && (stepper.settle_movement() || !stepper.active()))
        || stepper.stop_active()
    {
        return (Driven::idle(), None);
    }
    let mut turn = turn.unwrap_or_else(|| Turn::new(stepper));
    let mut driven = Driven::idle();
    match stepper.advance_turn(ch, hero, &can_step, &mut driven.effects, &mut turn) {
        stepper::Progress::Refresh => return (driven, Some(turn)),
        stepper::Progress::Move(action, seconds) => {
            driven.moved = Some(action.delta());
            queue.set_step_secs(seconds);
            queue.enqueue_route([action]);
        }
        stepper::Progress::Done => {}
    }
    (driven, None)
}

/// Apply the side effects a route command produced: toggle a game switch, play a
/// sound, or set the character's sprite transparency.
fn apply_effects(
    effects: Vec<StepEffect>,
    switches: &mut ResMut<Switches>,
    audio: &mut MessageWriter<AudioRequest>,
    sprite: &mut Sprite,
) {
    for effect in effects {
        match effect {
            StepEffect::Switch(id, on) => switches.set(id, on),
            StepEffect::Sound { name, params } => {
                audio.write(AudioRequest::play_sound(&name, &params));
            }
            StepEffect::Transparency(level) => {
                let alpha = crate::tiles::character_alpha(level);
                sprite.color = sprite.color.with_alpha(alpha);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn tile_open(
    ex: i32,
    ey: i32,
    dx: i32,
    dy: i32,
    jumping: bool,
    mover: (u32, u32),
    hero: (i32, i32),
    data: &MapData,
    map_events: &MapEvents,
    state: (&Switches, &Variables, &Party, &Inventory),
) -> bool {
    let character = EventSprite {
        id: mover.0,
        layer: mover.1,
        tile_x: ex,
        tile_y: ey,
        dir: 2,
        frame: 1,
        charset: "C".into(),
        index: 0,
    };
    MapCollision::new(data, map_events, state, &CollisionBodies::default()).can_move(
        (ex, ey),
        (ex + dx, ey + dy),
        Mover::event(&character, false),
        Some(hero),
        jumping,
    )
}

#[cfg(test)]
pub(super) fn route_events(world: &mut World) {
    events::route_event(world, None, None);
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod refresh_tests;

//! Move-route execution and shared side effects for map characters.

mod events;
mod stepper;
pub(super) use events::route_event;

pub use stepper::RouteStepper;

use super::collision::{CollisionBodies, MapCollision, Mover};
use super::{Character, EventSprite, MapData, MapEvents, MoveQueue};
use crate::audio::AudioRequest;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables};
use bevy::prelude::*;
pub(crate) use stepper::StepEffect;

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

/// Apply the side effects a route command produced: toggle a game switch, play a
/// sound, or set the character's sprite transparency.
fn apply_effects(
    effects: Vec<StepEffect>,
    switches: &mut Switches,
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
    world
        .run_system_cached_with(events::route_event, (None, None))
        .unwrap();
}

/// Step the hero's forced route (a `MoveEvent` targeting the hero). Same guards
/// as event routes; the hero is `self_id` 0 (no event) for the collision test.
#[allow(clippy::too_many_arguments)]
pub(super) fn route_hero(
    data: Res<MapData>,
    map_events: Res<MapEvents>,
    mut switches: ResMut<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    guards: super::autonomy::MoveGuards,
    mut audio: MessageWriter<AudioRequest>,
    mut hero: Query<
        (&mut Player, &mut MoveQueue, &mut RouteStepper, &mut Sprite),
        Without<EventSprite>,
    >,
    events: Query<(&EventSprite, Option<&RouteStepper>), Without<Player>>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
) {
    // The hero's only routes come from a `MoveEvent`, which is always forced, so they
    // must keep advancing through the very cutscene that issued them — RM2000 steps
    // an overwritten route even while the event interpreter runs and a message shows.
    if guards.forced_route_paused() || vehicles.as_ref().is_some_and(|v| v.airship_transitioning())
    {
        return;
    }
    let Ok((mut player, mut queue, mut stepper, mut sprite)) = hero.single_mut() else {
        return;
    };
    let (ex, ey) = (player.tile_x, player.tile_y);
    let pos = (ex, ey);
    let mut bodies = CollisionBodies::from_events(events.iter());
    bodies.include_vehicles(vehicles.as_deref(), data.map_id);
    let driven = {
        let collision = MapCollision::new(
            &data,
            &map_events,
            (&switches, &variables, &party, &inventory),
            &bodies,
        );
        let can_step = |_: &Player, dx: i32, dy: i32, jumping: bool, through: bool| {
            collision.can_move(pos, (ex + dx, ey + dy), Mover::hero(through), None, jumping)
        };
        drive(&mut *player, &mut queue, &mut stepper, pos, can_step)
    };
    // The hero is not a map event, so only its side effects need applying — its
    // tile is tracked by the `Player` component that `walk` updates.
    apply_effects(driven.effects, &mut switches, &mut audio, &mut sprite);
}

#[cfg(test)]
mod tests;

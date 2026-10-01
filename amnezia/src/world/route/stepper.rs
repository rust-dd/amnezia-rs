//! Character move routes, sharing the original stop count with autonomous movement.
//! Commands chain until movement starts or the current stop threshold blocks them.

use super::super::movement::{dir_delta, step_secs_for_speed};
use super::super::{Character, RouteAction};
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::stop_clock::{self, StopClock};
use amnezia_data::{MoveCommandDef, MoveRouteDef};
use bevy::prelude::Component;

mod attempt;
mod decode;
mod facing;
mod jump;
mod lifecycle;
mod saved;
mod stops;
mod turn;
pub(crate) use attempt::Attempt;
pub(crate) use turn::Boundary;
pub(crate) use turn::Progress;
pub(crate) use turn::Turn;

/// Logical frames per second the RM2000 stop-count delays are measured in.
const FPS: f32 = 60.0;

/// A side-effect a route command produces that the driving system applies to
/// shared state: a game-switch toggle (32/33), a sound effect (35), or a
/// transparency change (40/41) applied to the character's sprite.
pub(crate) enum StepEffect {
    Switch(u32, bool),
    Sound { name: String, params: [i32; 3] },
    Transparency(u8),
}

/// Every hero/NPC owns a stepper; custom-movement pages start with their route armed.
#[derive(Component, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RouteStepper {
    pub(crate) animation: crate::world::character_animation::CharacterAnimation,
    commands: Vec<MoveCommandDef>,
    index: usize,
    repeat: bool,
    skippable: bool,
    /// Live move speed (1–6) and frequency (1–8); route commands 28–31 change them.
    speed: u32,
    frequency: u32,
    /// Ignore passability while set (`walk everywhere`, commands 36/37).
    through: bool,
    #[serde(default)]
    route_through: Option<bool>,
    #[serde(default)]
    overlap_forbidden: Option<bool>,
    /// Transparency level 0 (opaque) … 7, adjusted by commands 40/41.
    transparency: u8,
    #[serde(default)]
    stop: Option<StopClock>,
    #[serde(
        default,
        rename = "timer",
        skip_serializing_if = "stop_clock::legacy_empty"
    )]
    legacy_timer: f32,
    rng: u32,
    active: bool,
    /// MoveEvent routes bypass event/message pauses; page routes follow autonomous-movement gates.
    forced: bool,
    facing_lock: Option<u32>,
    direction: Option<u32>,
    suspended: Option<lifecycle::Suspended>,
    page_present: bool,
    first_pass_complete: bool,
    moving: bool,
    #[serde(default, rename = "autonomy_reset", skip_serializing)]
    legacy_autonomy_reset: bool,
}

impl Default for RouteStepper {
    fn default() -> Self {
        Self::new(Vec::new(), false, false, 4, 3, false)
    }
}

impl RouteStepper {
    /// Apply commands until movement starts, the stop threshold blocks execution,
    /// or the route finishes. Zero-delay turns continue within the same update.
    /// Returns the movement and tween duration, collecting shared-state effects.
    /// The movement gate receives the live graphic and through state, including
    /// instant commands executed earlier in this same route tick.
    #[cfg(test)]
    pub(super) fn advance<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        can_step: &impl Fn(&C, i32, i32, bool, bool) -> bool,
        effects: &mut Vec<StepEffect>,
    ) -> Option<(RouteAction, f32)> {
        let mut turn = Turn::new(self);
        loop {
            match self.advance_turn(ch, hero, can_step, effects, &mut turn) {
                Progress::Done => return None,
                Progress::Refresh => {}
                Progress::Move(action, seconds) => return Some((action, seconds)),
            }
        }
    }

    fn step_one<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        effects: &mut Vec<StepEffect>,
        cmd: &MoveCommandDef,
        program: &MoveRouteDef,
        turn: &Turn,
    ) -> Step {
        match cmd.code {
            0..=7 => self.prepare_move(ch, Some(cmd.code), dir_delta(cmd.code)),
            8 => {
                let dir = self.random_dir();
                self.prepare_move(ch, Some(dir), dir_delta(dir))
            }
            9 => {
                let dir = toward_dir(hero, ch.tile());
                self.prepare_move(ch, Some(dir), dir_delta(dir))
            }
            10 => {
                let dir = away_dir(hero, ch.tile());
                self.prepare_move(ch, Some(dir), dir_delta(dir))
            }
            11 => self.prepare_move(ch, None, dir_delta(self.direction(ch))),
            12..=15 => self.turn_to(ch, cmd.code - 12),
            16 => self.turn(ch, 1),
            17 => self.turn(ch, 3),
            18 => self.turn(ch, 2),
            19 => {
                let step = if self.random_bit() { 1 } else { 3 };
                self.turn(ch, step)
            }
            20 => {
                let dir = self.random_dir();
                self.turn_to(ch, dir)
            }
            21 => self.turn_to(ch, toward_dir(hero, ch.tile())),
            22 => self.turn_to(ch, away_dir(hero, ch.tile())),
            23 => {
                self.set_stop_maximum(stop_clock::wait(self.frequency));
                self.set_stop_count(0);
                Step::Gate(None)
            }
            24 => self.begin_jump(ch, hero, program, turn),
            26 => {
                self.direction = Some(self.direction(ch));
                self.facing_lock = Some(ch.dir());
                Step::Next
            }
            27 => {
                self.facing_lock = None;
                Step::Next
            }
            28 => self.retune(&mut Self::adjust_speed, 1),
            29 => self.retune(&mut Self::adjust_speed, -1),
            30 => self.retune(&mut Self::adjust_freq, 1),
            31 => self.retune(&mut Self::adjust_freq, -1),
            32 => {
                effects.push(StepEffect::Switch(switch_id(cmd), true));
                Step::Next
            }
            33 => {
                effects.push(StepEffect::Switch(switch_id(cmd), false));
                Step::Next
            }
            34 => {
                let index = cmd.params.first().copied().unwrap_or(0).max(0) as u32;
                ch.set_graphic(cmd.string.clone(), index);
                Step::Next
            }
            35 => {
                effects.push(StepEffect::Sound {
                    name: cmd.string.clone(),
                    params: [
                        cmd.params.first().copied().unwrap_or(0),
                        cmd.params.get(1).copied().unwrap_or(0),
                        cmd.params.get(2).copied().unwrap_or(0),
                    ],
                });
                Step::Next
            }
            36 => {
                self.through = true;
                self.route_through = Some(true);
                Step::Next
            }
            37 => {
                self.through = false;
                self.route_through = Some(false);
                Step::Next
            }
            38 | 39 => {
                self.animation.paused = cmd.code == 38;
                Step::Next
            }
            40 => {
                self.transparency = (self.transparency + 1).min(7);
                effects.push(StepEffect::Transparency(self.transparency));
                Step::Next
            }
            41 => {
                self.transparency = self.transparency.saturating_sub(1);
                effects.push(StepEffect::Transparency(self.transparency));
                Step::Next
            }
            _ => Step::Next,
        }
    }

    fn turn<C: Character>(&mut self, ch: &mut C, quarters: u32) -> Step {
        self.turn_to(ch, (ch.dir() + quarters) % 4)
    }

    fn turn_to<C: Character>(&mut self, ch: &mut C, dir: u32) -> Step {
        self.direction = Some(dir);
        ch.set_dir(dir);
        self.gate_turn()
    }

    fn gate_turn(&mut self) -> Step {
        self.set_stop_maximum(stop_clock::turn(self.frequency));
        self.set_stop_count(0);
        Step::Gate(None)
    }

    fn retune(&mut self, apply: &mut impl FnMut(&mut Self, i32), delta: i32) -> Step {
        apply(self, delta);
        Step::Next
    }

    fn adjust_speed(&mut self, delta: i32) {
        self.speed = (self.speed as i32 + delta).clamp(1, 6) as u32;
    }

    fn adjust_freq(&mut self, delta: i32) {
        self.frequency = (self.frequency as i32 + delta).clamp(1, 8) as u32;
    }

    fn random_dir(&mut self) -> u32 {
        next_rand(&mut self.rng) % 4
    }

    fn random_bit(&mut self) -> bool {
        next_rand(&mut self.rng) & 1 == 0
    }
}

/// Movement or a new stop threshold, a blocked retry, or an instant command.
enum Step {
    Attempt(Attempt),
    Gate(Option<(RouteAction, f32)>),
    Retry,
    Next,
}

fn switch_id(cmd: &MoveCommandDef) -> u32 {
    cmd.params.first().copied().unwrap_or(0).max(0) as u32
}

/// The cardinal direction stepping toward `hero` from `me` (dominant axis first,
/// vertical on a tie), mirroring EasyRPG's `GetDirectionToCharacter`.
fn toward_dir(hero: (i32, i32), me: (i32, i32)) -> u32 {
    let (dx, dy) = (hero.0 - me.0, hero.1 - me.1);
    if dx.abs() > dy.abs() {
        if dx > 0 { DIR_RIGHT } else { DIR_LEFT }
    } else if dy != 0 {
        if dy > 0 { DIR_DOWN } else { DIR_UP }
    } else if dx != 0 {
        if dx > 0 { DIR_RIGHT } else { DIR_LEFT }
    } else {
        DIR_DOWN
    }
}

fn away_dir(hero: (i32, i32), me: (i32, i32)) -> u32 {
    (toward_dir(hero, me) + 2) % 4
}

/// Deterministic xorshift32 stream for random move/turn commands.
fn next_rand(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod packed_tests;

#[cfg(test)]
mod page_tests;

//! The [`RouteStepper`] itself: the RM2000 move-route state machine, decoded from
//! a page's custom route or a `MoveEvent` command, that a character advances one
//! command per idle tick. Mirrors EasyRPG's `Game_Character::UpdateMoveRoute`:
//! instant commands (speed/frequency/switch/graphic/sound/through/transparency)
//! run back-to-back, and the first gating command — a move, a turn, or a wait —
//! ends the tick. A blocked move is skipped on a `skippable` route and waited on
//! otherwise; a `repeat` route loops, a one-shot route stops at the end. The Bevy
//! systems that pump this each frame live in the parent module.

use super::super::movement::{dir_delta, step_secs_for_speed};
use super::super::{Character, RouteAction};
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use amnezia_data::{MoveCommandDef, MoveRouteDef};
use bevy::prelude::Component;

/// Logical frames per second the RM2000 stop-count delays are measured in.
const FPS: f32 = 60.0;

/// The `(dx, dy)` of a diagonal move command (4 up-right, 5 down-right, 6
/// down-left, 7 up-left).
const DIAGONALS: [(i32, i32); 4] = [(1, -1), (1, 1), (-1, 1), (-1, -1)];

/// A side-effect a route command produces that the driving system applies to
/// shared state: a game-switch toggle (32/33), a sound effect (35), or a
/// transparency change (40/41) applied to the character's sprite.
pub(crate) enum StepEffect {
    Switch(u32, bool),
    Sound { name: String, params: [i32; 3] },
    Transparency(u8),
}

/// A character's progress through a forced move route. Attached (inactive) to
/// every hero and event sprite so the `MoveEvent` opcode can load a route onto
/// any target; a `move_type == 6` NPC spawns with its page route already armed.
#[derive(Component)]
pub struct RouteStepper {
    commands: Vec<MoveCommandDef>,
    index: usize,
    repeat: bool,
    skippable: bool,
    /// Live move speed (1–6) and frequency (1–8); route commands 28–31 change them.
    speed: u32,
    frequency: u32,
    /// Ignore passability while set (`walk everywhere`, commands 36/37).
    through: bool,
    /// Transparency level 0 (opaque) … 7, adjusted by commands 40/41.
    transparency: u8,
    /// Seconds until the next command may run — RM2000's inter-command stop count.
    timer: f32,
    rng: u32,
    active: bool,
    /// True for a forced route from a `MoveEvent` (11330), false for a page's own
    /// `move_type == 6` custom route. A forced route keeps advancing while the event
    /// interpreter runs and while a message is up — RM2000's `IsMoveRouteOverwritten`
    /// short-circuits both pauses; a page route pauses like autonomous movement.
    forced: bool,
}

impl Default for RouteStepper {
    fn default() -> Self {
        Self::new(Vec::new(), false, false, 4, 3, false)
    }
}

impl RouteStepper {
    pub fn speed(&self) -> u32 {
        self.speed
    }

    pub fn with_speed(mut self, speed: u32) -> Self {
        self.speed = speed.clamp(1, 6);
        self
    }

    fn new(
        commands: Vec<MoveCommandDef>,
        repeat: bool,
        skippable: bool,
        speed: u32,
        frequency: u32,
        forced: bool,
    ) -> Self {
        Self {
            active: !commands.is_empty(),
            commands,
            index: 0,
            repeat,
            skippable,
            speed: speed.clamp(1, 6),
            frequency: frequency.clamp(1, 8),
            through: false,
            transparency: 0,
            timer: 0.0,
            rng: 0x9E37_79B9,
            forced,
        }
    }

    /// Arm a page's custom route (`move_type == 6`) at the page's speed/frequency.
    pub fn from_page(route: &MoveRouteDef, speed: u32, frequency: u32) -> Self {
        Self::new(
            route.commands.clone(),
            route.repeat,
            route.skippable,
            speed,
            frequency,
            false,
        )
    }

    /// Decode a `MoveEvent` (11330) route: `params = [char_ref, freq, repeat,
    /// skippable, commands…]`. An out-of-range frequency falls back to 6 (RM2000);
    /// the speed is the default hero pace (4), since the opcode carries none.
    pub fn from_move_event(params: &[i32]) -> Self {
        let freq = match params.get(1).copied().unwrap_or(6) {
            f @ 1..=8 => f as u32,
            _ => 6,
        };
        let repeat = params.get(2).copied().unwrap_or(0) != 0;
        let skippable = params.get(3).copied().unwrap_or(0) != 0;
        let tail = params.get(4..).unwrap_or(&[]);
        Self::new(decode_commands(tail), repeat, skippable, 4, freq, true)
    }

    /// Whether a route is loaded and still running — the flag the driving systems
    /// and the autonomy/player guards test to yield control to the stepper.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Whether this is a forced `MoveEvent` route (vs a page's custom route). The
    /// route drivers keep a forced route advancing during a running event or an open
    /// message while pausing a page route like autonomous movement.
    pub fn forced(&self) -> bool {
        self.forced
    }

    /// Count down the inter-command delay by `dt` and report whether the next
    /// command may run now. Called by the driving system only while the character
    /// stands idle, so the delay measures idle frames as RM2000's stop count does.
    pub(super) fn tick_ready(&mut self, dt: f32) -> bool {
        self.timer -= dt;
        self.timer <= 0.0
    }

    /// Advance the route: apply instant commands until the first gating command,
    /// mutating `ch`'s facing/graphic and pushing switch/sound/transparency
    /// `effects`. Returns the tile step to enqueue (with its tween seconds) for a
    /// move, or `None` for a turn/wait/finish/blocked-wait. `can_step(dx, dy)`
    /// reports whether the character may enter the tile at that delta.
    pub(super) fn advance<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        can_step: &impl Fn(i32, i32) -> bool,
        effects: &mut Vec<StepEffect>,
    ) -> Option<(RouteAction, f32)> {
        let len = self.commands.len();
        if len == 0 {
            self.active = false;
            return None;
        }
        // At most one full pass per call: instant commands chain, a gate returns,
        // and an all-instant route can't spin the loop forever.
        for _ in 0..=len {
            if self.index >= len {
                if !self.repeat {
                    self.active = false;
                    return None;
                }
                self.index = 0;
            }
            let cmd = self.commands[self.index].clone();
            match self.step_one(ch, hero, can_step, effects, &cmd) {
                Step::Gate(action) => {
                    self.index += 1;
                    return action;
                }
                Step::Retry => return None,
                Step::Next => self.index += 1,
            }
        }
        None
    }

    fn step_one<C: Character>(
        &mut self,
        ch: &mut C,
        hero: (i32, i32),
        can_step: &impl Fn(i32, i32) -> bool,
        effects: &mut Vec<StepEffect>,
        cmd: &MoveCommandDef,
    ) -> Step {
        match cmd.code {
            0..=3 => self.try_move(ch, Some(cmd.code), dir_delta(cmd.code), can_step),
            4..=7 => {
                let (dx, dy) = DIAGONALS[(cmd.code - 4) as usize];
                let face = if dy < 0 { DIR_UP } else { DIR_DOWN };
                self.try_move(ch, Some(face), (dx, dy), can_step)
            }
            8 => {
                let dir = self.random_dir();
                self.try_move(ch, Some(dir), dir_delta(dir), can_step)
            }
            9 => {
                let dir = toward_dir(hero, ch.tile());
                self.try_move(ch, Some(dir), dir_delta(dir), can_step)
            }
            10 => {
                let dir = away_dir(hero, ch.tile());
                self.try_move(ch, Some(dir), dir_delta(dir), can_step)
            }
            11 => self.try_move(ch, None, dir_delta(ch.dir()), can_step),
            12..=15 => {
                ch.set_dir(cmd.code - 12);
                self.gate_turn()
            }
            16 => self.turn(ch, 1),
            17 => self.turn(ch, 3),
            18 => self.turn(ch, 2),
            19 => {
                let step = if self.random_bit() { 1 } else { 3 };
                self.turn(ch, step)
            }
            20 => {
                let dir = self.random_dir();
                ch.set_dir(dir);
                self.gate_turn()
            }
            21 => {
                ch.set_dir(toward_dir(hero, ch.tile()));
                self.gate_turn()
            }
            22 => {
                ch.set_dir(away_dir(hero, ch.tile()));
                self.gate_turn()
            }
            23 => {
                self.timer = wait_delay_secs(self.frequency);
                Step::Gate(None)
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
                Step::Next
            }
            37 => {
                self.through = false;
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
            // Jump markers (24/25), facing lock (26/27), animation pause (38/39),
            // and any unknown code: no observable effect in this remake.
            _ => Step::Next,
        }
    }

    /// Attempt a move: face `new_dir` (unless `None`, i.e. move-forward), then step
    /// the delta if passable (or `through`). A blocked move on a `skippable` route
    /// un-turns and skips to the next command; otherwise it faces the obstacle and
    /// waits, retrying after the step delay.
    fn try_move<C: Character>(
        &mut self,
        ch: &mut C,
        new_dir: Option<u32>,
        (dx, dy): (i32, i32),
        can_step: &impl Fn(i32, i32) -> bool,
    ) -> Step {
        let prev = ch.dir();
        if let Some(dir) = new_dir {
            ch.set_dir(dir);
        }
        let face = ch.dir();
        if self.through || can_step(dx, dy) {
            self.timer = step_delay_secs(self.frequency);
            Step::Gate(Some((
                RouteAction::Step { dx, dy, face },
                step_secs_for_speed(self.speed),
            )))
        } else if self.skippable {
            ch.set_dir(prev);
            Step::Next
        } else {
            self.timer = step_delay_secs(self.frequency);
            Step::Retry
        }
    }

    fn turn<C: Character>(&mut self, ch: &mut C, quarters: u32) -> Step {
        ch.set_dir((ch.dir() + quarters) % 4);
        self.gate_turn()
    }

    fn gate_turn(&mut self) -> Step {
        self.timer = turn_delay_secs(self.frequency);
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

/// A single command's outcome for [`RouteStepper::advance`]: gate this tick
/// (optionally enqueuing a step), retry the same command next tick (blocked and
/// not skippable), or move straight on to the next command.
enum Step {
    Gate(Option<(RouteAction, f32)>),
    Retry,
    Next,
}

/// Decode a `MoveEvent` command tail (the ints after `[ref, freq, repeat, skip]`)
/// into normalized [`MoveCommandDef`]s: switch (32/33) carries one arg,
/// change-graphic (34) a length-prefixed name (one byte per int) then the frame,
/// play-SE (35) the name then three args. Every other code has no tail.
fn decode_commands(stream: &[i32]) -> Vec<MoveCommandDef> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < stream.len() {
        let code = stream[i].max(0) as u32;
        i += 1;
        let (params, string) = match code {
            32 | 33 => {
                let a = stream.get(i).copied().unwrap_or(0);
                i += 1;
                (vec![a], String::new())
            }
            34 => {
                let (name, frame) = read_named(stream, &mut i, 1);
                (vec![frame[0]], name)
            }
            35 => {
                let (name, args) = read_named(stream, &mut i, 3);
                (args, name)
            }
            _ => (Vec::new(), String::new()),
        };
        out.push(MoveCommandDef {
            code,
            params,
            string,
        });
    }
    out
}

/// Read a `[len][name bytes]` string then `arg_count` trailing ints from an int
/// stream, advancing the cursor. Used by change-graphic and play-SE decoding.
fn read_named(stream: &[i32], i: &mut usize, arg_count: usize) -> (String, Vec<i32>) {
    let len = stream.get(*i).copied().unwrap_or(0).max(0) as usize;
    *i += 1;
    let end = (*i + len).min(stream.len());
    let name: String = stream[*i..end].iter().map(|&b| b as u8 as char).collect();
    *i = end;
    let mut args = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        args.push(stream.get(*i).copied().unwrap_or(0));
        *i += 1;
    }
    (name, args)
}

fn switch_id(cmd: &MoveCommandDef) -> u32 {
    cmd.params.first().copied().unwrap_or(0).max(0) as u32
}

/// RM2000 `GetMaxStopCountForStep`: frames between route steps at `freq` (1 slow …
/// 8 fast), as seconds. This is the extra idle delay *after* a step's tween.
fn step_delay_secs(freq: u32) -> f32 {
    stop_frames(freq, 9) as f32 / FPS
}

/// RM2000 `GetMaxStopCountForTurn`: a turn's idle delay (half a step's).
fn turn_delay_secs(freq: u32) -> f32 {
    stop_frames(freq, 8) as f32 / FPS
}

/// RM2000 `GetMaxStopCountForWait`: 20 frames plus a turn's delay.
fn wait_delay_secs(freq: u32) -> f32 {
    (20 + stop_frames(freq, 8)) as f32 / FPS
}

fn stop_frames(freq: u32, base_shift: u32) -> u32 {
    let f = freq.clamp(1, 8);
    if f >= 8 { 0 } else { 1 << (base_shift - f) }
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

/// xorshift32 — cheap deterministic randomness for the random move/turn commands,
/// no `rand` dependency.
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

//! The event interpreter: runs an active event page's RM2000 command list one
//! step at a time. It drives the existing message box for dialogue and the
//! existing teleport fade for map transfers, and reads/writes the game's
//! switches and variables. A page's commands are a flat list with a per-command
//! `indent`; conditional branches use that indent to delimit their bodies.

use crate::animation::ShowMapAnimation;
use crate::appearance::SpriteChange;
use crate::audio::AudioRequest;
use crate::battle::{BattleActive, BattleOutcome, BattleRequest};
use crate::choice::Choice;
use crate::dialogue::{Dialogue, MessagePosition};
use crate::events::message_boxes;
use crate::gameover::GameOverActive;
use crate::menu::MenuOpen;
use crate::player::Player;
use crate::screenfx::Weather;
use crate::shop::{ShopOpen, ShopRequest};
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::teleport::{Fade, PendingTeleport};
use crate::text;
use crate::title::TitleActive;
use crate::world::{EventSprite, MapEvents, MoveQueue, RelocateEvent, decode_route};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::collections::HashMap;

mod actor_query;
mod branch;
mod commands;
mod control_vars;
mod event_rng;
mod flow;
mod opcodes;
mod params;
mod present;

use actor_query::ActorCtx;
use branch::branch_holds;
use commands::*;
use control_vars::{apply_control_variables, resolve_operand};
use event_rng::EventRng;
use flow::*;
use opcodes::*;
use params::{Blockers, IntroGuard, SubsystemIo};
use present::{Present, parse_present};

/// A frame-local cap on executed commands, so a malformed list (e.g. a branch
/// that never advances) can't lock up the frame. Well-formed pages never
/// approach it — every command strictly advances the instruction pointer.
const MAX_STEPS_PER_FRAME: usize = 10_000;

/// The maximum nested `CallEvent` depth, guarding a page that calls itself (or a
/// cycle of pages) from growing the call stack without bound.
const MAX_CALL_DEPTH: usize = 64;

/// A caller frame suspended by `CallEvent` (12330): the interrupted command list,
/// the instruction pointer to resume at, and the event id in scope. The callee
/// runs in place; reaching its end pops the frame and resumes the caller.
struct CallFrame {
    commands: Vec<EventCommand>,
    ip: usize,
    event_id: u32,
}

/// The event currently executing: its command list, the instruction pointer,
/// whether a run is live, and the remaining `Wait` countdown in seconds.
#[derive(Resource, Default)]
pub struct RunningEvent {
    commands: Vec<EventCommand>,
    ip: usize,
    active: bool,
    wait: f32,
    wait_move: bool,
    event_id: u32,
    choices: HashMap<u32, i32>,
    /// Suspended caller frames from `CallEvent`; a finished callee pops back to the
    /// top frame, and the run ends only when the stack is empty.
    call_stack: Vec<CallFrame>,
    /// Set while a waiting `KeyInputProc` (11610) holds the event: each frame's
    /// poll writes the pressed key's RM2000 code into `key_var` and resumes.
    key_pending: bool,
    key_var: u32,
    key_accept: KeyAccept,
    /// Set while a `BattleRequest` is in flight: holds the event paused across the
    /// fight until [`BattleResult`] is published, then consumed into `battle_outcome`.
    battle_pending: bool,
    /// The finished fight's outcome while an `EnemyEncounter` block runs its
    /// handlers; the matching Victory/Escape/Defeat body executes, `EndBattle` clears it.
    battle_outcome: Option<BattleOutcome>,
    /// Set while a shop/inn screen is open: holds the event paused until
    /// [`ShopOpen`] clears, then the block is skipped to its terminator.
    shop_pending: bool,
    /// Set while an `InputNumber` box is open: holds the event paused until the
    /// player confirms, then the entered value is written to the target variable.
    input_pending: bool,
    /// The merchant screen's result while a shop/inn block runs its handlers:
    /// Transaction/Stay (`true`) or NoTransaction/Cancel (`false`) self-selects,
    /// then `EndShop`/`EndInn` clears it. Mirrors `battle_outcome`.
    shop_transacted: Option<bool>,
}

impl RunningEvent {
    /// Whether an event is currently executing. Triggers and movement pause
    /// while this holds.
    pub fn active(&self) -> bool {
        self.active
    }

    /// The running event's id for the debug HUD (`None` when idle).
    pub fn debug_id(&self) -> Option<u32> {
        self.active.then_some(self.event_id)
    }

    /// Begin running `commands` from the top. Ignored if a run is already live,
    /// so one event can't interrupt another mid-sequence.
    pub fn start(&mut self, event_id: u32, commands: Vec<EventCommand>) {
        if self.active {
            return;
        }
        self.commands = commands;
        self.ip = 0;
        self.wait = 0.0;
        self.wait_move = false;
        self.event_id = event_id;
        self.choices.clear();
        self.call_stack.clear();
        self.key_pending = false;
        self.key_var = 0;
        self.key_accept = KeyAccept::default();
        self.battle_pending = false;
        self.battle_outcome = None;
        self.shop_pending = false;
        self.input_pending = false;
        self.shop_transacted = None;
        self.active = true;
    }

    fn stop(&mut self) {
        self.active = false;
        self.commands.clear();
        self.ip = 0;
        self.wait = 0.0;
        self.wait_move = false;
        self.event_id = 0;
        self.choices.clear();
        self.call_stack.clear();
        self.key_pending = false;
        self.key_var = 0;
        self.key_accept = KeyAccept::default();
        self.battle_pending = false;
        self.battle_outcome = None;
        self.shop_pending = false;
        self.input_pending = false;
        self.shop_transacted = None;
    }

    /// Self-select an `EnemyEncounter` outcome handler: run its body (advance into
    /// it) when the finished battle's outcome is `want`, otherwise skip to the next
    /// handler or the block terminator. Mirrors the `ShowChoice` option arms.
    fn select_battle_handler(&mut self, indent: u32, want: BattleOutcome) {
        if self.battle_outcome == Some(want) {
            self.ip += 1;
        } else {
            self.ip = skip_battle_handler(&self.commands, self.ip, indent);
        }
    }

    /// Self-select a shop/inn outcome handler: run its body when the merchant
    /// result matches `want` (Transaction/Stay = `true`, NoTransaction/Cancel =
    /// `false`), else skip to the next handler or the block terminator.
    fn select_shop_handler(&mut self, indent: u32, want: bool) {
        if self.shop_transacted == Some(want) {
            self.ip += 1;
        } else {
            self.ip = skip_battle_handler(&self.commands, self.ip, indent);
        }
    }
}

pub struct InterpreterPlugin;

impl Plugin for InterpreterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunningEvent>()
            .init_resource::<EventRng>()
            .add_systems(Update, (autorun, run_interpreter, clear_move_wait).chain());
    }
}

/// Execute the running event, one burst of commands per frame. Pauses while a
/// message box is open, a teleport fade is running, or a `Wait` is counting
/// down; resumes automatically once the block clears.
#[allow(clippy::too_many_arguments)]
fn run_interpreter(
    time: Res<Time>,
    fade: Res<Fade>,
    blockers: Blockers,
    mut running: ResMut<RunningEvent>,
    mut dialogue: ResMut<Dialogue>,
    mut choice: ResMut<Choice>,
    mut switches: ResMut<Switches>,
    mut variables: ResMut<Variables>,
    mut inventory: ResMut<Inventory>,
    mut party: ResMut<Party>,
    mut pending: ResMut<PendingTeleport>,
    mut hero_queue: Query<&mut MoveQueue, With<Player>>,
    mut event_movers: Query<(&EventSprite, &mut MoveQueue), Without<Player>>,
    mut audio: MessageWriter<AudioRequest>,
    mut subsystems: SubsystemIo,
) {
    if !running.active {
        return;
    }
    // Resume after a fight: consume the published outcome and drop the pause, so
    // the EnemyEncounter re-executes past the trigger and its handlers self-select.
    // A wipe the encounter can't recover from (no DefeatHandler) ends the game.
    // Runs before the guard because `battle_pending` is part of it.
    if running.battle_pending
        && let Some(outcome) = subsystems.battle_result.0.take()
    {
        running.battle_pending = false;
        let indent = running.commands.get(running.ip).map_or(0, |c| c.indent);
        if outcome == BattleOutcome::Defeat
            && !has_defeat_handler(&running.commands, running.ip, indent)
        {
            subsystems.gameover.0 = true;
            running.stop();
            return;
        }
        running.battle_outcome = Some(outcome);
    }
    // Pause while a message box, teleport fade, choice, pending transfer, blocking
    // overlay, or an in-flight fight is live; the transfer guard holds the event
    // across the fade so it resumes on the destination map (RM2000 Transfer Player
    // continues the calling event).
    if dialogue.active
        || fade.busy()
        || choice.active()
        || pending.0.is_some()
        || blockers.any()
        || subsystems.flow.title.0
        || running.battle_pending
        || subsystems.gameover.0
        || subsystems.input_number.active()
    {
        return;
    }
    // Resume after the player confirmed a choice: record the pick so the
    // ShowChoice re-executes past the menu and the options self-select.
    if let Some(result) = choice.result.take() {
        running.choices.insert(choice.indent, result);
    }
    // Resume after a merchant screen closes (past the guard means `ShopOpen` is
    // clear): skip the whole shop/inn block to its terminator, mirroring how an
    // empty ShowChoice skips itself. The shop transacts on the inventory directly,
    // so neither branch body runs.
    if running.shop_pending {
        // The merchant screen closed: record whether a trade happened and step into
        // the block so the Transaction/Stay (or NoTransaction/Cancel) handler arms
        // self-select, mirroring the battle-outcome handlers.
        running.shop_transacted = Some(subsystems.merchant.outcome.transacted);
        running.shop_pending = false;
        running.ip += 1;
    }
    // Resume after the player entered a number: store it in the target variable,
    // then step past the InputNumber command.
    if running.input_pending {
        if let Some(value) = subsystems.input_number.result.take() {
            variables.set(subsystems.input_number.var_id, value as i32);
        }
        running.input_pending = false;
        running.ip += 1;
    }
    // Resume a waiting KeyInputProc: poll the accepted keys and, once one is
    // pressed, store its RM2000 code in the target variable and step past the
    // command; otherwise keep the event paused for another frame.
    if running.key_pending {
        let keys = &subsystems.flow.keys;
        let code = key_code(
            &running.key_accept,
            keys.just_pressed(KeyCode::ArrowUp),
            keys.just_pressed(KeyCode::ArrowDown),
            keys.just_pressed(KeyCode::ArrowLeft),
            keys.just_pressed(KeyCode::ArrowRight),
            keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
            keys.just_pressed(KeyCode::Escape),
            keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight),
        );
        if code == 0 {
            return;
        }
        variables.set(running.key_var, code);
        running.key_pending = false;
        running.ip += 1;
    }
    if running.wait > 0.0 {
        running.wait -= time.delta_secs();
        return;
    }
    // Hold until the character a `MoveEvent` set walking has drained its queue.
    if running.wait_move {
        return;
    }
    for _ in 0..MAX_STEPS_PER_FRAME {
        let Some(command) = running.commands.get(running.ip).cloned() else {
            // A callee finished: pop back to the caller frame and resume it; the
            // run ends only when there is no caller left to return to.
            if let Some(frame) = running.call_stack.pop() {
                running.commands = frame.commands;
                running.ip = frame.ip;
                running.event_id = frame.event_id;
                continue;
            }
            running.stop();
            return;
        };
        match command.code {
            SHOW_MESSAGE | SHOW_MESSAGE_2 | CHANGE_FACE => {
                let run_len = running.commands[running.ip..]
                    .iter()
                    .take_while(|c| is_message(c.code))
                    .count();
                let mut boxes = message_boxes(&running.commands[running.ip..running.ip + run_len]);
                running.ip += run_len;
                if !boxes.is_empty() {
                    // Translate each line but keep its RM2000 control codes intact:
                    // the dialogue typewriter expands `\N`/`\V` and acts on the
                    // reveal-timing codes (`\s`, `\|`, `\^`, …) as it types the page.
                    for message in &mut boxes {
                        for line in &mut message.lines {
                            *line = crate::i18n::tr(line);
                        }
                    }
                    dialogue.open(boxes);
                    return;
                }
            }
            CONTROL_SWITCHES => {
                apply_control_switches(&mut switches, &command.params);
                running.ip += 1;
            }
            CONTROL_VARIABLES => {
                // Resolve the operand — constant / variable / var-of-var / random /
                // item / actor / character / other — from the live state, then
                // assign it under the command's target mode and operation. Only the
                // character operand (type 6) needs the world queries, resolved here.
                let character = if command.params.get(4).copied() == Some(6) {
                    resolve_character(
                        command.params.get(5).copied().unwrap_or(0),
                        running.event_id,
                        &subsystems.flow.players,
                        &event_movers,
                    )
                } else {
                    None
                };
                let actors = ActorCtx {
                    data: &subsystems.actor_edits.game_data,
                    progression: &subsystems.actor_edits.progression,
                    vitals: &subsystems.vitals,
                    hero_name: &subsystems.actor_edits.hero_name.0,
                };
                let operand = resolve_operand(
                    &command.params,
                    &variables,
                    &inventory,
                    &party,
                    &actors,
                    subsystems.mapfx.game_clock.seconds(),
                    character,
                );
                apply_control_variables(
                    &mut variables,
                    &mut subsystems.event_rng,
                    &command.params,
                    operand,
                );
                running.ip += 1;
            }
            CHANGE_GOLD => {
                apply_change_gold(&mut inventory, &command.params);
                running.ip += 1;
            }
            CHANGE_ITEMS => {
                apply_change_items(&mut inventory, &command.params);
                running.ip += 1;
            }
            CHANGE_PARTY => {
                apply_change_party(
                    &mut party,
                    &subsystems.actor_edits.game_data,
                    &command.params,
                );
                running.ip += 1;
            }
            FULL_HEAL => {
                // Restore the whole party (observed `params` is always `[0, 0]`).
                subsystems.vitals.heal_all();
                running.ip += 1;
            }
            INPUT_NUMBER => {
                // Open the numeric entry and pause; the resume above stores the
                // result and advances once the player confirms.
                let digits = command.params.first().copied().unwrap_or(0).max(0) as u32;
                let var_id = command.params.get(1).copied().unwrap_or(0) as u32;
                subsystems.input_number.open(digits, var_id);
                running.input_pending = true;
                return;
            }
            CHANGE_SPRITE => {
                // Reskin an actor; the appearance system retextures the hero when
                // the target is the party lead. `string` = charset name.
                let actor_id = command.params.first().copied().unwrap_or(0).max(0) as u32;
                let index = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
                subsystems.visuals.sprite_writer.write(SpriteChange {
                    actor_id,
                    charset: command.string.clone(),
                    index,
                });
                running.ip += 1;
            }
            CHANGE_EVENT_LOCATION => {
                // Move an event to a tile. `params = [event_ref, mode, x, y]`;
                // mode 1 reads the coords from variables. event_ref 10005 = this
                // event, N = event id (10001 = hero, not relocated here).
                let event_ref = command.params.first().copied().unwrap_or(0);
                let mode = command.params.get(1).copied().unwrap_or(0);
                let rx = command.params.get(2).copied().unwrap_or(0);
                let ry = command.params.get(3).copied().unwrap_or(0);
                let (x, y) = if mode == 1 {
                    (variables.get(rx as u32), variables.get(ry as u32))
                } else {
                    (rx, ry)
                };
                let event_id = if event_ref == 10005 {
                    running.event_id as i32
                } else {
                    event_ref
                };
                if event_ref != 10001 && event_id > 0 && x >= 0 && y >= 0 {
                    subsystems.relocate_writer.write(RelocateEvent {
                        event_id: event_id as u32,
                        x: x as u32,
                        y: y as u32,
                    });
                }
                running.ip += 1;
            }
            MESSAGE_OPTIONS => {
                subsystems.mapfx.message_transparent.0 =
                    command.params.first().copied().unwrap_or(0) != 0;
                *subsystems.mapfx.message_position =
                    match command.params.get(1).copied().unwrap_or(2) {
                        0 => MessagePosition::Top,
                        1 => MessagePosition::Middle,
                        _ => MessagePosition::Bottom,
                    };
                running.ip += 1;
            }
            TIMER => {
                match command.params.first().copied().unwrap_or(0) {
                    0 => subsystems
                        .mapfx
                        .game_clock
                        .set_secs(command.params.get(2).copied().unwrap_or(0).max(0) as u32),
                    1 => subsystems.mapfx.game_clock.start(),
                    2 => subsystems.mapfx.game_clock.stop(),
                    _ => {}
                }
                running.ip += 1;
            }
            PAN_SCREEN => {
                // op 2 = pan by `dist` tiles in `dir`; op 3 = return; lock/unlock ignored.
                let op = command.params.first().copied().unwrap_or(0);
                if op == 3 {
                    subsystems.mapfx.camera_pan.target = Vec2::ZERO;
                } else if op == 2 {
                    let dist = command.params.get(2).copied().unwrap_or(0).max(0) as f32
                        * crate::tiles::TILE;
                    let speed = command.params.get(3).copied().unwrap_or(4).max(1) as f32;
                    let delta = match command.params.get(1).copied().unwrap_or(0) {
                        0 => Vec2::new(0.0, dist),
                        1 => Vec2::new(dist, 0.0),
                        2 => Vec2::new(0.0, -dist),
                        _ => Vec2::new(-dist, 0.0),
                    };
                    subsystems.mapfx.camera_pan.target += delta;
                    subsystems.mapfx.camera_pan.speed = speed * crate::tiles::TILE * 2.0;
                }
                running.ip += 1;
            }
            WEATHER => {
                *subsystems.mapfx.weather =
                    Weather::from_code(command.params.first().copied().unwrap_or(0));
                subsystems.mapfx.weather_strength.0 = command.params.get(1).copied().unwrap_or(0);
                running.ip += 1;
            }
            PLAYER_TRANSPARENCY => {
                subsystems.mapfx.hero_transparency.0 =
                    u8::from(command.params.first().copied().unwrap_or(0) != 0) * 7;
                running.ip += 1;
            }
            SHOW_BATTLE_ANIMATION => {
                // `params = [anim_id, target_char_ref, wait, global]`. Decode the
                // id, resolve the char-ref exactly like 10860/11330, and emit the
                // request (the resolver projects the target to a screen position, so
                // no camera/transform query is needed here). `global` (params[3])
                // tiles the animation 3×3 across the screen; `wait` (params[2])
                // blocks the event for the animation's duration, mirroring EasyRPG
                // `CommandShowBattleAnimation`. A short `params` or an unresolvable
                // target simply no-ops (and never waits, matching a NULL character).
                let global = command.params.get(3).copied().unwrap_or(0) > 0;
                let wait = if let [anim_id, target_ref, ..] = command.params.as_slice()
                    && *anim_id >= 0
                    && let Some(target) = resolve_anim_target(*target_ref, running.event_id)
                {
                    let anim_id = *anim_id as u32;
                    subsystems.visuals.anim_writer.write(ShowMapAnimation {
                        anim_id,
                        target,
                        global,
                    });
                    let frames = anim_frame_count(&subsystems.visuals.library, anim_id);
                    battle_anim_wait(&command.params, frames)
                } else {
                    None
                };
                running.ip += 1;
                if let Some(secs) = wait {
                    running.wait = secs;
                    return;
                }
            }
            TRANSACTION | INN_STAY => running.select_shop_handler(command.indent, true),
            NO_TRANSACTION | INN_CANCEL => running.select_shop_handler(command.indent, false),
            END_SHOP | END_INN => {
                running.shop_transacted = None;
                running.ip += 1;
            }
            WAIT => {
                running.wait = command.params.first().copied().unwrap_or(0) as f32 / 10.0;
                running.ip += 1;
                return;
            }
            TELEPORT => {
                // Queue the transfer and pause (via the pending guard) until the
                // fade swaps the map; the event then resumes at the next command
                // on the destination map rather than terminating here.
                if let [map, x, y, ..] = command.params.as_slice() {
                    pending.0 = Some((*map as u32, *x as u32, *y as u32));
                }
                running.ip += 1;
                return;
            }
            CONDITIONAL_BRANCH => {
                // Timer conditional (type 2) compares the running clock; the rest
                // are state checks in `branch_holds`. The actor sub-checks (type 5)
                // read from `ActorCtx`, and the orientation check (type 6) needs the
                // referenced character's real facing, resolved here from the world.
                let kind = command.params.first().copied().unwrap_or(-1);
                let holds = if kind == 2 {
                    let target = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
                    let secs = subsystems.mapfx.game_clock.seconds();
                    if command.params.get(2).copied().unwrap_or(0) == 0 {
                        secs >= target
                    } else {
                        secs <= target
                    }
                } else {
                    let facing = if kind == 6 {
                        resolve_character(
                            command.params.get(1).copied().unwrap_or(0),
                            running.event_id,
                            &subsystems.flow.players,
                            &event_movers,
                        )
                        .map(|(_, _, dir)| dir)
                    } else {
                        None
                    };
                    let actors = ActorCtx {
                        data: &subsystems.actor_edits.game_data,
                        progression: &subsystems.actor_edits.progression,
                        vitals: &subsystems.vitals,
                        hero_name: &subsystems.actor_edits.hero_name.0,
                    };
                    branch_holds(
                        &command.params,
                        &command.string,
                        &switches,
                        &variables,
                        &party,
                        &inventory,
                        &actors,
                        facing,
                    )
                };
                if holds {
                    running.ip += 1;
                } else {
                    running.ip = skip_true_body(&running.commands, running.ip, command.indent);
                }
            }
            ELSE_BRANCH => {
                running.ip = skip_else_body(&running.commands, running.ip, command.indent);
            }
            END_BRANCH => {
                // The block terminator does nothing; flow continues past it.
                running.ip += 1;
            }
            LABEL => running.ip += 1,
            JUMP_TO_LABEL => {
                let id = command.params.first().copied().unwrap_or(0);
                running.ip = find_label(&running.commands, id).unwrap_or(running.ip + 1);
            }
            LOOP => running.ip += 1,
            END_LOOP => running.ip = loop_start(&running.commands, running.ip, command.indent),
            BREAK_LOOP => {
                running.ip = after_loop_end(&running.commands, running.ip, command.indent)
            }
            SHOW_CHOICE => {
                if running.choices.contains_key(&command.indent) {
                    running.ip += 1;
                } else {
                    let labels: Vec<String> =
                        choice_labels(&running.commands, running.ip, command.indent)
                            .iter()
                            .map(|l| {
                                text::substitute(
                                    &crate::i18n::tr(l),
                                    &subsystems.actor_edits.hero_name.0,
                                    &variables,
                                )
                            })
                            .collect();
                    if labels.is_empty() {
                        running.ip = skip_to_terminator(
                            &running.commands,
                            running.ip,
                            command.indent,
                            SHOW_CHOICE_END,
                        );
                    } else {
                        // RM2000 `ShowChoices` cancel type is `parameters[0]`.
                        let cancel_type = command.params.first().copied().unwrap_or(0);
                        choice.open(labels, command.indent, cancel_type);
                        return;
                    }
                }
            }
            SHOW_CHOICE_OPTION => {
                let want = running.choices.get(&command.indent).copied().unwrap_or(-1);
                if command.params.first().copied() == Some(want) {
                    running.ip += 1;
                } else {
                    running.ip = skip_option_body(&running.commands, running.ip, command.indent);
                }
            }
            SHOW_CHOICE_END => {
                running.choices.remove(&command.indent);
                running.ip += 1;
            }
            MOVE_EVENT => {
                // Enqueue the route onto its target and pause until it drains.
                // `10001` is the hero, `10005` this event, else an event id.
                let target = command.params.first().copied().unwrap_or(0);
                let steps = decode_route(&command.params);
                if target == 10001 {
                    if let Ok(mut queue) = hero_queue.single_mut() {
                        queue.enqueue_route(steps);
                    }
                } else {
                    let id = if target == 10005 {
                        running.event_id as i32
                    } else {
                        target
                    };
                    if let Some((_, mut queue)) =
                        event_movers.iter_mut().find(|(e, _)| e.id as i32 == id)
                    {
                        queue.enqueue_route(steps);
                    }
                }
                running.wait_move = true;
                running.ip += 1;
                return;
            }
            ENEMY_ENCOUNTER => {
                // Resumed (outcome in hand): fall into the handlers. Fresh: start
                // the fight and pause (via `battle_pending`) until it publishes a
                // result; `params[1]` is the troop id (`params[0]` is always 0).
                if running.battle_outcome.is_some() {
                    running.ip += 1;
                } else {
                    let troop_id = command.params.get(1).copied().unwrap_or(0) as u32;
                    subsystems.battle_writer.write(BattleRequest { troop_id });
                    running.battle_pending = true;
                    return;
                }
            }
            VICTORY_HANDLER => {
                running.select_battle_handler(command.indent, BattleOutcome::Victory)
            }
            ESCAPE_HANDLER => running.select_battle_handler(command.indent, BattleOutcome::Escape),
            DEFEAT_HANDLER => running.select_battle_handler(command.indent, BattleOutcome::Defeat),
            END_BATTLE => {
                running.battle_outcome = None;
                running.ip += 1;
            }
            OPEN_SHOP => {
                // `params[0]` is the mode (0 buy+sell, 1 buy-only, 2 sell-only),
                // `params[1]` the shop type (shopkeeper wording), and `params[4..]`
                // the offered item ids (negatives dropped). Pause until the screen
                // closes, then the shop-resume above skips the block and its
                // Transaction/NoTransaction handlers self-select.
                let (allow_buy, allow_sell) = match command.params.first().copied().unwrap_or(0) {
                    1 => (true, false),
                    2 => (false, true),
                    _ => (true, true),
                };
                let shop_type = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
                let items = command
                    .params
                    .iter()
                    .skip(4)
                    .filter(|&&p| p >= 0)
                    .map(|&p| p as u32)
                    .collect();
                subsystems.merchant.writer.write(ShopRequest::OpenShop {
                    items,
                    allow_buy,
                    allow_sell,
                    shop_type,
                });
                running.shop_pending = true;
                return;
            }
            SHOW_INN => {
                // Charge `params[1]` gold to rest; pause as for the shop.
                let cost = command.params.get(1).copied().unwrap_or(0);
                subsystems
                    .merchant
                    .writer
                    .write(ShopRequest::ShowInn { cost });
                running.shop_pending = true;
                return;
            }
            PLAY_SOUND => {
                audio.write(AudioRequest::play_sound(&command.string, &command.params));
                running.ip += 1;
            }
            PLAY_BGM => {
                audio.write(AudioRequest::play_bgm(&command.string, &command.params));
                running.ip += 1;
            }
            FADE_OUT_BGM => {
                // params[0] is the fade time in ms: ramp the BGM to silence over
                // it rather than cutting instantly.
                audio.write(AudioRequest::fade_out(&command.params));
                running.ip += 1;
            }
            MEMORIZE_BGM => {
                audio.write(AudioRequest::MemorizeBgm);
                running.ip += 1;
            }
            PLAY_MEMORIZED_BGM => {
                audio.write(AudioRequest::PlayMemorizedBgm);
                running.ip += 1;
            }
            ERASE_SCREEN | SHOW_SCREEN | TINT_SCREEN | FLASH_SCREEN | SHAKE_SCREEN
            | SHOW_PICTURE | MOVE_PICTURE | ERASE_PICTURE | GAME_OVER => {
                // Emit the screen/picture message the presentation plugins consume,
                // waiting (as `Wait` does) when the effect must finish before the
                // next command; Game Over ends the run.
                match parse_present(&command, &variables) {
                    Some(Present::Screen(effect, wait)) => {
                        subsystems.screen_writer.write(effect);
                        running.ip += 1;
                        if let Some(secs) = wait {
                            running.wait = secs;
                            return;
                        }
                    }
                    Some(Present::Picture(picture, wait)) => {
                        subsystems.picture_writer.write(picture);
                        running.ip += 1;
                        if let Some(secs) = wait {
                            running.wait = secs;
                            return;
                        }
                    }
                    Some(Present::GameOver) => {
                        subsystems.gameover.0 = true;
                        running.stop();
                        return;
                    }
                    None => running.ip += 1,
                }
            }
            OPEN_SAVE_MENU => {
                // Request a single-slot save. `save_or_load` performs it even
                // though this event is still `running.active()` — only a fade
                // defers it — so the save crystal saves while its page runs.
                subsystems.event_save.0 = true;
                running.ip += 1;
            }
            CHANGE_LEVEL => {
                apply_change_level(
                    &mut subsystems.actor_edits.progression,
                    &subsystems.actor_edits.game_data,
                    &command.params,
                    &variables,
                    &party,
                );
                running.ip += 1;
            }
            CHANGE_HERO_NAME => {
                // Set the hero's (actor 1's) display name to `string`; the remake
                // tracks no live name for other actors, so those are left as-is.
                if command.params.first().copied() == Some(1) {
                    subsystems.actor_edits.hero_name.0 = command.string.clone();
                }
                running.ip += 1;
            }
            MEMORIZE_LOCATION => {
                // Store the current map id and the hero's tile into three variables.
                if let [vm, vx, vy, ..] = command.params.as_slice()
                    && let Some(map) = subsystems.flow.map_data.as_deref()
                    && let Ok(player) = subsystems.flow.players.single()
                {
                    variables.set(*vm as u32, map.map_id as i32);
                    variables.set(*vx as u32, player.tile_x);
                    variables.set(*vy as u32, player.tile_y);
                }
                running.ip += 1;
            }
            RECALL_TO_LOCATION => {
                // Teleport the hero to the map/tile memorized into these variables,
                // pausing through the fade like TELEPORT.
                if let [vm, vx, vy, ..] = command.params.as_slice() {
                    let (map, x, y) = (
                        variables.get(*vm as u32),
                        variables.get(*vx as u32),
                        variables.get(*vy as u32),
                    );
                    if map > 0 && x >= 0 && y >= 0 {
                        pending.0 = Some((map as u32, x as u32, y as u32));
                    }
                }
                running.ip += 1;
                return;
            }
            HALT_ALL_MOVEMENT => {
                // Cancel every character's pending forced movement.
                if let Ok(mut queue) = hero_queue.single_mut() {
                    *queue = MoveQueue::default();
                }
                for (_, mut queue) in &mut event_movers {
                    *queue = MoveQueue::default();
                }
                running.wait_move = false;
                running.ip += 1;
            }
            KEY_INPUT_PROC => {
                let var_id = command.params.first().copied().unwrap_or(0).max(0) as u32;
                let wait = command.params.get(1).copied().unwrap_or(0) != 0;
                let accept = decode_key_accept(&command.params);
                if wait {
                    // Reset the target and pause; the resume block stores the pressed
                    // key's code and advances once a key comes in.
                    variables.set(var_id, 0);
                    running.key_var = var_id;
                    running.key_accept = accept;
                    running.key_pending = true;
                    return;
                }
                // No-wait: sample the accepted keys once (held counts) and store the
                // code (0 when none is down), then continue.
                let keys = &subsystems.flow.keys;
                let code = key_code(
                    &accept,
                    keys.pressed(KeyCode::ArrowUp),
                    keys.pressed(KeyCode::ArrowDown),
                    keys.pressed(KeyCode::ArrowLeft),
                    keys.pressed(KeyCode::ArrowRight),
                    keys.pressed(KeyCode::Enter) || keys.pressed(KeyCode::Space),
                    keys.pressed(KeyCode::Escape),
                    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
                );
                variables.set(var_id, code);
                running.ip += 1;
            }
            CHANGE_SAVE_ACCESS => {
                subsystems.access.save_access.0 = command.params.first().copied().unwrap_or(0) != 0;
                running.ip += 1;
            }
            CHANGE_MENU_ACCESS => {
                subsystems.access.menu_access.0 = command.params.first().copied().unwrap_or(0) != 0;
                running.ip += 1;
            }
            CALL_EVENT => {
                // Run the called page as a sub-frame; the caller resumes at ip+1
                // when the callee ends. Depth-guarded against a self-calling cycle.
                match subsystems
                    .flow
                    .map_events
                    .as_ref()
                    .and_then(|ev| call_event_page(&ev.events, &command.params, running.event_id))
                {
                    Some((commands, target)) if running.call_stack.len() < MAX_CALL_DEPTH => {
                        let frame = CallFrame {
                            commands: std::mem::take(&mut running.commands),
                            ip: running.ip + 1,
                            event_id: running.event_id,
                        };
                        running.call_stack.push(frame);
                        running.commands = commands;
                        running.ip = 0;
                        running.event_id = target;
                    }
                    _ => running.ip += 1,
                }
            }
            RETURN_TO_TITLE => {
                // Hand the screen back to the title, mirroring the menu's End Game
                // path; the run ends here.
                subsystems.flow.title.0 = true;
                running.stop();
                return;
            }
            CHANGE_SKILLS
            | CHANGE_EQUIPMENT
            | CHANGE_CONDITION
            | CHANGE_SYSTEM_BGM
            | CHANGE_SCREEN_TRANSITIONS
            | FLASH_SPRITE
            | CHANGE_PBG
            | ENTER_EXIT_VEHICLE
            | SET_VEHICLE_LOCATION
            | COMMENT
            | COMMENT_2
            | END_MARKER => {
                // Faithfully decoded but deliberately inert in this remake (each
                // rationale is on its constant in `opcodes`): the skill/equipment/
                // condition commands have no per-actor state to mutate; the system
                // BGM slots have no audio consumer; the transition/panorama/flash
                // commands have no renderer; vehicles do not exist; Comment/END are
                // structural markers.
                running.ip += 1;
            }
            _ => {
                // Any remaining unmapped command (including the empty `code: 0`)
                // simply advances.
                running.ip += 1;
            }
        }
    }
}

/// Start the map's autorun (trigger 3) event when nothing else is running.
/// RM2000 replays an autorun page every frame its condition holds; a cutscene
/// ends by flipping a switch so a non-autorun page becomes active and it stops.
#[allow(clippy::too_many_arguments)]
fn autorun(
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    dialogue: Res<Dialogue>,
    fade: Res<Fade>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    battle: Res<BattleActive>,
    title: Res<TitleActive>,
    gameover: Res<GameOverActive>,
    intro_guard: IntroGuard,
    mut running: ResMut<RunningEvent>,
) {
    if running.active()
        || dialogue.active
        || fade.busy()
        || menu.0
        || shop.0
        || battle.0
        || title.0
        || gameover.0
    {
        return;
    }
    // After a Continue, never let the start map's unconditional New Game intro
    // replay: a resume that resolved onto the start map (a stale slot, or a race)
    // would otherwise re-run the opening and teleport the player to the beginning.
    if intro_guard.suppresses_start_intro() {
        return;
    }
    for event in &map_events.events {
        if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
            && page.trigger == 3
        {
            running.start(event.id, page.commands.clone());
            return;
        }
    }
}

fn is_message(code: u32) -> bool {
    matches!(code, SHOW_MESSAGE | SHOW_MESSAGE_2 | CHANGE_FACE)
}

/// Resolve an RM2000 character reference — 10001 the hero, 10005 this event, any
/// other positive value an event id — to its `(tile_x, tile_y, facing)`, read from
/// the live hero and event sprites. Shared by the `ControlVariables` character
/// operand and the `ConditionalBranch` orientation check; an unknown reference or
/// a missing sprite yields `None`.
fn resolve_character(
    char_ref: i32,
    this_event: u32,
    players: &Query<&Player>,
    events: &Query<(&EventSprite, &mut MoveQueue), Without<Player>>,
) -> Option<(i32, i32, u32)> {
    if char_ref == 10001 {
        players.single().ok().map(|p| (p.tile_x, p.tile_y, p.dir))
    } else {
        let id = if char_ref == 10005 {
            this_event as i32
        } else {
            char_ref
        };
        events
            .iter()
            .find(|(e, _)| e.id as i32 == id)
            .map(|(e, _)| (e.tile_x, e.tile_y, e.dir))
    }
}

/// Release the `MoveEvent` pause once every moved character's queue has drained,
/// letting [`run_interpreter`] advance past the move on the next frame.
fn clear_move_wait(mut running: ResMut<RunningEvent>, movers: Query<&MoveQueue>) {
    if running.wait_move && movers.iter().all(|queue| !queue.busy()) {
        running.wait_move = false;
    }
}

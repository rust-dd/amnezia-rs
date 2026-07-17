//! The event interpreter: runs an active event page's RM2000 command list one
//! step at a time. It drives the existing message box for dialogue and the
//! existing teleport fade for map transfers, and reads/writes the game's
//! switches and variables. A page's commands are a flat list with a per-command
//! `indent`; conditional branches use that indent to delimit their bodies.

use crate::appearance::SpriteChange;
use crate::audio::AudioRequest;
use crate::battle::{BattleActive, BattleOutcome, BattleRequest, BattleResult};
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::events::message_boxes;
use crate::gameover::GameOverActive;
use crate::inputnumber::InputNumber;
use crate::menu::MenuOpen;
use crate::picture::PictureCommand;
use crate::player::Player;
use crate::screenfx::ScreenEffect;
use crate::shop::{ShopOpen, ShopRequest};
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::teleport::{Fade, PendingTeleport};
use crate::text::{self, HeroName};
use crate::title::TitleActive;
use crate::vitals::Vitals;
use crate::world::{EventSprite, MapEvents, MoveQueue, RelocateEvent, decode_route};
use amnezia_data::EventCommand;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::HashMap;

mod commands;
mod flow;
mod opcodes;
mod present;

use commands::*;
use flow::*;
use opcodes::*;
use present::{Present, parse_present};

/// The overlays that pause the running event (title, menu, shop, battle). Bundled
/// into one `SystemParam` so `run_interpreter` stays within Bevy's 16-parameter cap.
#[derive(SystemParam)]
pub struct Blockers<'w> {
    menu: Res<'w, MenuOpen>,
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
    title: Res<'w, TitleActive>,
}

impl Blockers<'_> {
    fn any(&self) -> bool {
        self.menu.0 || self.shop.0 || self.battle.0 || self.title.0
    }
}

/// The interpreter's channel to the shop and battle subsystems: the writers that
/// open each screen and the finished-battle result it consumes to pick a handler
/// branch. Bundled into one `SystemParam` so `run_interpreter` stays within
/// Bevy's 16-parameter cap.
#[derive(SystemParam)]
pub struct SubsystemIo<'w> {
    battle_result: ResMut<'w, BattleResult>,
    shop_writer: MessageWriter<'w, ShopRequest>,
    battle_writer: MessageWriter<'w, BattleRequest>,
    screen_writer: MessageWriter<'w, ScreenEffect>,
    picture_writer: MessageWriter<'w, PictureCommand>,
    gameover: ResMut<'w, GameOverActive>,
    vitals: ResMut<'w, Vitals>,
    input_number: ResMut<'w, InputNumber>,
    sprite_writer: MessageWriter<'w, SpriteChange>,
    relocate_writer: MessageWriter<'w, RelocateEvent>,
}

/// A frame-local cap on executed commands, so a malformed list (e.g. a branch
/// that never advances) can't lock up the frame. Well-formed pages never
/// approach it — every command strictly advances the instruction pointer.
const MAX_STEPS_PER_FRAME: usize = 10_000;

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
        self.battle_pending = false;
        self.battle_outcome = None;
        self.shop_pending = false;
        self.input_pending = false;
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
        self.battle_pending = false;
        self.battle_outcome = None;
        self.shop_pending = false;
        self.input_pending = false;
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
}

pub struct InterpreterPlugin;

impl Plugin for InterpreterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunningEvent>()
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
    hero: Res<HeroName>,
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
        if let Some((code, indent)) = running.commands.get(running.ip).map(|c| (c.code, c.indent)) {
            let terminator = if code == OPEN_SHOP { END_SHOP } else { END_INN };
            running.ip = skip_to_terminator(&running.commands, running.ip, indent, terminator);
        }
        running.shop_pending = false;
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
                    for message in &mut boxes {
                        for line in &mut message.lines {
                            *line =
                                text::substitute(&crate::i18n::tr(line), &hero.0, &variables);
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
                apply_control_variables(&mut variables, &command.params);
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
                apply_change_party(&mut party, &command.params);
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
                subsystems.sprite_writer.write(SpriteChange {
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
                if branch_holds(&command.params, &switches, &variables, &party, &inventory) {
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
                            .map(|l| text::substitute(&crate::i18n::tr(l), &hero.0, &variables))
                            .collect();
                    if labels.is_empty() {
                        running.ip = skip_to_terminator(
                            &running.commands,
                            running.ip,
                            command.indent,
                            SHOW_CHOICE_END,
                        );
                    } else {
                        choice.open(labels, command.indent);
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
                // Offer `params[4..]` (item ids, negatives dropped); pause until the
                // screen closes, then the shop-resume above skips the block.
                let items = command
                    .params
                    .iter()
                    .skip(4)
                    .filter(|&&p| p >= 0)
                    .map(|&p| p as u32)
                    .collect();
                subsystems
                    .shop_writer
                    .write(ShopRequest::OpenShop { items });
                running.shop_pending = true;
                return;
            }
            SHOW_INN => {
                // Charge `params[1]` gold to rest; pause as for the shop.
                let cost = command.params.get(1).copied().unwrap_or(0);
                subsystems.shop_writer.write(ShopRequest::ShowInn { cost });
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
                audio.write(AudioRequest::StopBgm);
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
            _ => {
                // Every not-yet-supported command (weather, pan, …) simply advances.
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

/// Release the `MoveEvent` pause once every moved character's queue has drained,
/// letting [`run_interpreter`] advance past the move on the next frame.
fn clear_move_wait(mut running: ResMut<RunningEvent>, movers: Query<&MoveQueue>) {
    if running.wait_move && movers.iter().all(|queue| !queue.busy()) {
        running.wait_move = false;
    }
}

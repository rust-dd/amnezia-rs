//! The event interpreter: runs an active event page's RM2000 command list one
//! step at a time. It drives the existing message box for dialogue and the
//! existing teleport fade for map transfers, and reads/writes the game's
//! switches and variables. A page's commands are a flat list with a per-command
//! `indent`; conditional branches use that indent to delimit their bodies.

use crate::audio::AudioRequest;
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::events::message_boxes;
use crate::player::Player;
use crate::state::{active_page, Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport};
use crate::text::{self, HeroName};
use crate::world::{EventSprite, MapEvents};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::collections::HashMap;

// RM2000 opcodes, verified empirically against the converted map assets.
const SHOW_MESSAGE: u32 = 10110;
const SHOW_MESSAGE_2: u32 = 20110;
const CHANGE_FACE: u32 = 10130;
const CONTROL_SWITCHES: u32 = 10210;
const CONTROL_VARIABLES: u32 = 10220;
const TELEPORT: u32 = 10810;
const WAIT: u32 = 11410;
const CONDITIONAL_BRANCH: u32 = 12010;
const ELSE_BRANCH: u32 = 22010;
const END_BRANCH: u32 = 22011;
const ENEMY_ENCOUNTER: u32 = 10710;
const OPEN_SHOP: u32 = 10720;
const SHOW_INN: u32 = 10730;
const CHANGE_GOLD: u32 = 10310;
const CHANGE_ITEMS: u32 = 10320;
const CHANGE_PARTY: u32 = 10330;

/// Subsystem block openers not yet implemented, paired with their terminator
/// sub-code. The whole block is skipped so its branch-selector sub-codes
/// (VictoryHandler, Transaction, …) don't all execute in sequence.
const SKIP_BLOCKS: &[(u32, u32)] =
    &[(ENEMY_ENCOUNTER, 20713), (OPEN_SHOP, 20722), (SHOW_INN, 20732)];
const LABEL: u32 = 12110;
const JUMP_TO_LABEL: u32 = 12120;
const LOOP: u32 = 12210;
const END_LOOP: u32 = 22210;
const BREAK_LOOP: u32 = 12220;
const SHOW_CHOICE: u32 = 10140;
const SHOW_CHOICE_OPTION: u32 = 20140;
const SHOW_CHOICE_END: u32 = 20141;
const MOVE_EVENT: u32 = 11330;
const PLAY_BGM: u32 = 11510;
const FADE_OUT_BGM: u32 = 11520;
const PLAY_SOUND: u32 = 11550;

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
    event_id: u32,
    choices: HashMap<u32, i32>,
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
        self.event_id = event_id;
        self.choices.clear();
        self.active = true;
    }

    fn stop(&mut self) {
        self.active = false;
        self.commands.clear();
        self.ip = 0;
        self.wait = 0.0;
        self.event_id = 0;
        self.choices.clear();
    }
}

pub struct InterpreterPlugin;

impl Plugin for InterpreterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunningEvent>()
            .add_systems(Update, (autorun, run_interpreter).chain());
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
    mut running: ResMut<RunningEvent>,
    mut dialogue: ResMut<Dialogue>,
    mut choice: ResMut<Choice>,
    mut switches: ResMut<Switches>,
    mut variables: ResMut<Variables>,
    mut inventory: ResMut<Inventory>,
    mut party: ResMut<Party>,
    mut pending: ResMut<PendingTeleport>,
    mut players: Query<&mut Player>,
    mut event_sprites: Query<&mut EventSprite>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !running.active {
        return;
    }
    if dialogue.active || fade.busy() || choice.active() {
        return;
    }
    // Resume after the player confirmed a choice: record the pick so the
    // ShowChoice re-executes past the menu and the options self-select.
    if let Some(result) = choice.result.take() {
        running.choices.insert(choice.indent, result);
    }
    if running.wait > 0.0 {
        running.wait -= time.delta_secs();
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
                            *line = text::substitute(line, &hero.0, &variables);
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
            WAIT => {
                running.wait = command.params.first().copied().unwrap_or(0) as f32 / 10.0;
                running.ip += 1;
                return;
            }
            TELEPORT => {
                if let [map, x, y, ..] = command.params.as_slice() {
                    pending.0 = Some((*map as u32, *x as u32, *y as u32));
                }
                running.stop();
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
            BREAK_LOOP => running.ip = after_loop_end(&running.commands, running.ip, command.indent),
            SHOW_CHOICE => {
                if running.choices.contains_key(&command.indent) {
                    running.ip += 1;
                } else {
                    let labels: Vec<String> =
                        choice_labels(&running.commands, running.ip, command.indent)
                            .iter()
                            .map(|l| text::substitute(l, &hero.0, &variables))
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
                let target = command.params.first().copied().unwrap_or(0);
                let id = if target == 10005 { running.event_id as i32 } else { target };
                for action in decode_route(&command.params) {
                    match action {
                        RouteAction::Face(dir) => {
                            if target == 10001 {
                                if let Ok(mut player) = players.single_mut() {
                                    player.dir = dir;
                                }
                            } else if let Some(mut sprite) =
                                event_sprites.iter_mut().find(|s| s.id as i32 == id)
                            {
                                sprite.dir = dir;
                            }
                        }
                        RouteAction::ChangeGraphic(name, index) => {
                            if let Some(mut sprite) =
                                event_sprites.iter_mut().find(|s| s.id as i32 == id)
                            {
                                sprite.charset = name;
                                sprite.index = index;
                            }
                        }
                    }
                }
                running.ip += 1;
            }
            ENEMY_ENCOUNTER | OPEN_SHOP | SHOW_INN => {
                let terminator = SKIP_BLOCKS
                    .iter()
                    .find(|(opener, _)| *opener == command.code)
                    .map(|(_, terminator)| *terminator)
                    .unwrap_or(0);
                running.ip =
                    skip_to_terminator(&running.commands, running.ip, command.indent, terminator);
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
            _ => {
                // Every not-yet-supported command (movement, screen effects)
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
    mut running: ResMut<RunningEvent>,
) {
    if running.active() || dialogue.active || fade.busy() {
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

/// Apply a `ControlSwitches` command `[mode, start_id, end_id, op]` to the id
/// range `start..=end`: op 0 turns switches ON, 1 OFF, 2 toggles. `mode` (direct
/// range vs. variable-referenced id) is treated as a direct range for now.
fn apply_control_switches(switches: &mut Switches, params: &[i32]) {
    let [_, start, end, op, ..] = params else {
        return;
    };
    for id in *start..=*end {
        let id = id as u32;
        match op {
            0 => switches.set(id, true),
            1 => switches.set(id, false),
            2 => switches.set(id, !switches.get(id)),
            _ => {}
        }
    }
}

/// Apply a `ControlVariables` command `[mode, start_id, end_id, op, operand_type,
/// a, b]` to the id range `start..=end`. `op` 0 set, 1 add, 2 sub, 3 mul, 4 div,
/// 5 mod (div/mod by zero leave the value unchanged). The operand is the
/// constant `a` (operand_type 0) or the value of variable `a` (operand_type 1);
/// other operand types are treated as the constant `a` for now.
fn apply_control_variables(variables: &mut Variables, params: &[i32]) {
    let [_, start, end, op, operand_type, a, ..] = params else {
        return;
    };
    let operand = if *operand_type == 1 { variables.get(*a as u32) } else { *a };
    for id in *start..=*end {
        let id = id as u32;
        let current = variables.get(id);
        let next = match op {
            0 => operand,
            1 => current + operand,
            2 => current - operand,
            3 => current * operand,
            4 => if operand != 0 { current / operand } else { current },
            5 => if operand != 0 { current % operand } else { current },
            _ => current,
        };
        variables.set(id, next);
    }
}

/// Apply a `ChangeGold` command `[op, operand_type, amount]`: op 0 adds gold,
/// 1 removes it (constant operand, first-pass).
fn apply_change_gold(inventory: &mut Inventory, params: &[i32]) {
    let [op, _, amount, ..] = params else {
        return;
    };
    match op {
        0 => inventory.add_gold(*amount),
        1 => inventory.remove_gold(*amount),
        _ => {}
    }
}

/// Apply a `ChangeItems` command `[op, operand_type, item_id, count_operand,
/// count]`: op 0 adds `count` of `item_id`, 1 removes.
fn apply_change_items(inventory: &mut Inventory, params: &[i32]) {
    let [op, _, item_id, _, count, ..] = params else {
        return;
    };
    let (item_id, count) = (*item_id as u32, (*count).max(0) as u32);
    match op {
        0 => inventory.add_item(item_id, count),
        1 => inventory.remove_item(item_id, count),
        _ => {}
    }
}

/// Apply a `ChangePartyMembers` command `[op, operand_type, actor_id]`: op 0
/// adds the actor to the party, 1 removes.
fn apply_change_party(party: &mut Party, params: &[i32]) {
    let [op, _, actor_id, ..] = params else {
        return;
    };
    let actor_id = *actor_id as u32;
    match op {
        0 => party.add(actor_id),
        1 => party.remove(actor_id),
        _ => {}
    }
}

/// Whether a `ConditionalBranch`'s condition holds. Switch (0), variable (1),
/// money (3), item (4), and hero-in-party (5) are evaluated; unsupported kinds
/// (timer, …) return `true` so their body runs rather than the event stalling.
/// Money/item/hero use first-pass semantics (see the arms); the hero sub-check
/// (level/equipment in `params[2..]`) is treated as just "actor in party".
fn branch_holds(
    params: &[i32],
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
) -> bool {
    match params.first().copied().unwrap_or(-1) {
        0 => {
            let id = params.get(1).copied().unwrap_or(0) as u32;
            let want_on = params.get(2).copied().unwrap_or(0) == 0;
            switches.get(id) == want_on
        }
        1 => {
            let lhs = variables.get(params.get(1).copied().unwrap_or(0) as u32);
            let operand = params.get(3).copied().unwrap_or(0);
            let rhs = if params.get(2).copied().unwrap_or(0) == 1 {
                variables.get(operand as u32)
            } else {
                operand
            };
            match params.get(4).copied().unwrap_or(0) {
                0 => lhs == rhs,
                1 => lhs >= rhs,
                2 => lhs <= rhs,
                3 => lhs > rhs,
                4 => lhs < rhs,
                5 => lhs != rhs,
                _ => true,
            }
        }
        3 => {
            let amount = params.get(1).copied().unwrap_or(0);
            if params.get(2).copied().unwrap_or(0) == 0 {
                inventory.gold() >= amount
            } else {
                inventory.gold() <= amount
            }
        }
        4 => inventory.has(params.get(1).copied().unwrap_or(0) as u32),
        5 => party.has(params.get(1).copied().unwrap_or(0) as u32),
        _ => true,
    }
}

/// The instruction pointer to jump to when a branch at `indent` is NOT taken:
/// skip the true body (every command deeper than `indent`), then enter the else
/// body if an `ELSE_BRANCH` marker follows, else land on the block terminator
/// (which the main loop skips).
fn skip_true_body(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && commands[j].indent > indent {
        j += 1;
    }
    if j < commands.len() && commands[j].code == ELSE_BRANCH && commands[j].indent == indent {
        j + 1
    } else {
        j
    }
}

/// The instruction pointer to jump to when the true body falls through to an
/// `ELSE_BRANCH` at `indent`: skip the else body, landing on the block
/// terminator (which the main loop skips).
fn skip_else_body(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && commands[j].indent > indent {
        j += 1;
    }
    j
}

/// Index of the `Label` (12110) whose first param equals `id`, anywhere in the
/// page (RM2000 jumps forward or back).
fn find_label(commands: &[EventCommand], id: i32) -> Option<usize> {
    commands.iter().position(|c| c.code == LABEL && c.params.first().copied() == Some(id))
}

/// The option labels of a `ShowChoice` at `ip`/`indent`: each following
/// `ShowChoiceOption` (20140) `.string` at `indent`, until `ShowChoiceEnd`.
fn choice_labels(commands: &[EventCommand], ip: usize, indent: u32) -> Vec<String> {
    let mut labels = Vec::new();
    for c in &commands[(ip + 1).min(commands.len())..] {
        if c.indent == indent && c.code == SHOW_CHOICE_END {
            break;
        }
        if c.indent == indent && c.code == SHOW_CHOICE_OPTION {
            labels.push(c.string.clone());
        }
    }
    labels
}

/// Index of the next `ShowChoiceOption` or `ShowChoiceEnd` at `indent` — where
/// execution resumes after skipping a non-selected option's body.
fn skip_option_body(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len()
        && !(commands[j].indent == indent
            && matches!(commands[j].code, SHOW_CHOICE_OPTION | SHOW_CHOICE_END))
    {
        j += 1;
    }
    j
}

/// A decoded MoveEvent route action the interpreter applies instantly. Movement
/// and other sub-commands are consumed for alignment but not represented.
enum RouteAction {
    Face(u32),
    ChangeGraphic(String, u32),
}

/// Decode a MoveEvent's route (the ints after `[ref, freq, repeat, skip]`) into
/// the `Face`/`ChangeGraphic` actions we apply; other sub-commands are skipped
/// with their arguments so the int stream stays aligned.
fn decode_route(params: &[i32]) -> Vec<RouteAction> {
    let mut actions = Vec::new();
    let mut i = 4;
    while i < params.len() {
        let sub = params[i];
        i += 1;
        match sub {
            12..=15 => actions.push(RouteAction::Face((sub - 12) as u32)),
            32 | 33 => i += 1,
            34 => {
                let len = params.get(i).copied().unwrap_or(0).max(0) as usize;
                i += 1;
                let end = (i + len).min(params.len());
                let name: String = params[i..end].iter().map(|&b| b as u8 as char).collect();
                i = end;
                let frame = params.get(i).copied().unwrap_or(0).max(0) as u32;
                i += 1;
                actions.push(RouteAction::ChangeGraphic(name, frame));
            }
            35 => {
                let len = params.get(i).copied().unwrap_or(0).max(0) as usize;
                i += 1 + len + 3;
            }
            _ => {}
        }
    }
    actions
}

/// Index of the `Loop` (12210) at `indent` that an `EndLoop` at `ip` closes
/// (scanning backward); falls back to `ip` if unmatched (a one-shot loop).
fn loop_start(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    (0..ip)
        .rev()
        .find(|&j| commands[j].code == LOOP && commands[j].indent == indent)
        .unwrap_or(ip)
}

/// Index just past the `EndLoop` enclosing a `BreakLoop` at `ip` (indent
/// `break_indent`): the next `EndLoop` at a shallower indent.
fn after_loop_end(commands: &[EventCommand], ip: usize, break_indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && !(commands[j].code == END_LOOP && commands[j].indent < break_indent)
    {
        j += 1;
    }
    (j + 1).min(commands.len())
}

/// Index just past a subsystem block's terminator: the first command at
/// `indent` whose `code == terminator`, plus one (or the end of the list).
fn skip_to_terminator(commands: &[EventCommand], ip: usize, indent: u32, terminator: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && !(commands[j].code == terminator && commands[j].indent == indent) {
        j += 1;
    }
    (j + 1).min(commands.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(code: u32, indent: u32) -> EventCommand {
        EventCommand { code, indent, string: String::new(), params: Vec::new() }
    }

    #[test]
    fn skips_whole_subsystem_block() {
        // 0: EnemyEncounter@0  1: VictoryHandler@0  2: body@1  3: EndBattle@0  4: after@0
        let commands =
            vec![cmd(10710, 0), cmd(20710, 0), cmd(10210, 1), cmd(20713, 0), cmd(10110, 0)];
        assert_eq!(skip_to_terminator(&commands, 0, 0, 20713), 4);
    }

    fn cmd_params(code: u32, indent: u32, params: Vec<i32>) -> EventCommand {
        EventCommand { code, indent, string: String::new(), params }
    }

    #[test]
    fn jump_finds_label_forward_and_back() {
        let commands = vec![
            cmd_params(12110, 0, vec![1]),
            cmd(10110, 0),
            cmd_params(12110, 0, vec![2]),
        ];
        assert_eq!(find_label(&commands, 2), Some(2));
        assert_eq!(find_label(&commands, 1), Some(0));
        assert_eq!(find_label(&commands, 9), None);
    }

    #[test]
    fn end_loop_jumps_back_to_loop() {
        // 0: Loop@0  1: Wait@1  2: EndLoop@0
        let commands = vec![cmd(12210, 0), cmd(11410, 1), cmd(22210, 0)];
        assert_eq!(loop_start(&commands, 2, 0), 0);
    }

    #[test]
    fn break_loop_skips_past_end() {
        // 0: Loop@0  1: Break@1  2: body@1  3: EndLoop@0  4: after@0
        let commands =
            vec![cmd(12210, 0), cmd(12220, 1), cmd(10110, 1), cmd(22210, 0), cmd(10110, 0)];
        assert_eq!(after_loop_end(&commands, 1, 1), 4);
    }

    #[test]
    fn choice_labels_collects_option_strings() {
        // ShowChoice@0; Option0 "Yes"@0 body@1; Option1 "No"@0 body@1; End@0
        let commands = vec![
            cmd_params(10140, 0, vec![0]),
            EventCommand { code: 20140, indent: 0, string: "Yes".into(), params: vec![0] },
            cmd(10210, 1),
            EventCommand { code: 20140, indent: 0, string: "No".into(), params: vec![1] },
            cmd(10210, 1),
            cmd(20141, 0),
        ];
        assert_eq!(choice_labels(&commands, 0, 0), vec!["Yes", "No"]);
    }

    #[test]
    fn skip_option_body_lands_on_next_option() {
        // 0: ShowChoice@0  1: Option0@0  2: body@1  3: Option1@0  4: End@0
        let commands =
            vec![cmd(10140, 0), cmd(20140, 0), cmd(10210, 1), cmd(20140, 0), cmd(20141, 0)];
        assert_eq!(skip_option_body(&commands, 1, 0), 3);
    }

    #[test]
    fn decodes_change_graphic_and_face() {
        // ref, freq, repeat, skip, then: change_graphic "Torch" frame 1, face-left (15)
        let params = vec![10005, 8, 0, 0, 34, 5, 84, 111, 114, 99, 104, 1, 15];
        let actions = decode_route(&params);
        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], RouteAction::ChangeGraphic(n, 1) if n == "Torch"));
        assert!(matches!(actions[1], RouteAction::Face(3)));
    }

    #[test]
    fn change_items_adds_and_removes() {
        let mut inv = Inventory::default();
        apply_change_items(&mut inv, &[0, 0, 181, 0, 2]);
        assert_eq!(inv.count(181), 2);
        apply_change_items(&mut inv, &[1, 0, 181, 0, 1]);
        assert_eq!(inv.count(181), 1);
    }

    #[test]
    fn change_gold_adds_and_removes() {
        let mut inv = Inventory::default();
        apply_change_gold(&mut inv, &[0, 0, 50]);
        apply_change_gold(&mut inv, &[1, 0, 30]);
        assert_eq!(inv.gold(), 20);
    }

    #[test]
    fn change_party_adds_and_removes() {
        let mut party = Party::default();
        apply_change_party(&mut party, &[0, 0, 2]);
        assert!(party.has(2));
        apply_change_party(&mut party, &[1, 0, 2]);
        assert!(!party.has(2));
    }

    #[test]
    fn control_switches_on_off_toggle() {
        let mut sw = Switches::default();
        apply_control_switches(&mut sw, &[0, 3, 3, 0]);
        assert!(sw.get(3));
        apply_control_switches(&mut sw, &[0, 3, 3, 1]);
        assert!(!sw.get(3));
        apply_control_switches(&mut sw, &[0, 3, 3, 2]);
        assert!(sw.get(3));
    }

    #[test]
    fn control_switches_range() {
        let mut sw = Switches::default();
        apply_control_switches(&mut sw, &[0, 5, 7, 0]);
        assert!(sw.get(5) && sw.get(6) && sw.get(7));
    }

    #[test]
    fn control_variables_set_add_and_from_variable() {
        let mut var = Variables::default();
        apply_control_variables(&mut var, &[0, 1, 1, 0, 0, 10, 0]);
        assert_eq!(var.get(1), 10);
        apply_control_variables(&mut var, &[0, 1, 1, 1, 0, 5, 0]);
        assert_eq!(var.get(1), 15);
        // var 2 = value of var 1 (operand_type 1)
        apply_control_variables(&mut var, &[0, 2, 2, 0, 1, 1, 0]);
        assert_eq!(var.get(2), 15);
    }

    #[test]
    fn branch_switch_on_and_off() {
        let mut sw = Switches::default();
        let (var, party, inv) = (Variables::default(), Party::default(), Inventory::default());
        // [type 0, switch 4, state 0 => branch if ON]
        assert!(!branch_holds(&[0, 4, 0, 0, 0, 0], &sw, &var, &party, &inv));
        sw.set(4, true);
        assert!(branch_holds(&[0, 4, 0, 0, 0, 0], &sw, &var, &party, &inv));
        // state 1 => branch if OFF
        assert!(!branch_holds(&[0, 4, 1, 0, 0, 0], &sw, &var, &party, &inv));
    }

    #[test]
    fn branch_variable_comparisons() {
        let sw = Switches::default();
        let (party, inv) = (Party::default(), Inventory::default());
        let mut var = Variables::default();
        var.set(1, 6);
        assert!(branch_holds(&[1, 1, 0, 6, 0, 0], &sw, &var, &party, &inv)); // == 6
        assert!(!branch_holds(&[1, 1, 0, 10, 1, 0], &sw, &var, &party, &inv)); // >= 10 false
        assert!(branch_holds(&[1, 1, 0, 10, 4, 0], &sw, &var, &party, &inv)); // < 10 true
    }

    #[test]
    fn branch_money_item_hero() {
        let (sw, var) = (Switches::default(), Variables::default());
        let mut party = Party::default();
        let mut inv = Inventory::default();
        // money: gold >= 100 (false, then true)
        assert!(!branch_holds(&[3, 100, 0, 0, 0, 0], &sw, &var, &party, &inv));
        inv.add_gold(120);
        assert!(branch_holds(&[3, 100, 0, 0, 0, 0], &sw, &var, &party, &inv));
        // item: has item 129
        assert!(!branch_holds(&[4, 129, 0, 0, 0, 0], &sw, &var, &party, &inv));
        inv.add_item(129, 1);
        assert!(branch_holds(&[4, 129, 0, 0, 0, 0], &sw, &var, &party, &inv));
        // hero: actor 2 in party
        assert!(!branch_holds(&[5, 2, 0, 0, 0, 0], &sw, &var, &party, &inv));
        party.add(2);
        assert!(branch_holds(&[5, 2, 0, 0, 0, 0], &sw, &var, &party, &inv));
    }

    #[test]
    fn skip_true_body_with_else() {
        // 0: branch@0  1: body@1  2: else@0  3: elsebody@1  4: end@0
        let commands = vec![
            cmd(CONDITIONAL_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(ELSE_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(END_BRANCH, 0),
        ];
        // Not taken → jump into the else body at index 3.
        assert_eq!(skip_true_body(&commands, 0, 0), 3);
    }

    #[test]
    fn skip_true_body_without_else() {
        // 0: branch@0  1: body@1  2: end@0
        let commands =
            vec![cmd(CONDITIONAL_BRANCH, 0), cmd(SHOW_MESSAGE, 1), cmd(END_BRANCH, 0)];
        // Not taken, no else → land on the end marker at index 2.
        assert_eq!(skip_true_body(&commands, 0, 0), 2);
    }

    #[test]
    fn skip_else_body_lands_on_end() {
        let commands = vec![
            cmd(CONDITIONAL_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(ELSE_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(END_BRANCH, 0),
        ];
        // From the else marker at index 2, skip the else body → end at index 4.
        assert_eq!(skip_else_body(&commands, 2, 0), 4);
    }
}

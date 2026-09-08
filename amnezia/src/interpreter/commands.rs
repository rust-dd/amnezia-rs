//! The pure command-application helpers: how a single RM2000 state command
//! (`ControlSwitches`, `ControlVariables`, the `Change*` family) mutates the
//! game state, and how a `ConditionalBranch` condition is evaluated. Kept free of
//! Bevy so they unit-test directly against the plain state resources.

use crate::animation::{AnimTarget, AnimationLibrary};
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party, Switches, Variables};

/// Apply a `ControlSwitches` command `[mode, start_id, end_id, op]` to the id
/// range `start..=end`: op 0 turns switches ON, 1 OFF, 2 toggles. `mode` (direct
/// range vs. variable-referenced id) is treated as a direct range for now.
pub(super) fn apply_control_switches(switches: &mut Switches, params: &[i32]) {
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

/// Apply a `ChangeGold` command using a literal amount or a variable's value.
pub(super) fn apply_change_gold(inventory: &mut Inventory, params: &[i32], variables: &Variables) {
    let [op, operand_type, amount, ..] = params else {
        return;
    };
    let amount = operate_value(0, *operand_type, *amount, variables);
    match op {
        0 => inventory.add_gold(amount),
        1 => inventory.remove_gold(amount),
        _ => {}
    }
}

/// Apply a `ChangeItems` command `[op, operand_type, item_id, count_operand,
/// count]`: op 0 adds `count` of `item_id`, 1 removes.
pub(super) fn apply_change_items(inventory: &mut Inventory, params: &[i32]) {
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
/// adds the actor to the party, 1 removes. Adding mirrors RM2000
/// `Game_Party::AddActor`: an id with no actor behind it is ignored, and the
/// party itself caps the roster at four (see [`Party::add`]).
pub(super) fn apply_change_party(party: &mut Party, data: &GameData, params: &[i32]) {
    let [op, _, actor_id, ..] = params else {
        return;
    };
    let actor_id = *actor_id as u32;
    match op {
        0 if data.actor(actor_id).is_some() => party.add(actor_id),
        1 => party.remove(actor_id),
        _ => {}
    }
}

/// Resolve an RM2000 character reference (as `ShowBattleAnimation` carries one)
/// to an [`AnimTarget`], mirroring the 10860/11330 decode: 10001 = hero, 10005 =
/// this event (resolved to `this_event`), any other positive value = that event
/// id. A non-positive reference names no character and yields `None`.
pub(super) fn resolve_anim_target(char_ref: i32, this_event: u32) -> Option<AnimTarget> {
    match char_ref {
        10001 => Some(AnimTarget::Hero),
        10005 => Some(AnimTarget::Event(this_event)),
        id if id > 0 => Some(AnimTarget::Event(id as u32)),
        _ => None,
    }
}

/// The data-frame count of animation `id` in the library, `0` when it is absent
/// (a bad id no-ops with no wait). Used to size a waiting `ShowBattleAnimation`.
pub(super) fn anim_frame_count(library: &AnimationLibrary, id: u32) -> usize {
    library
        .0
        .iter()
        .find(|a| a.id == id)
        .map_or(0, |a| a.frames.len())
}

/// The seconds a `ShowBattleAnimation` (11210) blocks its event when the wait flag
/// (`params[2]`) is set: the animation's `frames` data frames at the fixed 1/30 s
/// cadence ([`crate::animation::FRAME_SECS`]), mirroring
/// `Game_Interpreter_Map::CommandShowBattleAnimation` storing the animation's
/// frame count as `_state.wait_time`. `None` when the flag is clear (fire-and-forget).
pub(super) fn battle_anim_wait(params: &[i32], frames: usize) -> Option<f32> {
    (params.get(2).copied().unwrap_or(0) > 0)
        .then_some(frames as f32 * crate::animation::FRAME_SECS)
}

/// Resolve an RM2000 change-command value operand, mirroring EasyRPG's
/// `OperateValue`: `operand_type` 0 reads the constant `operand`, 1 reads the
/// value of variable `operand`; `operation` 1 (subtract) negates the result, any
/// other (0 = add) leaves it. Shared by the actor `Change*` commands.
pub(super) fn operate_value(
    operation: i32,
    operand_type: i32,
    operand: i32,
    variables: &Variables,
) -> i32 {
    let value = if operand_type == 1 {
        variables.get(operand as u32)
    } else {
        operand
    };
    if operation == 1 { -value } else { value }
}

/// The actor ids an actor-target `Change*` command addresses, mirroring EasyRPG's
/// `GetActors`: mode 0 = the whole party, 1 = the fixed actor `id_operand`, 2 =
/// the actor whose id is in variable `id_operand`. An unknown mode targets none.
pub(super) fn actor_targets(
    mode: i32,
    id_operand: i32,
    variables: &Variables,
    party: &Party,
) -> Vec<u32> {
    match mode {
        0 => party.snapshot(),
        1 => vec![id_operand.max(0) as u32],
        2 => vec![variables.get(id_operand as u32).max(0) as u32],
        _ => Vec::new(),
    }
}

/// Apply a `ChangeLevel` command `[mode, id, operation, operand_type, operand,
/// show_msg]` (EasyRPG code 10420): raise or lower each targeted actor's level by
/// the operate-value delta, via [`Progression::set_level`] so the new level drives
/// the actor's curve-derived battle stats. The `show_msg` flag (a level-up popup)
/// has no analogue here and is ignored.
pub(super) fn apply_change_level(
    progression: &mut Progression,
    data: &GameData,
    params: &[i32],
    variables: &Variables,
    party: &Party,
) {
    let [mode, id, operation, operand_type, operand, ..] = params else {
        return;
    };
    let delta = operate_value(*operation, *operand_type, *operand, variables);
    for actor_id in actor_targets(*mode, *id, variables, party) {
        if let Some(def) = data.actor(actor_id) {
            let target = (progression.level(def) as i32 + delta).max(1) as u32;
            progression.set_level(def, target);
        }
    }
}

/// Which keys a `KeyInputProc` (11610) accepts, decoded from its parameters.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(super) struct KeyAccept {
    pub decision: bool,
    pub cancel: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub shift: bool,
}

/// Decode which keys a `KeyInputProc` accepts, following EasyRPG's RPG2k layout:
/// `params[3]` = decision, `params[4]` = cancel always; a short list (< 6, the
/// pre-1.50 form seen in this game's data) enables all four arrows together via
/// `params[2]`, while a full list reads `params[5]` = shift and `params[6..10]` =
/// down/left/right/up individually.
pub(super) fn decode_key_accept(params: &[i32]) -> KeyAccept {
    let flag = |i: usize| params.get(i).copied().unwrap_or(0) != 0;
    let (decision, cancel) = (flag(3), flag(4));
    if params.len() < 6 {
        let dirs = flag(2);
        KeyAccept {
            decision,
            cancel,
            up: dirs,
            down: dirs,
            left: dirs,
            right: dirs,
            shift: false,
        }
    } else {
        KeyAccept {
            decision,
            cancel,
            shift: flag(5),
            down: flag(6),
            left: flag(7),
            right: flag(8),
            up: flag(9),
        }
    }
}

/// The RM2000 key code a `KeyInputProc` stores, given which accepted keys are
/// active this frame. Mirrors EasyRPG's `CheckInput` priority (shift 7, cancel 6,
/// decision 5, up 4, right 3, left 2, down 1); nothing pressed yields 0.
#[allow(clippy::too_many_arguments)]
pub(super) fn key_code(
    accept: &KeyAccept,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    decision: bool,
    cancel: bool,
    shift: bool,
) -> i32 {
    if accept.shift && shift {
        7
    } else if accept.cancel && cancel {
        6
    } else if accept.decision && decision {
        5
    } else if accept.up && up {
        4
    } else if accept.right && right {
        3
    } else if accept.left && left {
        2
    } else if accept.down && down {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anim_target_decodes_hero_this_event_and_ids() {
        assert_eq!(resolve_anim_target(10001, 42), Some(AnimTarget::Hero));
        assert_eq!(resolve_anim_target(10005, 42), Some(AnimTarget::Event(42)));
        assert_eq!(resolve_anim_target(7, 42), Some(AnimTarget::Event(7)));
        assert_eq!(resolve_anim_target(0, 42), None);
        assert_eq!(resolve_anim_target(-3, 42), None);
    }

    #[test]
    fn battle_anim_wait_blocks_only_when_the_wait_flag_is_set() {
        // `params = [anim_id, target, wait, global]`. A clear wait flag never
        // blocks; a set one blocks for the animation's frame count at 1/30 s.
        assert_eq!(battle_anim_wait(&[62, 10001, 0, 0], 12), None);
        assert_eq!(
            battle_anim_wait(&[62, 10001, 1, 0], 12),
            Some(12.0 * crate::animation::FRAME_SECS)
        );
        // 12 frames at 1/30 s is 0.4 s.
        assert!((battle_anim_wait(&[62, 10001, 1, 0], 12).unwrap() - 0.4).abs() < 1e-6);
        // A missing wait param (short list) is treated as no-wait.
        assert_eq!(battle_anim_wait(&[62, 10001], 12), None);
        // A set flag with a zero-frame (or absent) animation blocks for zero time.
        assert_eq!(battle_anim_wait(&[62, 10001, 1, 0], 0), Some(0.0));
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
        apply_change_gold(&mut inv, &[0, 0, 50], &Variables::default());
        apply_change_gold(&mut inv, &[1, 0, 30], &Variables::default());
        assert_eq!(inv.gold(), 20);
    }

    #[test]
    fn gambling_reward_uses_the_winnings_variable() {
        let mut inventory = Inventory::default();
        let mut variables = Variables::default();
        variables.set(84, 1250);
        apply_change_gold(&mut inventory, &[0, 1, 84], &variables);
        assert_eq!(inventory.gold(), 1250);
        apply_change_gold(&mut inventory, &[1, 1, 84], &variables);
        assert_eq!(inventory.gold(), 0);
    }

    #[test]
    fn change_party_adds_and_removes() {
        let mut actor2 = level_def();
        actor2.id = 2;
        let data = GameData {
            actors: vec![level_def(), actor2],
            items: vec![],
            skills: vec![],
        };
        let mut party = Party::default();
        apply_change_party(&mut party, &data, &[0, 0, 2]);
        assert!(party.has(2));
        apply_change_party(&mut party, &data, &[1, 0, 2]);
        assert!(!party.has(2));
        // An actor with no database entry is ignored (RM2000 AddActor).
        apply_change_party(&mut party, &data, &[0, 0, 99]);
        assert!(!party.has(99), "unknown actor id refused");
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
    fn operate_value_reads_constant_variable_and_negates_on_subtract() {
        let mut vars = Variables::default();
        vars.set(7, 40);
        // operand_type 0 = constant, 1 = variable; operation 1 = subtract.
        assert_eq!(operate_value(0, 0, 5, &vars), 5);
        assert_eq!(operate_value(0, 1, 7, &vars), 40);
        assert_eq!(operate_value(1, 0, 5, &vars), -5);
        assert_eq!(operate_value(1, 1, 7, &vars), -40);
    }

    #[test]
    fn actor_targets_resolves_party_fixed_and_variable() {
        let mut vars = Variables::default();
        vars.set(3, 9);
        let mut party = Party::default();
        party.add(2);
        assert_eq!(actor_targets(0, 0, &vars, &party), vec![1, 2]);
        assert_eq!(actor_targets(1, 5, &vars, &party), vec![5]);
        assert_eq!(actor_targets(2, 3, &vars, &party), vec![9]);
        assert!(actor_targets(7, 0, &vars, &party).is_empty());
    }

    fn level_def() -> amnezia_data::ActorDef {
        amnezia_data::ActorDef {
            id: 1,
            name: "Ron".into(),
            title: String::new(),
            level: 2,
            max_level: 50,
            hp: 30,
            sp: 10,
            curves: amnezia_data::ActorCurves::default(),
            learnings: Vec::new(),
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
            face_name: String::new(),
            face_index: 0,
        }
    }

    #[test]
    fn change_level_raises_the_targeted_actor() {
        let data = GameData {
            actors: vec![level_def()],
            items: vec![],
            skills: vec![],
        };
        let mut prog = Progression::default();
        let vars = Variables::default();
        let party = Party::default();
        assert_eq!(prog.level(&data.actors[0]), 2);
        // [mode 1 (actor 1), add, constant, +5, show_msg] -> level 2 + 5 = 7.
        apply_change_level(&mut prog, &data, &[1, 1, 0, 0, 5, 0], &vars, &party);
        assert_eq!(prog.level(&data.actors[0]), 7);
        apply_change_level(&mut prog, &data, &[1, 1, 1, 0, 99, 0], &vars, &party);
        assert_eq!(prog.level(&data.actors[0]), 1);
    }

    #[test]
    fn key_accept_decodes_short_and_full_forms() {
        // Short (< 6): params[2] enables all arrows, params[3]/[4] decision/cancel.
        let short = decode_key_accept(&[52, 1, 1, 1, 0]);
        assert_eq!(
            short,
            KeyAccept {
                decision: true,
                cancel: false,
                up: true,
                down: true,
                left: true,
                right: true,
                shift: false,
            }
        );
        // Full (>= 6): individual down/left/right/up plus shift.
        let full = decode_key_accept(&[52, 1, 0, 1, 0, 1, 1, 0, 0, 0]);
        assert!(full.decision && full.shift && full.down && !full.left && !full.up);
    }

    #[test]
    fn key_code_follows_easyrpg_priority() {
        let accept = KeyAccept {
            decision: true,
            cancel: true,
            up: true,
            down: true,
            left: true,
            right: true,
            shift: true,
        };
        // Cancel (6) outranks decision (5) which outranks the arrows.
        assert_eq!(
            key_code(&accept, false, false, false, false, true, true, false),
            6
        );
        assert_eq!(
            key_code(&accept, true, false, false, false, true, false, false),
            5
        );
        assert_eq!(
            key_code(&accept, true, false, false, true, false, false, false),
            4
        );
        assert_eq!(
            key_code(&accept, false, true, false, false, false, false, false),
            1
        );
        assert_eq!(
            key_code(&accept, false, false, false, false, false, false, false),
            0
        );
        // An unaccepted key stays silent even when pressed.
        let only_decision = KeyAccept {
            decision: true,
            ..Default::default()
        };
        assert_eq!(
            key_code(&only_decision, true, true, true, true, false, true, true),
            0
        );
    }
}

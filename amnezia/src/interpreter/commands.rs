//! The pure command-application helpers: how a single RM2000 state command
//! (`ControlSwitches`, `ControlVariables`, the `Change*` family) mutates the
//! game state, and how a `ConditionalBranch` condition is evaluated. Kept free of
//! Bevy so they unit-test directly against the plain state resources.

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

/// Apply a `ControlVariables` command `[mode, start_id, end_id, op, operand_type,
/// a, b]` to the id range `start..=end`. `op` 0 set, 1 add, 2 sub, 3 mul, 4 div,
/// 5 mod (div/mod by zero leave the value unchanged). The operand is the
/// constant `a` (operand_type 0) or the value of variable `a` (operand_type 1);
/// other operand types are treated as the constant `a` for now.
pub(super) fn apply_control_variables(variables: &mut Variables, params: &[i32]) {
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
pub(super) fn apply_change_gold(inventory: &mut Inventory, params: &[i32]) {
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
/// adds the actor to the party, 1 removes.
pub(super) fn apply_change_party(party: &mut Party, params: &[i32]) {
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
pub(super) fn branch_holds(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

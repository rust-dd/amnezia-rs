//! The `ControlVariables` (opcode 10220) operand resolver and its assignment.
//! An operand is first resolved to a value (or a random range) from the game
//! state, then applied to the target variable(s) under the command's target mode
//! and arithmetic operation. Mirrors EasyRPG's `CommandControlVariables` and the
//! `ControlVariables::` helpers, restricted to the RM2000 (non-Maniac) operand
//! and operation set. Kept Bevy-free so the resolution and the arithmetic
//! unit-test against plain state resources; the actor operand delegates to
//! [`super::actor_query`] and randomness to [`super::event_rng`].

use super::actor_query::{ActorCtx, actor_param};
use super::event_rng::EventRng;
use crate::gamedata::GameData;
use crate::state::{Inventory, Party, Variables};

/// A resolved `ControlVariables` operand: a single value (operand types 0, 1, 2,
/// 4, 5, 6, 7) or a random range that redraws for each variable in a target range
/// (type 3), mirroring RPG_RT's per-element `SetRangeRandom`.
#[derive(Clone, Copy)]
pub(super) enum Operand {
    Value(i32),
    Random { lo: i32, hi: i32 },
}

/// Resolve a `ControlVariables` operand from its command parameters, reading
/// `params[5]` as the first operand argument and `params[6]` as the second, per
/// EasyRPG's decode. `character` is the referenced character's `(x, y, facing)`
/// for operand type 6 (resolved by the caller, which holds the world queries).
pub(super) fn resolve_operand(
    params: &[i32],
    variables: &Variables,
    inventory: &Inventory,
    party: &Party,
    actors: &ActorCtx,
    timer_secs: u32,
    character: Option<(i32, i32, u32)>,
) -> Operand {
    let a = params.get(5).copied().unwrap_or(0);
    let b = params.get(6).copied().unwrap_or(0);
    match params.get(4).copied().unwrap_or(0) {
        0 => Operand::Value(a),
        1 => Operand::Value(variables.get(a.max(0) as u32)),
        2 => Operand::Value(variables.get(variables.get(a.max(0) as u32).max(0) as u32)),
        3 => Operand::Random {
            lo: a.min(b),
            hi: a.max(b),
        },
        4 => Operand::Value(item_count(
            b,
            a.max(0) as u32,
            inventory,
            party,
            actors.data,
        )),
        5 => Operand::Value(actor_param(b, a.max(0) as u32, actors)),
        6 => Operand::Value(character_param(b, character)),
        7 => Operand::Value(other_param(a, inventory, party, timer_secs)),
        _ => Operand::Value(0),
    }
}

/// Operand type 4 (Items): sub-op 0 counts the item owned in the inventory, sub-op
/// 1 counts how many party members have it equipped (EasyRPG `Item`).
fn item_count(
    sub_op: i32,
    item_id: u32,
    inventory: &Inventory,
    party: &Party,
    data: &GameData,
) -> i32 {
    match sub_op {
        1 => equipped_count(item_id, party, data),
        _ => inventory.count(item_id) as i32,
    }
}

fn equipped_count(item_id: u32, party: &Party, data: &GameData) -> i32 {
    party
        .snapshot()
        .into_iter()
        .filter_map(|id| data.actor(id))
        .map(|d| {
            [d.weapon, d.shield, d.armor, d.helmet, d.accessory]
                .into_iter()
                .filter(|&slot| slot == item_id)
                .count()
        })
        .sum::<usize>() as i32
}

/// Operand type 6 (Character): read the referenced character's tile X (sub-op 1),
/// tile Y (sub-op 2), or orientation code (sub-op 3). An unresolved character, or
/// a sub-op needing the live map scroll (0 map id, 4 screen X, 5 screen Y), reads
/// 0 — the scroll position is not plumbed into the interpreter.
fn character_param(sub_op: i32, character: Option<(i32, i32, u32)>) -> i32 {
    let Some((x, y, dir)) = character else {
        return 0;
    };
    match sub_op {
        1 => x,
        2 => y,
        3 => facing_code(dir),
        _ => 0,
    }
}

/// The RPG_RT orientation code a facing maps to (EasyRPG `ControlVariables::Event`
/// op 3): up→8, right→6, down→2, left→4.
fn facing_code(dir: u32) -> i32 {
    match dir {
        0 => 8,
        1 => 6,
        2 => 2,
        _ => 4,
    }
}

/// Operand type 7 (Other): gold (sub-op 0), the timer's remaining seconds (1), or
/// the party size (2). The remaining EasyRPG sub-ops — save count (3), battle
/// count (4), win/defeat/escape counts (5–7) — have no backing resource in this
/// remake and read 0.
fn other_param(sub_op: i32, inventory: &Inventory, party: &Party, timer_secs: u32) -> i32 {
    match sub_op {
        0 => inventory.gold(),
        1 => timer_secs as i32,
        2 => party.snapshot().len() as i32,
        _ => 0,
    }
}

/// Apply a resolved operand to the command's target variable(s). The target mode
/// `params[0]` selects a single variable (0), an inclusive range (1), or the
/// variable whose id is held in another variable (2); `params[3]` is the
/// operation (0 set, 1 add, 2 sub, 3 mul, 4 div, 5 mod). A random operand redraws
/// for each variable in a range. Every write clamps to the RM2000 range (in
/// [`Variables::set`]); division or modulo by zero yields 0.
pub(super) fn apply_control_variables(
    variables: &mut Variables,
    rng: &mut EventRng,
    params: &[i32],
    operand: Operand,
) {
    let (mut start, mut end) = target_range(params, variables);
    if end < start {
        std::mem::swap(&mut start, &mut end);
    }
    let op = params.get(3).copied().unwrap_or(0);
    for id in start..=end {
        if id < 1 {
            continue;
        }
        let value = match operand {
            Operand::Value(v) => v,
            Operand::Random { lo, hi } => rng.range(lo, hi),
        };
        let next = combine(variables.get(id as u32), value, op);
        variables.set(id as u32, next);
    }
}

/// Decode the target id range from the command's target mode (`params[0]`).
fn target_range(params: &[i32], variables: &Variables) -> (i32, i32) {
    let start = params.get(1).copied().unwrap_or(0);
    match params.first().copied().unwrap_or(0) {
        1 => (start, params.get(2).copied().unwrap_or(start)),
        2 => {
            let id = variables.get(start.max(0) as u32);
            (id, id)
        }
        _ => (start, start),
    }
}

/// Combine the current value with the operand under `op`, in `i64` so an
/// overshooting add/mul never overflows before the write clamps it. Division and
/// modulo by zero yield 0 (per the remake's rule; RPG_RT's own `VarDiv` instead
/// keeps the dividend, but here both zero-divisors resolve to 0).
fn combine(current: i32, operand: i32, op: i32) -> i32 {
    let (a, b) = (current as i64, operand as i64);
    let next = match op {
        0 => b,
        1 => a + b,
        2 => a - b,
        3 => a * b,
        4 => {
            if b != 0 {
                a / b
            } else {
                0
            }
        }
        5 => {
            if b != 0 {
                a % b
            } else {
                0
            }
        }
        _ => a,
    };
    next.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::super::actor_query::fixtures::game_data;
    use super::*;
    use crate::progression::Progression;
    use crate::vitals::Vitals;
    use std::collections::HashSet;

    fn value(operand: Operand) -> i32 {
        match operand {
            Operand::Value(v) => v,
            Operand::Random { .. } => panic!("expected a resolved value, got a random range"),
        }
    }

    fn ctx<'a>(data: &'a GameData, prog: &'a Progression, vit: &'a Vitals) -> ActorCtx<'a> {
        ActorCtx {
            data,
            progression: prog,
            vitals: vit,
            equipment: super::super::actor_query::fixtures::equipment(),
            hero_name: "Ron",
        }
    }

    #[test]
    fn set_add_sub_mul_and_divmod_by_zero() {
        let mut var = Variables::default();
        let mut rng = EventRng::seeded(1);
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 0], Operand::Value(10));
        assert_eq!(var.get(1), 10);
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 1], Operand::Value(5));
        assert_eq!(var.get(1), 15);
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 2], Operand::Value(3));
        assert_eq!(var.get(1), 12);
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 3], Operand::Value(4));
        assert_eq!(var.get(1), 48);
        // Division by zero resolves to 0, not the dividend.
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 4], Operand::Value(0));
        assert_eq!(var.get(1), 0);
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 0], Operand::Value(7));
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 5], Operand::Value(0));
        assert_eq!(var.get(1), 0, "modulo by zero resolves to 0");
    }

    #[test]
    fn writes_clamp_to_the_rm2000_range() {
        let mut var = Variables::default();
        let mut rng = EventRng::seeded(1);
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 0], Operand::Value(999_999));
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 3], Operand::Value(999_999));
        assert_eq!(
            var.get(1),
            999_999,
            "a mul overshoot saturates at the ceiling"
        );
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 0], Operand::Value(-999_999));
        apply_control_variables(&mut var, &mut rng, &[0, 1, 1, 3], Operand::Value(999_999));
        assert_eq!(
            var.get(1),
            -999_999,
            "a negative overshoot saturates at the floor"
        );
    }

    #[test]
    fn range_and_indirect_single_targets() {
        let mut var = Variables::default();
        let mut rng = EventRng::seeded(1);
        // Range mode: vars 2..=4 all set to 5.
        apply_control_variables(&mut var, &mut rng, &[1, 2, 4, 0], Operand::Value(5));
        assert_eq!((var.get(2), var.get(3), var.get(4)), (5, 5, 5));
        // Indirect single: var 10 holds 7, so the target is var 7.
        var.set(10, 7);
        apply_control_variables(&mut var, &mut rng, &[2, 10, 10, 0], Operand::Value(99));
        assert_eq!(var.get(7), 99);
    }

    #[test]
    fn constant_variable_and_var_of_var_operands() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let (party, inv) = (Party::default(), Inventory::default());
        let mut var = Variables::default();
        var.set(5, 42);
        var.set(6, 5); // var 6 points at var 5
        let c = ctx(&data, &prog, &vit);
        let resolve =
            |p: &[i32], var: &Variables| value(resolve_operand(p, var, &inv, &party, &c, 0, None));
        // Constant (type 0): params[5] verbatim.
        assert_eq!(resolve(&[0, 1, 1, 0, 0, 13, 0], &var), 13);
        // Variable (type 1): value of var 5.
        assert_eq!(resolve(&[0, 1, 1, 0, 1, 5, 0], &var), 42);
        // Var-of-var (type 2): value of var(var 6) = value of var 5 = 42.
        assert_eq!(resolve(&[0, 1, 1, 0, 2, 6, 0], &var), 42);
    }

    #[test]
    fn actor_operand_dispatches_to_the_actor_lookup() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let (party, inv, var) = (Party::default(), Inventory::default(), Variables::default());
        let c = ctx(&data, &prog, &vit);
        // Operand type 5, actor 1, sub-op 0 (level) resolves via `actor_param`.
        let level = value(resolve_operand(
            &[0, 1, 1, 0, 5, 1, 0],
            &var,
            &inv,
            &party,
            &c,
            0,
            None,
        ));
        assert_eq!(level, 1);
    }

    #[test]
    fn character_operand_reads_x_y_and_direction() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let (party, inv, var) = (Party::default(), Inventory::default(), Variables::default());
        let c = ctx(&data, &prog, &vit);
        let facing_up = Some((4, 9, 0));
        let field = |sub: i32| {
            value(resolve_operand(
                &[0, 1, 1, 0, 6, 10001, sub],
                &var,
                &inv,
                &party,
                &c,
                0,
                facing_up,
            ))
        };
        assert_eq!(field(1), 4, "tile X");
        assert_eq!(field(2), 9, "tile Y");
        assert_eq!(field(3), 8, "facing up maps to the RPG_RT code 8");
    }

    #[test]
    fn other_and_item_operands_read_state() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let mut party = Party::default();
        party.add(2);
        let mut inv = Inventory::default();
        inv.add_gold(250);
        inv.add_item(7, 3);
        let var = Variables::default();
        let c = ctx(&data, &prog, &vit);
        let resolve =
            |p: &[i32], secs: u32| value(resolve_operand(p, &var, &inv, &party, &c, secs, None));
        // Other: gold (0), timer seconds (1), party size (2).
        assert_eq!(resolve(&[0, 1, 1, 0, 7, 0, 0], 0), 250);
        assert_eq!(resolve(&[0, 1, 1, 0, 7, 1, 0], 90), 90);
        assert_eq!(resolve(&[0, 1, 1, 0, 7, 2, 0], 0), 2);
        // Item: count of item 7 owned.
        assert_eq!(resolve(&[0, 1, 1, 0, 4, 7, 0], 0), 3);
    }

    #[test]
    fn random_operand_stays_in_range_and_varies() {
        let mut var = Variables::default();
        let mut rng = EventRng::seeded(0xC0FFEE);
        let mut seen = HashSet::new();
        for _ in 0..200 {
            apply_control_variables(
                &mut var,
                &mut rng,
                &[0, 1, 1, 0],
                Operand::Random { lo: 3, hi: 8 },
            );
            let v = var.get(1);
            assert!((3..=8).contains(&v), "roll {v} outside [3, 8]");
            seen.insert(v);
        }
        assert!(seen.len() > 1, "a random operand must not be constant");
    }

    #[test]
    fn random_range_redraws_for_each_variable() {
        let mut var = Variables::default();
        let mut rng = EventRng::seeded(99);
        let mut differed = false;
        for _ in 0..5 {
            apply_control_variables(
                &mut var,
                &mut rng,
                &[1, 1, 3, 0],
                Operand::Random {
                    lo: 0,
                    hi: 1_000_000,
                },
            );
            let (a, b, c) = (var.get(1), var.get(2), var.get(3));
            for v in [a, b, c] {
                assert!((0..=1_000_000).contains(&v));
            }
            differed |= a != b || b != c;
        }
        assert!(
            differed,
            "a random range must redraw per variable, not share one roll"
        );
    }
}

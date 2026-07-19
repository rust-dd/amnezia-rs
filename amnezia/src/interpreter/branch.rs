//! `ConditionalBranch` (opcode 12010) evaluation: whether a branch's condition
//! holds. Mirrors EasyRPG's `CommandConditionalBranch` for the RM2000 condition
//! kinds, sharing the actor lookups with the `ControlVariables` resolver. Kept
//! Bevy-free so each condition unit-tests against plain state resources; the timer
//! condition (kind 2) is evaluated by the caller, which holds the live clock.

use super::actor_query::{ActorCtx, actor_param};
use crate::state::{Inventory, Party, Switches, Variables};

/// Whether a `ConditionalBranch`'s condition holds. Switch (0), variable (1),
/// gold (3), item (4), actor (5), and character orientation (6) are evaluated
/// here; the timer condition (2) is handled by the caller and any unsupported
/// kind returns `true` so its body runs rather than the event stalling. `actors`
/// backs the actor sub-checks and `facing` is the referenced character's
/// orientation (both resolved by the caller from the world). `string` is the
/// command's text, compared by the actor "name is" sub-check.
#[allow(clippy::too_many_arguments)]
pub(super) fn branch_holds(
    params: &[i32],
    string: &str,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
    actors: &ActorCtx,
    facing: Option<u32>,
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
        4 => {
            // Item: params[2] is 0 = having, 1 = NOT having. The "not having" sense
            // was previously dropped, so a "does not have" branch always failed.
            let has = inventory.has(params.get(1).copied().unwrap_or(0) as u32);
            if params.get(2).copied().unwrap_or(0) == 0 {
                has
            } else {
                !has
            }
        }
        5 => actor_branch(params, string, actors, party),
        6 => match facing {
            // Character orientation: true only when the resolved facing matches
            // params[2] (RPG_RT compares the raw facing, up/right/down/left = 0–3).
            Some(dir) => params.get(2).copied() == Some(dir as i32),
            None => false,
        },
        _ => true,
    }
}

/// The actor conditional sub-checks (EasyRPG `case 5`, `params[2]`): in party (0),
/// name is (1), level ≥ (2), HP ≥ (3), skill learned (4), equipment worn (5). The
/// EasyRPG "state affliction" sub-check (6) has no persistent out-of-battle store
/// in this remake and reads false. An id with no actor behind it fails every
/// non-party check. The compared value is `params[3]`.
fn actor_branch(params: &[i32], string: &str, actors: &ActorCtx, party: &Party) -> bool {
    let actor_id = params.get(1).copied().unwrap_or(0).max(0) as u32;
    let sub = params.get(2).copied().unwrap_or(0);
    let arg = params.get(3).copied().unwrap_or(0);
    if sub == 0 {
        return party.has(actor_id);
    }
    let Some(def) = actors.data.actor(actor_id) else {
        return false;
    };
    match sub {
        1 => {
            // Actor 1 carries a live, renamable name; the rest use their def name.
            let name = if actor_id == 1 {
                actors.hero_name
            } else {
                def.name.as_str()
            };
            name == string
        }
        2 => actor_param(0, actor_id, actors) >= arg,
        3 => actor_param(2, actor_id, actors) >= arg,
        4 => actors
            .progression
            .known_skill_ids(def)
            .contains(&(arg.max(0) as u32)),
        5 => [def.weapon, def.shield, def.armor, def.helmet, def.accessory]
            .contains(&(arg.max(0) as u32)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::actor_query::fixtures::game_data;
    use super::*;
    use crate::gamedata::GameData;
    use crate::progression::Progression;
    use crate::vitals::Vitals;

    fn ctx<'a>(
        data: &'a GameData,
        progression: &'a Progression,
        vitals: &'a Vitals,
        hero_name: &'a str,
    ) -> ActorCtx<'a> {
        ActorCtx {
            data,
            progression,
            vitals,
            hero_name,
        }
    }

    #[test]
    fn switch_variable_and_gold_still_hold() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let (mut sw, mut var, party, mut inv) = (
            Switches::default(),
            Variables::default(),
            Party::default(),
            Inventory::default(),
        );
        let c = ctx(&data, &prog, &vit, "Ron");
        let holds = |p: &[i32], sw: &Switches, var: &Variables, inv: &Inventory| {
            branch_holds(p, "", sw, var, &party, inv, &c, None)
        };
        // Switch 4: branch if ON.
        assert!(!holds(&[0, 4, 0], &sw, &var, &inv));
        sw.set(4, true);
        assert!(holds(&[0, 4, 0], &sw, &var, &inv));
        // Variable comparison: var 1 == 6.
        var.set(1, 6);
        assert!(holds(&[1, 1, 0, 6, 0], &sw, &var, &inv));
        assert!(!holds(&[1, 1, 0, 10, 1], &sw, &var, &inv)); // >= 10 false
        // Gold >= 100.
        assert!(!holds(&[3, 100, 0], &sw, &var, &inv));
        inv.add_gold(120);
        assert!(holds(&[3, 100, 0], &sw, &var, &inv));
    }

    #[test]
    fn item_having_and_not_having_take_the_right_branch() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let (sw, var, party) = (Switches::default(), Variables::default(), Party::default());
        let mut inv = Inventory::default();
        let c = ctx(&data, &prog, &vit, "Ron");
        let holds =
            |p: &[i32], inv: &Inventory| branch_holds(p, "", &sw, &var, &party, inv, &c, None);
        // Without item 129: "having" is false, "not having" is true.
        assert!(!holds(&[4, 129, 0], &inv));
        assert!(holds(&[4, 129, 1], &inv));
        inv.add_item(129, 1);
        // With item 129: "having" is true, "not having" is false.
        assert!(holds(&[4, 129, 0], &inv));
        assert!(!holds(&[4, 129, 1], &inv));
    }

    #[test]
    fn actor_sub_conditions_evaluate_the_real_checks() {
        let data = game_data();
        let mut prog = Progression::default();
        let mut vit = Vitals::default();
        prog.set_level(&data.actors[0], 4);
        vit.set(1, 25, 5); // current HP 25
        let (sw, var, inv) = (
            Switches::default(),
            Variables::default(),
            Inventory::default(),
        );
        let mut party = Party::default(); // starts with actor 1
        let c = ctx(&data, &prog, &vit, "Ron");
        let holds = |p: &[i32]| branch_holds(p, "", &sw, &var, &party, &inv, &c, None);
        // In party (sub 0).
        assert!(holds(&[5, 1, 0]));
        assert!(!holds(&[5, 2, 0]));
        // Level ≥ (sub 2): level is 4.
        assert!(holds(&[5, 1, 2, 4]));
        assert!(!holds(&[5, 1, 2, 5]));
        // HP ≥ (sub 3): current HP is 25.
        assert!(holds(&[5, 1, 3, 25]));
        assert!(!holds(&[5, 1, 3, 26]));
        // Skill learned (sub 4): actor 1 knows skill 5, not 6.
        assert!(holds(&[5, 1, 4, 5]));
        assert!(!holds(&[5, 1, 4, 6]));
        // Equipment worn (sub 5): weapon is item 2, not 9.
        assert!(holds(&[5, 1, 5, 2]));
        assert!(!holds(&[5, 1, 5, 9]));
        // A separate party for the name check so actor 1's live name is compared.
        party.add(3);
        assert!(branch_holds(
            &[5, 1, 1],
            "Ron",
            &sw,
            &var,
            &party,
            &inv,
            &c,
            None
        ));
        assert!(!branch_holds(
            &[5, 1, 1],
            "Kyle",
            &sw,
            &var,
            &party,
            &inv,
            &c,
            None
        ));
    }

    #[test]
    fn character_facing_reflects_the_actual_direction() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let (sw, var, party, inv) = (
            Switches::default(),
            Variables::default(),
            Party::default(),
            Inventory::default(),
        );
        let c = ctx(&data, &prog, &vit, "Ron");
        // The character faces down (2): a "faces down" branch holds, "faces up" does not.
        assert!(branch_holds(
            &[6, 10001, 2],
            "",
            &sw,
            &var,
            &party,
            &inv,
            &c,
            Some(2)
        ));
        assert!(!branch_holds(
            &[6, 10001, 0],
            "",
            &sw,
            &var,
            &party,
            &inv,
            &c,
            Some(2)
        ));
        // An unresolved character never matches.
        assert!(!branch_holds(
            &[6, 10001, 2],
            "",
            &sw,
            &var,
            &party,
            &inv,
            &c,
            None
        ));
    }
}

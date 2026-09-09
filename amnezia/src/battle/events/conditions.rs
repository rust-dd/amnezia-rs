use super::{EventWorld, actors};
use crate::battle::logic::check_turn;
use crate::battle::model::Battle;
use amnezia_data::TroopPageConditionDef;

pub(super) fn matches(
    condition: &TroopPageConditionDef,
    turn: u32,
    battle: &Battle,
    world: &EventWorld,
) -> bool {
    let flags = condition.flags;
    if flags & 127 == 0 {
        return false;
    }
    if flags & 1 != 0 && !world.switches.get(condition.switch_a_id)
        || flags & 2 != 0 && !world.switches.get(condition.switch_b_id)
        || flags & 4 != 0 && world.variables.get(condition.variable_id) < condition.variable_value
        || flags & 8 != 0 && !check_turn(turn, condition.turn_b, condition.turn_a)
    {
        return false;
    }
    if flags & 32 != 0
        && !battle
            .enemies
            .get(condition.enemy_index as usize)
            .is_some_and(|enemy| {
                hp_in_range(
                    enemy.hp,
                    enemy.max_hp,
                    condition.enemy_hp_min,
                    condition.enemy_hp_max,
                )
            })
    {
        return false;
    }
    if flags & 64 != 0
        && !actors::health(condition.actor_id, battle, world).is_some_and(|(hp, max)| {
            hp_in_range(hp, max, condition.actor_hp_min, condition.actor_hp_max)
        })
    {
        return false;
    }
    // RPG2000 ignores the fatigue condition; only RPG2003 evaluates it.
    true
}

fn hp_in_range(hp: i32, max: i32, min: u32, upper: u32) -> bool {
    let percent = i64::from(hp.max(0)) * 100 / i64::from(max.max(1));
    (i64::from(min)..=i64::from(upper)).contains(&percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_a_is_the_period_and_turn_b_is_the_first_turn() {
        assert!(check_turn(0, 0, 0));
        assert!(!check_turn(1, 0, 0));
        assert!(!check_turn(0, 3, 0));
        assert!(check_turn(3, 3, 0));
        assert!(!check_turn(4, 3, 0));
        assert!(!check_turn(0, 1, 2));
        assert!(check_turn(1, 1, 2));
        assert!(!check_turn(2, 1, 2));
        assert!(check_turn(3, 1, 2));
    }

    #[test]
    fn hp_conditions_use_truncated_percentages_and_include_both_ends() {
        assert!(hp_in_range(1, 999, 0, 0));
        assert!(hp_in_range(0, 100, 0, 0));
        assert!(hp_in_range(100, 100, 100, 100));
        assert!(!hp_in_range(51, 100, 0, 50));
    }
}

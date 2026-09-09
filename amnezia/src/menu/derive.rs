//! Actor numbers derived for the menu the same way the battle system derives
//! them, so a member's menu figures match the fight exactly. The battle's
//! `logic` module is private, so this mirrors its `actor_hp_sp_at`,
//! `actor_stats_at` (+ `equipment_bonus`), and the RM2000 experience curve behind
//! [`crate::progression`]; keep it in step if those change.

use amnezia_data::{ActorDef, ItemDef};

/// A member's max HP/SP at `level`: the value on their curve (level L at index
/// L-1), or the actor's starting HP/SP when the curve is empty. Mirrors battle's
/// `actor_hp_sp_at(&curves, level, def.hp, def.sp)`.
pub(super) fn max_hp_sp(def: &ActorDef, level: u32) -> (i32, i32) {
    let i = (level.max(1) - 1) as usize;
    let hp = def.curves.max_hp.get(i).copied().unwrap_or(def.hp);
    let sp = def.curves.max_sp.get(i).copied().unwrap_or(def.sp);
    (hp as i32, sp as i32)
}

/// A member's `[atk, def, spi, agi]` at `level` with an explicit five-slot
/// loadout: the curve value (or battle's linear fallback when the curve is empty)
/// plus the summed stat bonus of the equipped items. Mirrors battle's
/// `actor_stats_at` + `equipment_bonus_slots`, so the menu's figures — and its
/// stat-change preview — match a real fight built from the same loadout.
pub(super) fn stats_with_slots(
    def: &ActorDef,
    level: u32,
    items: &[ItemDef],
    slots: [u32; 5],
) -> [u32; 4] {
    let i = (level.max(1) - 1) as usize;
    let c = &def.curves;
    let mut out = match (
        c.attack.get(i),
        c.defense.get(i),
        c.spirit.get(i),
        c.agility.get(i),
    ) {
        (Some(&a), Some(&d), Some(&s), Some(&g)) => [a, d, s, g],
        _ => [16 + level * 6, 8 + level * 4, 8 + level * 3, 8 + level * 2],
    };
    for id in slots {
        if id == 0 {
            continue;
        }
        if let Some(item) = items.iter().find(|item| item.id == id) {
            out[0] += item.atk;
            out[1] += item.def;
            out[2] += item.spi;
            out[3] += item.agi;
        }
    }
    out
}

/// The experience still owed to reach the next level, or `None` at `max_level`.
/// `total` and `level` come from [`crate::progression::Progression`]; the curve
/// below mirrors that module's private `exp_for_level`.
pub(super) fn exp_to_next(def: &ActorDef, total: u32, level: u32) -> Option<u32> {
    if level >= def.max_level {
        None
    } else {
        Some(exp_for_level(level + 1, def).saturating_sub(total))
    }
}

/// Cumulative experience needed to reach `level` (level 1 = 0), a verbatim mirror
/// of [`crate::progression`]'s private curve so the menu's "to next" figure lines
/// up with the level the progression actually awards.
fn exp_for_level(level: u32, def: &ActorDef) -> u32 {
    if level <= 1 {
        return 0;
    }
    let factor = 1.0 + def.exp_inflation as f64 / 100.0;
    let correction = def.exp_correction as f64;
    let mut standard = def.exp_base as f64;
    let mut total = 0.0_f64;
    for _ in 1..level {
        total += standard.floor();
        standard = standard * factor + correction;
    }
    total as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::ActorCurves;

    fn def() -> ActorDef {
        ActorDef {
            critical_hit: false,
            critical_hit_chance: 30,
            state_ranks: Vec::new(),
            attribute_ranks: Vec::new(),
            id: 1,
            name: "Ron".into(),
            title: "Zsoldos".into(),
            level: 1,
            max_level: 10,
            hp: 63,
            sp: 37,
            curves: ActorCurves::default(),
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

    fn weapon(id: u32, atk: u32) -> ItemDef {
        ItemDef {
            actor_set: Vec::new(),
            state_chance: 0,
            id,
            name: "Kard".into(),
            description: String::new(),
            item_type: 1,
            price: 0,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            cure_states: vec![],
            scope: 0,
            only_field: false,
            ko_only: false,
            uses: 0,
            atk,
            def: 0,
            spi: 0,
            agi: 0,
            attribute_defense: vec![],
            state_defense: vec![],
            two_handed: false,
            hit: 0,
            crit: 0,
            weapon_animation: 0,
        }
    }

    #[test]
    fn max_hp_sp_reads_curve_then_falls_back_to_starting_values() {
        let mut d = def();
        d.curves.max_hp = vec![100, 150, 200];
        d.curves.max_sp = vec![10, 20, 30];
        assert_eq!(max_hp_sp(&d, 2), (150, 20));
        let empty = def();
        assert_eq!(max_hp_sp(&empty, 5), (63, 37));
    }

    #[test]
    fn stats_with_slots_reads_curve_and_adds_equipment_bonus() {
        let mut d = def();
        d.curves.attack = vec![20, 30];
        d.curves.defense = vec![10, 15];
        d.curves.spirit = vec![8, 12];
        d.curves.agility = vec![6, 9];
        let items = vec![weapon(7, 5)];
        assert_eq!(
            stats_with_slots(&d, 2, &items, [7, 0, 0, 0, 0]),
            [35, 15, 12, 9]
        );
        let bare = def();
        assert_eq!(
            stats_with_slots(&bare, 1, &items, [7, 0, 0, 0, 0]),
            [16 + 6 + 5, 8 + 4, 8 + 3, 8 + 2]
        );
    }

    #[test]
    fn exp_to_next_is_positive_before_max_and_none_at_max() {
        let d = def();
        let rem = exp_to_next(&d, 0, 1).expect("level 1 still owes exp");
        assert!(rem > 0);
        assert_eq!(exp_to_next(&d, 0, d.max_level), None);
    }
}

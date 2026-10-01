//! Menu statistics matching battle derivation, with the shared experience curve.

use amnezia_data::{ActorDef, ItemDef};

/// Use the shared battle curve lookup, including its starting-vitals fallback.
pub(super) fn max_hp_sp(def: &ActorDef, level: u32) -> (i32, i32) {
    let i = (level.max(1) - 1) as usize;
    let hp = def.curves.max_hp.get(i).copied().unwrap_or(def.hp);
    let sp = def.curves.max_sp.get(i).copied().unwrap_or(def.sp);
    (hp as i32, sp as i32)
}

/// Derive `[atk, def, spi, agi]` from the same curves and loadout bonuses as battle.
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

/// Equipment-adjusted field stats, clamped before applying active state modifiers.
pub(super) fn field_stats(
    def: &ActorDef,
    level: u32,
    items: &[ItemDef],
    slots: [u32; 5],
    vitals: &crate::vitals::Vitals,
) -> crate::battle::logic::Stats {
    use crate::battle::logic::{Stats, state_stats};
    let [attack, defense, spirit, agility] =
        stats_with_slots(def, level, items, slots).map(|value| value.clamp(1, 999));
    let states = vitals
        .states(def.id)
        .into_iter()
        .map(|id| (id, 0))
        .collect::<Vec<_>>();
    state_stats(
        Stats {
            attack,
            defense,
            spirit,
            agility,
        },
        &states,
        crate::conditions::definitions(),
    )
}

/// The experience still owed to reach the next level, or `None` at `max_level`.
pub(super) fn exp_to_next(def: &ActorDef, total: u32, level: u32) -> Option<u32> {
    if level >= def.max_level {
        None
    } else {
        Some(crate::progression::exp_for_level(level + 1, def).saturating_sub(total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::ActorCurves;

    fn def() -> ActorDef {
        ActorDef {
            character_name: String::new(),
            character_index: 0,
            rename_skill: false,
            skill_name: String::new(),
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
            prevent_critical: false,
            raise_evasion: false,
            half_sp_cost: false,
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
    fn menu_experience_uses_the_original_decaying_inflation_curve() {
        let d = def();
        assert_eq!(exp_to_next(&d, 30, 2), Some(54));
        assert_eq!(exp_to_next(&d, 84, 3), Some(88));
        let mut corrected = d;
        corrected.exp_correction = 100;
        assert_eq!(exp_to_next(&corrected, 0, 1), Some(130));
    }

    #[test]
    fn every_original_actor_menu_threshold_is_the_actual_next_level_boundary() {
        let actors = crate::assets::load_ron::<Vec<ActorDef>>(&format!(
            "{}/actors.ron",
            crate::assets::asset_root()
        ));
        assert_eq!(actors.len(), 10);
        for actor in actors {
            for target in 1..actor.max_level {
                let mut progression = crate::progression::Progression::default();
                progression.set_level(&actor, target);
                let level = progression.level(&actor);
                let total = progression.total(&actor);
                let Some(remaining) = exp_to_next(&actor, total, level) else {
                    assert_eq!(level, actor.max_level);
                    continue;
                };
                assert!(remaining > 0, "actor {} level {level}", actor.id);
                progression.add(&actor, remaining - 1);
                assert_eq!(progression.level(&actor), level);
                progression.add(&actor, 1);
                assert!(
                    progression.level(&actor) > level,
                    "actor {} level {level}: displayed threshold {}",
                    actor.id,
                    total + remaining
                );
            }
        }
    }

    #[test]
    fn exp_to_next_is_positive_before_max_and_none_at_max() {
        let d = def();
        let rem = exp_to_next(&d, 0, 1).expect("level 1 still owes exp");
        assert!(rem > 0);
        assert_eq!(exp_to_next(&d, 0, d.max_level), None);
    }
}

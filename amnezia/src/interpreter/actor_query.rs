//! Reading an actor's live parameters — the lookup shared by the
//! `ControlVariables` actor operand (opcode 10220, operand type 5) and the
//! `ConditionalBranch` actor sub-checks (opcode 12010, condition 5). Mirrors
//! EasyRPG's `ControlVariables::Actor`: level/exp from the progression store,
//! current and max HP/SP from the vitals store and stat curve, and the battle
//! stats folding in equipped gear — the same combination a battle builds a
//! fighter from. Kept Bevy-free so it unit-tests against plain state resources.

use crate::battle::{Stats, actor_hp_sp_at, actor_stats_at, equipment_bonus_slots};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::vitals::Vitals;
use amnezia_data::ActorDef;

/// The read-only actor state the actor operand and the actor conditional
/// sub-checks share: the database (defs and equipment), the level/exp store,
/// current HP/SP, and the hero's live name (which only actor 1 carries).
pub(super) struct ActorCtx<'a> {
    pub(super) data: &'a GameData,
    pub(super) progression: &'a Progression,
    pub(super) vitals: &'a Vitals,
    pub(super) equipment: &'a Equipment,
    pub(super) hero_name: &'a str,
}

/// Read one parameter of an actor by its EasyRPG sub-op: 0 level, 1 exp, 2 HP,
/// 3 SP, 4 max HP, 5 max SP, 6 atk, 7 def, 8 spi, 9 agi, 10–14 the equipped item
/// ids. Attack/defence/spirit/agility include the equipped gear's bonus, matching
/// the value a battle would build. An unknown actor or sub-op reads 0.
pub(super) fn actor_param(sub_op: i32, actor_id: u32, ctx: &ActorCtx) -> i32 {
    let Some(def) = ctx.data.actor(actor_id) else {
        return 0;
    };
    let level = ctx.progression.level(def);
    match sub_op {
        0 => level as i32,
        1 => ctx.progression.total(def) as i32,
        2 => current_hp(def, level, ctx.vitals),
        3 => current_sp(def, level, ctx.vitals),
        4 => actor_hp_sp_at(&def.curves, level, def.hp, def.sp).0 as i32,
        5 => actor_hp_sp_at(&def.curves, level, def.hp, def.sp).1 as i32,
        6 => stat(def, level, ctx).attack as i32,
        7 => stat(def, level, ctx).defense as i32,
        8 => stat(def, level, ctx).spirit as i32,
        9 => stat(def, level, ctx).agility as i32,
        10..=14 => ctx.equipment.slots(def)[(sub_op - 10) as usize] as i32,
        _ => 0,
    }
}

/// An actor's curve-derived battle stats at `level` plus its equipped-gear bonus,
/// the same combination the battle builds a fighter from.
fn stat(def: &ActorDef, level: u32, ctx: &ActorCtx) -> Stats {
    let mut s = actor_stats_at(&def.curves, level);
    let bonus = equipment_bonus_slots(ctx.equipment.slots(def), &ctx.data.items);
    s.attack += bonus.attack;
    s.defense += bonus.defense;
    s.spirit += bonus.spirit;
    s.agility += bonus.agility;
    s
}

/// An actor's current HP: its stored vitals, or full at its current level when it
/// has none yet (mirrors the battle's fighter build).
fn current_hp(def: &ActorDef, level: u32, vitals: &Vitals) -> i32 {
    let (max_hp, _) = actor_hp_sp_at(&def.curves, level, def.hp, def.sp);
    vitals
        .get_stored(def.id)
        .map_or(max_hp as i32, |(hp, _)| hp)
}

fn current_sp(def: &ActorDef, level: u32, vitals: &Vitals) -> i32 {
    let (_, max_sp) = actor_hp_sp_at(&def.curves, level, def.hp, def.sp);
    vitals
        .get_stored(def.id)
        .map_or(max_sp as i32, |(_, sp)| sp)
}

#[cfg(test)]
pub(super) mod fixtures {
    use super::*;
    use amnezia_data::{ActorCurves, ItemDef, Learning};

    pub(in crate::interpreter) fn equipment() -> &'static Equipment {
        static EQUIPMENT: std::sync::LazyLock<Equipment> =
            std::sync::LazyLock::new(Equipment::default);
        &EQUIPMENT
    }

    pub(in crate::interpreter) fn actor_def() -> ActorDef {
        ActorDef {
            critical_hit: false,
            critical_hit_chance: 30,
            state_ranks: Vec::new(),
            attribute_ranks: Vec::new(),
            id: 1,
            name: "Ron".into(),
            title: String::new(),
            level: 1,
            max_level: 5,
            hp: 30,
            sp: 10,
            curves: ActorCurves {
                max_hp: vec![40, 50, 60, 70, 80],
                max_sp: vec![10, 12, 14, 16, 18],
                attack: vec![20, 24, 28, 32, 36],
                defense: vec![10, 12, 14, 16, 18],
                spirit: vec![8, 9, 10, 11, 12],
                agility: vec![6, 7, 8, 9, 10],
            },
            learnings: vec![Learning {
                level: 1,
                skill_id: 5,
            }],
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 2,
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

    pub(in crate::interpreter) fn weapon(id: u32, atk: u32) -> ItemDef {
        ItemDef {
            prevent_critical: false,
            raise_evasion: false,
            half_sp_cost: false,
            actor_set: Vec::new(),
            state_chance: 0,
            id,
            name: String::new(),
            description: String::new(),
            item_type: 1,
            price: 0,
            recover_hp: 0,
            recover_hp_rate: 0,
            recover_sp: 0,
            recover_sp_rate: 0,
            cure_states: Vec::new(),
            scope: 0,
            only_field: false,
            ko_only: false,
            uses: 0,
            atk,
            def: 0,
            spi: 0,
            agi: 0,
            attribute_defense: Vec::new(),
            state_defense: Vec::new(),
            two_handed: false,
            hit: 0,
            crit: 0,
            weapon_animation: 0,
        }
    }

    pub(in crate::interpreter) fn game_data() -> GameData {
        GameData {
            actors: vec![actor_def()],
            items: vec![weapon(2, 5)],
            skills: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::game_data;
    use super::*;

    fn ctx<'a>(data: &'a GameData, prog: &'a Progression, vit: &'a Vitals) -> ActorCtx<'a> {
        ActorCtx {
            data,
            progression: prog,
            vitals: vit,
            equipment: fixtures::equipment(),
            hero_name: "Ron",
        }
    }

    #[test]
    fn reads_level_max_hp_and_equipped_attack() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let c = ctx(&data, &prog, &vit);
        assert_eq!(actor_param(0, 1, &c), 1, "level 1");
        assert_eq!(
            actor_param(4, 1, &c),
            40,
            "max HP from the curve at level 1"
        );
        assert_eq!(
            actor_param(6, 1, &c),
            25,
            "attack folds in the equipped weapon"
        );
        assert_eq!(actor_param(10, 1, &c), 2, "the equipped weapon id");
    }

    #[test]
    fn queries_use_the_current_loadout_after_unequipping() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        let mut equipment = Equipment::default();
        equipment.set_slot(&data.actors[0], 0, 0);
        let mut c = ctx(&data, &prog, &vit);
        c.equipment = &equipment;
        assert_eq!(actor_param(10, 1, &c), 0);
        assert_eq!(actor_param(6, 1, &c), 20);
    }

    #[test]
    fn current_hp_defaults_to_full_then_reads_stored_damage() {
        let data = game_data();
        let prog = Progression::default();
        let mut vit = Vitals::default();
        assert_eq!(actor_param(2, 1, &ctx(&data, &prog, &vit)), 40);
        vit.set(1, 17, 4);
        let c = ctx(&data, &prog, &vit);
        assert_eq!(actor_param(2, 1, &c), 17, "stored HP is read back");
        assert_eq!(actor_param(3, 1, &c), 4, "stored SP is read back");
    }

    #[test]
    fn an_unknown_actor_reads_zero() {
        let (data, prog, vit) = (game_data(), Progression::default(), Vitals::default());
        assert_eq!(actor_param(0, 99, &ctx(&data, &prog, &vit)), 0);
    }
}

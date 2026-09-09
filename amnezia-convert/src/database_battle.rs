//! Conversion of the battle-side `RPG_RT.ldb` tables (states, attributes,
//! monsters, troops) into their clean RON assets.

use amnezia_data::{
    AttributeDef, EventCommand, MonsterDef, StateDef, TroopDef, TroopMemberDef,
    TroopPageConditionDef, TroopPageDef,
};
use anyhow::{Context, Result};
use std::path::Path;

/// Convert the state (status condition) table in `input/RPG_RT.ldb` into
/// `output/states.ron` (each state's id, name, action restriction, priority,
/// recovery odds, and per-turn HP-change fields), returning the number of states
/// written. The battle system reads it to apply and lift status conditions and
/// to drain or regenerate HP each turn (e.g. Poison).
pub fn convert_states(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_states(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let states: Vec<StateDef> = parsed
        .into_iter()
        .map(|s| StateDef {
            affect_type: s.affect_type,
            affect_stats: s.affect_stats,
            reduce_hit_ratio: s.reduce_hit_ratio,
            restrict_skill: s.restrict_skill,
            restrict_skill_level: s.restrict_skill_level,
            restrict_magic: s.restrict_magic,
            restrict_magic_level: s.restrict_magic_level,
            sp_change_type: s.sp_change_type,
            sp_change_max: s.sp_change_max,
            sp_change_val: s.sp_change_val,
            rates: s.rates,
            persistence: s.persistence,
            id: s.id,
            name: s.name,
            restriction: s.restriction,
            priority: s.priority,
            hold_turn: s.hold_turn,
            auto_release_prob: s.auto_release_prob,
            release_by_damage: s.release_by_damage,
            hp_change_type: s.hp_change_type,
            hp_change_max: s.hp_change_max,
            hp_change_val: s.hp_change_val,
            hp_change_map_steps: s.hp_change_map_steps,
            hp_change_map_val: s.hp_change_map_val,
        })
        .collect();
    let count = states.len();
    let serialised = ron::to_string(&states).context("serialising states to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("states.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("states.ron").display()))?;
    Ok(count)
}

/// Convert the attribute (element) table in `input/RPG_RT.ldb` into
/// `output/attributes.ron` (each element's id, name, physical/magical type, and
/// A–E resistance-rank damage percentages), returning the number of attributes
/// written. The battle system reads it to scale elemental damage.
pub fn convert_attributes(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed =
        lcf::parse_attributes(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let attributes: Vec<AttributeDef> = parsed
        .into_iter()
        .map(|a| AttributeDef {
            id: a.id,
            name: a.name,
            attribute_type: a.attribute_type,
            a_rate: a.a_rate,
            b_rate: a.b_rate,
            c_rate: a.c_rate,
            d_rate: a.d_rate,
            e_rate: a.e_rate,
        })
        .collect();
    let count = attributes.len();
    let serialised = ron::to_string(&attributes).context("serialising attributes to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("attributes.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("attributes.ron").display()))?;
    Ok(count)
}

/// Convert the enemy table in `input/RPG_RT.ldb` into `output/monsters.ron`
/// (each monster's id, name, combat stats, and exp/gold reward), returning the
/// number of monsters written. The battle system reads it.
pub fn convert_monsters(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed =
        lcf::parse_monsters(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let monsters: Vec<MonsterDef> = parsed
        .into_iter()
        .map(|m| MonsterDef {
            id: m.id,
            name: m.name,
            battler: m.battler,
            max_hp: m.max_hp,
            max_sp: m.max_sp,
            attack: m.attack,
            defense: m.defense,
            spirit: m.spirit,
            agility: m.agility,
            exp: m.exp,
            gold: m.gold,
            attribute_ranks: m.attribute_ranks,
            state_ranks: m.state_ranks,
            actions: m
                .actions
                .into_iter()
                .map(|a| amnezia_data::EnemyActionDef {
                    kind: a.kind,
                    basic: a.basic,
                    skill_id: a.skill_id,
                    enemy_id: a.enemy_id,
                    condition_type: a.condition_type,
                    condition_min: a.condition_min,
                    condition_max: a.condition_max,
                    priority: a.priority,
                    switch_id: a.switch_id,
                    switch_on: a.switch_on,
                    switch_on_id: a.switch_on_id,
                    switch_off: a.switch_off,
                    switch_off_id: a.switch_off_id,
                })
                .collect(),
        })
        .collect();
    let count = monsters.len();
    let serialised = ron::to_string(&monsters).context("serialising monsters to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("monsters.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("monsters.ron").display()))?;
    Ok(count)
}

/// Convert the troop table in `input/RPG_RT.ldb` into `output/troops.ron` (each
/// troop's members and conditional event pages), returning the number written.
pub fn convert_troops(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_troops(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let troops = parsed
        .into_iter()
        .map(|t| TroopDef {
            id: t.id,
            name: t.name,
            pages: t.pages.into_iter().map(convert_troop_page).collect(),
            members: t
                .members
                .into_iter()
                .map(|m| TroopMemberDef {
                    enemy_id: m.enemy_id,
                    x: m.x,
                    y: m.y,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let count = troops.len();
    let serialised = ron::to_string(&troops).context("serialising troops to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("troops.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("troops.ron").display()))?;
    Ok(count)
}

fn convert_troop_page(page: lcf::TroopPage) -> TroopPageDef {
    let c = page.condition;
    TroopPageDef {
        condition: TroopPageConditionDef {
            flags: c.flags,
            switch_a_id: c.switch_a_id,
            switch_b_id: c.switch_b_id,
            variable_id: c.variable_id,
            variable_value: c.variable_value,
            turn_a: c.turn_a,
            turn_b: c.turn_b,
            fatigue_min: c.fatigue_min,
            fatigue_max: c.fatigue_max,
            enemy_index: c.enemy_index,
            enemy_hp_min: c.enemy_hp_min,
            enemy_hp_max: c.enemy_hp_max,
            actor_id: c.actor_id,
            actor_hp_min: c.actor_hp_min,
            actor_hp_max: c.actor_hp_max,
        },
        commands: page
            .commands
            .into_iter()
            .map(|c| EventCommand {
                code: c.code,
                indent: c.indent,
                string: c.string,
                params: c.params,
            })
            .collect(),
    }
}

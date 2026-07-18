//! Conversion of the party-side `RPG_RT.ldb` tables (hero name, actors, items,
//! skills) into their clean RON assets.

use amnezia_data::{ActorCurves, ActorDef, Hero, ItemDef, SkillDef};
use anyhow::{Context, Result};
use std::path::Path;

/// Convert the hero's name (actor 1's default name in `input/RPG_RT.ldb`) into
/// `output/hero.ron`, returning the name written. The game reads it to expand
/// the `\N[k]` message control code.
pub fn convert_hero(input: &Path, output: &Path) -> Result<String> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let actors = lcf::parse_actors(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let name = actors
        .into_iter()
        .find(|a| a.id == 1)
        .map(|a| a.name)
        .context("database has no actor 1")?;
    let hero = Hero { name: name.clone() };
    let serialised = ron::to_string(&hero).context("serialising hero to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("hero.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("hero.ron").display()))?;
    Ok(name)
}

/// Convert the actor table in `input/RPG_RT.ldb` into `output/actors.ron` (each
/// actor's id, name, class title, levels, starting HP/SP, per-level stat curves,
/// experience-curve parameters, initial equipment ids, and the dual-wield /
/// fixed-equipment / unarmed-animation flags), returning the number of actors
/// written. The status and equip menus and the level-up system read it.
pub fn convert_actors(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_actors(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let actors: Vec<ActorDef> = parsed
        .into_iter()
        .map(|a| ActorDef {
            id: a.id,
            name: a.name,
            title: a.title,
            level: a.initial_level,
            max_level: a.max_level,
            hp: a.initial_hp,
            sp: a.initial_sp,
            curves: ActorCurves {
                max_hp: a
                    .stat_curves
                    .max_hp
                    .iter()
                    .map(|&v| v.max(0) as u32)
                    .collect(),
                max_sp: a
                    .stat_curves
                    .max_sp
                    .iter()
                    .map(|&v| v.max(0) as u32)
                    .collect(),
                attack: a
                    .stat_curves
                    .attack
                    .iter()
                    .map(|&v| v.max(0) as u32)
                    .collect(),
                defense: a
                    .stat_curves
                    .defense
                    .iter()
                    .map(|&v| v.max(0) as u32)
                    .collect(),
                spirit: a
                    .stat_curves
                    .spirit
                    .iter()
                    .map(|&v| v.max(0) as u32)
                    .collect(),
                agility: a
                    .stat_curves
                    .agility
                    .iter()
                    .map(|&v| v.max(0) as u32)
                    .collect(),
            },
            exp_base: a.exp_base,
            exp_inflation: a.exp_inflation,
            exp_correction: a.exp_correction,
            weapon: a.weapon,
            shield: a.shield,
            armor: a.armor,
            helmet: a.helmet,
            accessory: a.accessory,
            two_weapons: a.two_weapons,
            fix_equipment: a.fix_equipment,
            unarmed_animation: a.unarmed_animation,
        })
        .collect();
    let count = actors.len();
    let serialised = ron::to_string(&actors).context("serialising actors to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("actors.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("actors.ron").display()))?;
    Ok(count)
}

/// Convert the item table in `input/RPG_RT.ldb` into `output/items.ron` (each
/// item's id, name, description, category, price, the use-effect a consumable
/// applies — recovery amounts, cured states, scope, field-only flag, uses — and
/// the equipment parameters gear applies — stat bonuses, resisted attributes and
/// guarded states, two-handed flag, hit/crit, and weapon animation), returning
/// the number of items written. The shop, item, and equip menus read it.
pub fn convert_items(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_items(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let items: Vec<ItemDef> = parsed
        .into_iter()
        .map(|i| {
            // The raw `state_set`/`attribute_set` sets play different roles by
            // category: on gear (types 1–5) they are the resisted attributes and
            // guarded states, on a consumable the states it cures.
            let is_equipment = matches!(i.item_type, 1..=5);
            let (cure_states, state_defense) = if is_equipment {
                (Vec::new(), i.state_set)
            } else {
                (i.state_set, Vec::new())
            };
            let attribute_defense = if is_equipment {
                i.attribute_set
            } else {
                Vec::new()
            };
            ItemDef {
                id: i.id,
                name: i.name,
                description: i.description,
                item_type: i.item_type,
                price: i.price,
                recover_hp: i.recover_hp,
                recover_hp_rate: i.recover_hp_rate,
                recover_sp: i.recover_sp,
                recover_sp_rate: i.recover_sp_rate,
                cure_states,
                scope: i.scope,
                only_field: i.only_field,
                uses: i.uses,
                atk: i.atk,
                def: i.def,
                spi: i.spi,
                agi: i.agi,
                attribute_defense,
                state_defense,
                two_handed: i.two_handed,
                hit: i.hit,
                crit: i.crit,
                weapon_animation: i.weapon_animation,
            }
        })
        .collect();
    let count = items.len();
    let serialised = ron::to_string(&items).context("serialising items to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("items.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("items.ron").display()))?;
    Ok(count)
}

/// Convert the skill table in `input/RPG_RT.ldb` into `output/skills.ron` (each
/// skill's id, name, description, SP cost, power, hit rate, and battle effect —
/// target scope, type, physical/magical rates, damage variance, HP/SP and
/// absorb flags, and the element and inflicted-state id lists), returning the
/// number of skills written. The skill menu and battle system read it.
pub fn convert_skills(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_skills(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let skills: Vec<SkillDef> = parsed
        .into_iter()
        .map(|s| SkillDef {
            id: s.id,
            name: s.name,
            description: s.description,
            sp_cost: s.sp_cost,
            power: s.power,
            hit: s.hit,
            skill_type: s.skill_type,
            scope: s.scope,
            physical_rate: s.physical_rate,
            magical_rate: s.magical_rate,
            variance: s.variance,
            affect_hp: s.affect_hp,
            affect_sp: s.affect_sp,
            absorb: s.absorb,
            attributes: s.attributes,
            affected_states: s.affected_states,
        })
        .collect();
    let count = skills.len();
    let serialised = ron::to_string(&skills).context("serialising skills to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("skills.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("skills.ron").display()))?;
    Ok(count)
}

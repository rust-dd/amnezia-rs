use amnezia_data::TerrainDef;
use anyhow::{Context, Result};
use std::path::Path;

pub fn convert_terrains(input: &Path, output: &Path) -> Result<usize> {
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let terrains = lcf::parse_terrains(&bytes)?
        .into_iter()
        .map(|t| TerrainDef {
            id: t.id,
            name: t.name,
            damage: t.damage,
            encounter_rate: t.encounter_rate,
            background_name: t.background_name,
            boat_pass: t.boat_pass,
            ship_pass: t.ship_pass,
            airship_pass: t.airship_pass,
            airship_land: t.airship_land,
            bush_depth: t.bush_depth,
        })
        .collect::<Vec<_>>();
    std::fs::create_dir_all(output)?;
    std::fs::write(output.join("terrains.ron"), ron::to_string(&terrains)?)?;
    Ok(terrains.len())
}

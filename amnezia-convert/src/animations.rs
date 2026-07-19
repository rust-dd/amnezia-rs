//! Conversion of the battle-animation table into its clean RON asset.

use amnezia_data::{AnimationCellDef, AnimationDef, AnimationFrameDef, AnimationTimingDef};
use anyhow::{Context, Result};
use std::path::Path;

/// Convert the battle-animation table in `input/RPG_RT.ldb` into
/// `output/animations.ron` (each animation's id, name, `Battle`/`Battle2`
/// graphic, scope and target position, and its per-frame sprite-sheet cell
/// placements plus flash / sound-effect timeline), returning the number of
/// animations written. The battle system reads it to play a skill or attack's
/// on-hit effect.
pub fn convert_animations(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed =
        lcf::parse_animations(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let animations: Vec<AnimationDef> = parsed
        .into_iter()
        .map(|a| AnimationDef {
            id: a.id,
            name: a.name,
            animation_name: a.animation_name,
            scope: a.scope,
            position: a.position,
            frames: a
                .frames
                .into_iter()
                .map(|f| AnimationFrameDef {
                    cells: f
                        .cells
                        .into_iter()
                        .map(|c| AnimationCellDef {
                            valid: c.valid,
                            cell_id: c.cell_id,
                            x: c.x,
                            y: c.y,
                            scale: c.scale,
                            tone_red: c.tone_red,
                            tone_green: c.tone_green,
                            tone_blue: c.tone_blue,
                            tone_gray: c.tone_gray,
                            transparency: c.transparency,
                        })
                        .collect(),
                })
                .collect(),
            timings: a
                .timings
                .into_iter()
                .map(|t| AnimationTimingDef {
                    frame: t.frame,
                    se_name: t.se_name,
                    se_volume: t.se_volume,
                    se_tempo: t.se_tempo,
                    flash_scope: t.flash_scope,
                    flash_red: t.flash_red,
                    flash_green: t.flash_green,
                    flash_blue: t.flash_blue,
                    flash_power: t.flash_power,
                })
                .collect(),
        })
        .collect();
    let count = animations.len();
    let serialised = ron::to_string(&animations).context("serialising animations to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("animations.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("animations.ron").display()))?;
    Ok(count)
}

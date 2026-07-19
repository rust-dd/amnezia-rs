//! Conversion of the map and world-scope tables (maps, party start, chipsets,
//! common events) into their clean RON assets.

use amnezia_data::{Chipset, CommonEvent, Event, EventCommand, EventPage, Map, Start};
use anyhow::{Context, Result};
use std::path::Path;

/// Convert every `MapXXXX.lmu` directly under `input` into a `map_XXXX.ron`
/// under `output/maps/`, returning the number written.
pub fn convert_maps(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let out_dir = output.join("maps");
    let mut count = 0;
    for entry in std::fs::read_dir(input)? {
        let path = entry?.path();
        let is_lmu = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lmu"));
        if !is_lmu {
            continue;
        }
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let number = stem
            .strip_prefix("Map")
            .or_else(|| stem.strip_prefix("map"))
            .unwrap_or(&stem);

        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let unit = lcf::parse_map(&bytes).with_context(|| format!("parsing {}", path.display()))?;
        let events = unit
            .events
            .into_iter()
            .map(|e| Event {
                id: e.id,
                x: e.x,
                y: e.y,
                name: e.name,
                pages: e
                    .pages
                    .into_iter()
                    .map(|p| EventPage {
                        trigger: p.trigger,
                        graphic_name: p.graphic_name,
                        graphic_index: p.graphic_index,
                        direction: p.direction,
                        pattern: p.pattern,
                        move_type: p.move_type,
                        move_frequency: p.move_frequency,
                        move_speed: p.move_speed,
                        layer: p.layer,
                        condition: amnezia_data::EventCondition {
                            flags: p.condition.flags,
                            switch_a: p.condition.switch_a,
                            switch_b: p.condition.switch_b,
                            variable_id: p.condition.variable_id,
                            variable_value: p.condition.variable_value,
                            item_id: p.condition.item_id,
                            actor_id: p.condition.actor_id,
                        },
                        commands: p
                            .commands
                            .into_iter()
                            .map(|c| EventCommand {
                                code: c.code,
                                indent: c.indent,
                                string: c.string,
                                params: c.params,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        let map = Map {
            chipset_id: unit.chipset_id,
            width: unit.width,
            height: unit.height,
            lower: unit.lower_layer,
            upper: unit.upper_layer,
            events,
        };
        let serialised = ron::to_string(&map).context("serialising map to RON")?;
        std::fs::create_dir_all(&out_dir)
            .with_context(|| format!("creating {}", out_dir.display()))?;
        let out = out_dir.join(format!("map_{number}.ron"));
        std::fs::write(&out, serialised).with_context(|| format!("writing {}", out.display()))?;
        count += 1;
    }
    Ok(count)
}

/// Convert the party start in `input/RPG_RT.lmt` into `output/start.ron`,
/// returning the starting map id.
pub fn convert_start(input: &Path, output: &Path) -> Result<u32> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let lmt = input.join("RPG_RT.lmt");
    let bytes = std::fs::read(&lmt).with_context(|| format!("reading {}", lmt.display()))?;
    let parsed = lcf::parse_start(&bytes).with_context(|| format!("parsing {}", lmt.display()))?;
    let start = Start {
        map_id: parsed.map_id,
        x: parsed.x,
        y: parsed.y,
    };
    let serialised = ron::to_string(&start).context("serialising start to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("start.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("start.ron").display()))?;
    Ok(start.map_id)
}

/// Convert the chipset table in `input/RPG_RT.ldb` into `output/chipsets.ron`
/// (a list of `id -> graphic`), returning the number of chipsets written.
pub fn convert_chipsets(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed =
        lcf::parse_chipsets(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let chipsets: Vec<Chipset> = parsed
        .into_iter()
        .map(|c| Chipset {
            id: c.id,
            graphic: c.name,
            passages_down: c.passages_down,
            passages_up: c.passages_up,
        })
        .collect();
    let count = chipsets.len();
    let serialised = ron::to_string(&chipsets).context("serialising chipsets to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("chipsets.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("chipsets.ron").display()))?;
    Ok(count)
}

/// Convert the common-event table in `input/RPG_RT.ldb` into
/// `output/common_events.ron` (each event's id, name, trigger, condition
/// switch, and command list), returning the number written. The interpreter
/// reads it to run global call/autostart/parallel event scripts.
pub fn convert_common_events(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed =
        lcf::parse_common_events(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let events: Vec<CommonEvent> = parsed
        .into_iter()
        .map(|e| CommonEvent {
            id: e.id,
            name: e.name,
            trigger: e.trigger,
            switch_id: e.switch_id,
            commands: e
                .commands
                .into_iter()
                .map(|c| EventCommand {
                    code: c.code,
                    indent: c.indent,
                    string: c.string,
                    params: c.params,
                })
                .collect(),
        })
        .collect();
    let count = events.len();
    let serialised = ron::to_string(&events).context("serialising common events to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("common_events.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("common_events.ron").display()))?;
    Ok(count)
}

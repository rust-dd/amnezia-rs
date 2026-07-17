//! Offline converter from the original RPG Maker 2000 project to the clean
//! intermediate assets the game consumes.

use amnezia_data::{
    ActorDef, Chipset, CommonEvent, Event, EventCommand, EventPage, Hero, ItemDef, Map, MonsterDef,
    SkillDef, Start, TroopDef, TroopMemberDef,
};
use anyhow::{Context, Result};
use std::path::Path;

const GRAPHIC_CATEGORIES: &[&str] = &[
    "Backdrop", "Battle", "CharSet", "ChipSet", "FaceSet", "GameOver", "Monster", "Panorama",
    "Picture", "System", "Title",
];

const TRANSPARENT_CATEGORIES: &[&str] = &[
    "Battle", "CharSet", "ChipSet", "Monster", "Picture", "System",
];

/// Convert every `.xyz` graphic under `input`'s category directories into a
/// PNG under `output/graphics/<Category>/`, returning the number written.
/// Categories in `TRANSPARENT_CATEGORIES` map palette index 0 to a
/// transparent alpha; the rest stay fully opaque.
pub fn convert_graphics(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let mut count = 0;
    for category in GRAPHIC_CATEGORIES {
        let dir = input.join(category);
        if !dir.is_dir() {
            continue;
        }
        let transparent = TRANSPARENT_CATEGORIES.contains(category);
        let out_dir = output.join("graphics").join(category);
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let is_xyz = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("xyz"));
            if !is_xyz {
                continue;
            }
            let bytes =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            let decoded = xyz::decode(&bytes, transparent)
                .with_context(|| format!("decoding {}", path.display()))?;
            let buffer = image::RgbaImage::from_raw(
                decoded.width as u32,
                decoded.height as u32,
                decoded.rgba,
            )
            .context("decoded RGBA buffer has an unexpected size")?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out = out_dir.join(format!("{stem}.png"));
            std::fs::create_dir_all(&out_dir)
                .with_context(|| format!("creating {}", out_dir.display()))?;
            buffer
                .save(&out)
                .with_context(|| format!("writing {}", out.display()))?;
            count += 1;
        }
    }
    Ok(count)
}

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
/// actor's id, name, class title, levels, and starting HP/SP), returning the
/// number of actors written. The status and equip menus read it.
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
/// item's id, name, description, category, and price), returning the number of
/// items written. The shop and item menus read it.
pub fn convert_items(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_items(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let items: Vec<ItemDef> = parsed
        .into_iter()
        .map(|i| ItemDef {
            id: i.id,
            name: i.name,
            description: i.description,
            item_type: i.item_type,
            price: i.price,
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
/// skill's id, name, description, SP cost, power, and hit rate), returning the
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
        })
        .collect();
    let count = skills.len();
    let serialised = ron::to_string(&skills).context("serialising skills to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("skills.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("skills.ron").display()))?;
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
/// troop's id, name, and members with their battle positions), returning the
/// number of troops written. The battle system reads it to build encounters.
pub fn convert_troops(input: &Path, output: &Path) -> Result<usize> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_troops(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let troops: Vec<TroopDef> = parsed
        .into_iter()
        .map(|t| TroopDef {
            id: t.id,
            name: t.name,
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
        .collect();
    let count = troops.len();
    let serialised = ron::to_string(&troops).context("serialising troops to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("troops.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("troops.ron").display()))?;
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

/// Copy the game's audio into `output/audio/`: sound effects (`Sound/*.wav`,
/// which Bevy plays directly) and music (`Music/*`). Music is mostly `*.mid`,
/// staged for a later MIDI-to-audio step, but a handful of ambient loops ship as
/// `*.wav` and play directly. Returns `(sound_effects, music_tracks)` copied.
pub fn convert_audio(input: &Path, output: &Path) -> Result<(usize, usize)> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let audio = output.join("audio");
    let effects = copy_audio_dir(&input.join("Sound"), &audio.join("Sound"), "wav")?;
    let music_out = audio.join("Music");
    let music = copy_audio_dir(&input.join("Music"), &music_out, "mid")?
        + copy_audio_dir(&input.join("Music"), &music_out, "wav")?;
    Ok((effects, music))
}

/// Copy every file with `extension` (case-insensitive) from `dir` into `out`,
/// returning the number copied. A missing `dir` copies nothing.
fn copy_audio_dir(dir: &Path, out: &Path, extension: &str) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let matches = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(extension));
        if !matches {
            continue;
        }
        let name = path.file_name().context("audio file has no name")?;
        std::fs::copy(&path, out.join(name))
            .with_context(|| format!("copying {}", path.display()))?;
        count += 1;
    }
    Ok(count)
}

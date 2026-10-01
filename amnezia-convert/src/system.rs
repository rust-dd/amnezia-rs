//! Conversion of `RPG_RT.ldb` system settings, music and sound effects.

use amnezia_data::{MusicDef, SoundDef, SystemDef};
use anyhow::{Context, Result};
use std::path::Path;

/// Write the `RPG_RT.ldb` system section to `output/system.ron`.
pub fn convert_system(input: &Path, output: &Path) -> Result<()> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let parsed = lcf::parse_system(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let system = SystemDef {
        font_id: parsed.font_id,
        transitions: parsed.transitions,
        title_music: music(parsed.title_music),
        battle_music: music(parsed.battle_music),
        battle_end_music: music(parsed.battle_end_music),
        gameover_music: music(parsed.gameover_music),
        inn_music: music(parsed.inn_music),
        boat_music: music(parsed.boat_music),
        ship_music: music(parsed.ship_music),
        airship_music: music(parsed.airship_music),
        cursor_se: sound(parsed.cursor_se),
        decision_se: sound(parsed.decision_se),
        cancel_se: sound(parsed.cancel_se),
        buzzer_se: sound(parsed.buzzer_se),
        battle_se: sound(parsed.battle_se),
        escape_se: sound(parsed.escape_se),
        enemy_attack_se: sound(parsed.enemy_attack_se),
        enemy_damaged_se: sound(parsed.enemy_damaged_se),
        actor_damaged_se: sound(parsed.actor_damaged_se),
        dodge_se: sound(parsed.dodge_se),
        enemy_defeated_se: sound(parsed.enemy_defeated_se),
        item_se: sound(parsed.item_se),
    };
    let serialised = ron::to_string(&system).context("serialising system to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("system.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("system.ron").display()))?;
    Ok(())
}

fn music(m: lcf::Music) -> MusicDef {
    MusicDef {
        name: m.name,
        volume: m.volume,
        tempo: m.tempo,
        balance: m.balance,
        fadein: m.fadein,
    }
}

fn sound(s: lcf::Sound) -> SoundDef {
    SoundDef {
        name: s.name,
        volume: s.volume,
        tempo: s.tempo,
        balance: s.balance,
    }
}

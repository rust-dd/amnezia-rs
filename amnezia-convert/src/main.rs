//! CLI wrapper for the offline asset converter. Run once at development time;
//! not part of the shipped game.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Convert the original Amnesia RPG Maker 2000 project into clean assets")]
struct Args {
    /// Path to the extracted original project (the `original/` directory).
    #[arg(long)]
    input: PathBuf,
    /// Output directory for the clean assets (e.g. `assets`).
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let images = amnezia_convert::convert_graphics(&args.input, &args.output)?;
    let maps = amnezia_convert::convert_maps(&args.input, &args.output)?;
    let chipsets = amnezia_convert::convert_chipsets(&args.input, &args.output)?;
    let actors = amnezia_convert::convert_actors(&args.input, &args.output)?;
    let items = amnezia_convert::convert_items(&args.input, &args.output)?;
    let skills = amnezia_convert::convert_skills(&args.input, &args.output)?;
    let monsters = amnezia_convert::convert_monsters(&args.input, &args.output)?;
    let troops = amnezia_convert::convert_troops(&args.input, &args.output)?;
    let common_events = amnezia_convert::convert_common_events(&args.input, &args.output)?;
    let start_map = amnezia_convert::convert_start(&args.input, &args.output)?;
    let hero = amnezia_convert::convert_hero(&args.input, &args.output)?;
    let (effects, music) = amnezia_convert::convert_audio(&args.input, &args.output)?;
    println!(
        "converted {images} images, {maps} maps, {chipsets} chipsets, {actors} actors, \
         {items} items, {skills} skills, {monsters} monsters, {troops} troops, \
         {common_events} common events, {effects} sfx, {music} music; start map {start_map}; \
         hero {hero}"
    );
    Ok(())
}

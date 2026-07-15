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
    let count = amnezia_convert::convert_graphics(&args.input, &args.output)?;
    println!("converted {count} images");
    Ok(())
}

# amnezia-rs

Faithful **Bevy/Rust remake** of *Amnézia* — MoonDragon Entertainment's Hungarian RPG Maker 2000 game (2001/2004). Native and playable on macOS, Linux and Windows, with the same gameplay, assets and maps as the original, and containing **zero** RPG Maker 2000 runtime code.

## How it works

The original game data lives in proprietary RPG Maker 2000 formats: `.ldb`/`.lmt`/`.lmu` (the LCF binary format), `.xyz` images and MIDI music. A dev-time converter reads these **once** and emits a clean, engine-agnostic format (PNG + RON, and later OGG). The shipped game loads only that clean format — the legacy formats and the original files never reach the game binary. The Cargo dependency graph enforces this: the `amnezia` game crate depends only on `amnezia-data` and Bevy, never on the `lcf`/`xyz` parser crates.

## Crates

- `xyz` — decoder for the RM2000 `XYZ` image format (dev-time).
- `lcf` — parser for the RM2000 LCF binaries `ldb`/`lmt`/`lmu` (dev-time).
- `amnezia-data` — the clean intermediate format shared by the converter and the game.
- `amnezia-convert` — the offline converter CLI: `original/` → `assets/`.
- `amnezia` — the Bevy game.

## Development flow

1. `original/` holds the extracted original RM2000 project (git-ignored; the converter's input).
2. `cargo run -p amnezia-convert` converts it into `assets/`.
3. `cargo run -p amnezia` runs the game against `assets/`.

## Status

Scaffold + Phase 1 design. See `docs/superpowers/specs/`.

Amnézia is freeware by MoonDragon Entertainment; this remake is non-commercial. Original game © MoonDragon Entertainment 2001/2004.

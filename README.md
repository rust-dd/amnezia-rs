# amnezia-rs

Faithful **Bevy/Rust remake** of *Amnézia* — MoonDragon Entertainment's Hungarian RPG Maker 2000 game (2001/2004). Native and playable on macOS, Linux and Windows, with the same gameplay, assets and maps as the original, and containing **zero** RPG Maker 2000 runtime code.

## How it works

The original game data lives in proprietary RPG Maker 2000 formats: `.ldb`/`.lmt`/`.lmu` (the LCF binary format), `.xyz` images and MIDI music. A dev-time converter reads these **once** and emits a clean, engine-agnostic format (PNG, RON and OGG). The shipped game loads only that clean format — the legacy formats and the original files never reach the game binary. The Cargo dependency graph enforces this: the `amnezia` game crate depends only on `amnezia-data` and Bevy, never on the `lcf`/`xyz` parser crates.

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

## Controls

The game opens on a **title screen** — pick *Új játék* (New Game) or *Folytatás* (Continue) with the arrows and confirm.

| Key | Action |
| --- | --- |
| **Arrow keys** | Walk (hold to keep walking); move the cursor in any menu, choice, shop or battle |
| **Space** / **Enter** | Action: talk to people, open doors, read signs; advance a message; confirm a menu/choice/shop/battle selection |
| **Escape** | Open/close the in-game menu; also backs out of a shop/inn |
| **← / →** (menu open) | Switch tab: Party · Items · Skills |
| **↑ / ↓** (menu open) | Scroll the list |
| **S** (menu open) | Save the game |
| **F5** / **F9** | Quick-save / quick-load (anywhere) |
| **F2** | Toggle the display language: Magyar / English |

Menus, shops, battles and the title all **pause the world** while they are up.

### Debug keys (temporary, dev-only)

| Key | Action |
| --- | --- |
| **P** | Toggle the passability overlay (red = impassable) — the HUD shows the map id, tile and running event |
| **F6** | Start a test battle |
| **F7** / **F8** | Open a test shop / inn |

## Status

Playable end-to-end: title → New Game / Continue → explore, talk, shop, rest, fight, level up, save & resume. Implemented: the event interpreter (messages, switches/variables, conditions, loops, choices, move routes, numeric input, actor reskins, event relocation), tile & autotile rendering (incl. animated water, above-hero occlusion), smooth movement and collision, teleport fades, face portraits, screen effects and pictures, synthesized MIDI music and sound effects, an in-game menu, shops/inns with full healing, a turn-based battle system with experience and level-up, save/load (including party HP/SP and progression), a native 320×240 viewport, and an optional English translation (F2).

Amnézia is freeware by MoonDragon Entertainment; this remake is non-commercial. Original game © MoonDragon Entertainment 2001/2004.

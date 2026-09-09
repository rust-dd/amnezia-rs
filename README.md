# amnezia-rs

A **Bevy/Rust restoration in progress** of *Amnézia* — MoonDragon Entertainment's Hungarian RPG Maker 2000 game (2001/2004). It uses the original assets and maps without requiring the RPG Maker runtime. Native scenarios are tested on macOS; a complete campaign playthrough and Linux/Windows validation are still outstanding.

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
2. `cargo run -p amnezia-convert --bin amnezia-convert --locked -- --input original --output assets` converts it into `assets/`. Add `--data-only` when changing the database/map parser without reconverting graphics or music.
3. `cargo run -p amnezia --locked` runs the game against the converted `assets/`.

### Checks

```sh
cargo fmt --all -- --check
cargo test -p amnezia --locked
cargo test --workspace --exclude amnezia --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Native smoke scenarios require a graphical desktop. Each supplies its own input, captures screenshots in the OS temporary directory (`amnezia-smoke-*.png`), and exits automatically:

```sh
cargo run -p amnezia --locked -- --smoke-test
cargo run -p amnezia --locked -- --smoke-test --smoke-battle-menus
cargo run -p amnezia --locked -- --smoke-test --smoke-battle
cargo run -p amnezia --locked -- --smoke-test --smoke-airship-escape
cargo run -p amnezia --locked -- --smoke-test --smoke-timer
cargo run -p amnezia --locked -- --smoke-test --smoke-pictures
cargo run -p amnezia --locked -- --smoke-test --smoke-colors
```

Run them one at a time. These are focused regression scenarios, not a full campaign playthrough. The battle-menu fixture adds party members, skills, and items for coverage but keeps the actors' real HP/SP.

Add `--smoke-offscreen` to render all camera layers into a GPU texture without a native window, including on a locked desktop. This still requires a working graphics adapter. Captures use the separate `amnezia-smoke-offscreen-*.png` prefix; empty images fail the check. Offscreen runs test rendering and scripted input, not native window/input integration.

### Packaging (macOS)

`bash scripts/bundle-mac.sh` builds the release binary and replaces `target/Amnézia.app`, bundling converted assets inside `Contents/Resources/assets` (dev-only `.mid` intermediates excluded). Release assets are resolved relative to the executable. Packaging and a clean-machine release run still need validation after the current restoration changes.

## Controls

The game opens on a **title screen** — pick *Új játék* (New Game) or *Betöltés* (Load) with the arrows and confirm.

| Key | Action |
| --- | --- |
| **Arrow keys** | Walk (hold to keep walking); move the cursor in any menu, choice, shop or battle |
| **Space** / **Enter** | Action: talk to people, open doors, read signs; advance a message; confirm a menu/choice/shop/battle selection |
| **Escape** | Open the in-game menu or return to the previous menu/target selection |
| **→** (main menu commands) | Select a party member to inspect their status |
| **↑ / ↓** (menu open) | Select a command, party member, or list entry |
| **← / →** (battle items/skills) | Move between the two list columns |
| **F2** | Toggle the display language: Magyar / English |

Amnézia uses save crystals. The menu's Save command and **S** shortcut work only while the original events allow saving. Development saves use `saves/slot1.ron`; macOS release saves use `~/Library/Application Support/Amnezia/saves/slot1.ron`.

### Debug keys

Run `cargo run -p amnezia --locked -- --debug-tools` to enable development shortcuts. They are disabled in release builds. **F3** toggles the diagnostic HUD in a debug build even without this flag.

| Key | Action |
| --- | --- |
| **F3** | Toggle the HUD showing the map id, tile, and running event |
| **P** | Toggle the passability overlay (red = impassable) |
| **F5** / **F9** | Development save/load, subject to active-event and transition guards |
| **F6** | Start a test battle |
| **F7** / **F8** | Open a test shop / inn |

## Status

Core systems are implemented: event interpretation, map rendering and movement, dialogue, music, menus, shops/inns, turn-based battles, progression, and save/load. Automated tests cover original-data regressions including font selection, battle-menu geometry, revival targeting, item consumption, hit chances, and scripted airship movement.

The restoration is not yet verified end-to-end. The [complete restoration checklist](docs/RESTORATION_CHECKLIST.md) tracks all known original-game differences, acceptance criteria, implementation progress, and remaining playthrough/release checks. Passing focused tests does not establish full compatibility with every original event or battle.

Amnézia is freeware by MoonDragon Entertainment; this remake is non-commercial. Original game © MoonDragon Entertainment 2001/2004.

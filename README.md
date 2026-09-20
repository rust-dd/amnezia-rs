# amnezia-rs

A Rust and Bevy restoration of *Amnézia*, MoonDragon Entertainment's Hungarian RPG Maker 2000 game (2001/2004). It runs the original maps and converted assets without the RPG Maker runtime.

This is a work in progress, not a finished remake. Focused native scenarios have been tested on macOS; a complete campaign playthrough, full English localization, and Linux/Windows validation are still outstanding.

## Getting started

You need Git, a current stable Rust toolchain (tested with Rust 1.98.0), a native build toolchain, and a graphics adapter supported by Bevy. On macOS, install the Xcode Command Line Tools.

The converted runtime assets are included in this repository. Access to the private repository is required; you do not need to run the converter just to play.

```sh
git clone git@github.com:rust-dd/amnezia-rs.git
cd amnezia-rs
cargo run -p amnezia --locked
```

The game starts at the title screen. Select *Új játék* (New Game) or *Betöltés* (Load) with the arrow keys and confirm.

On macOS, the game opens centered on the active built-in display, regardless of the focused or primary monitor. If the built-in display is unavailable, it uses the primary display. You can move the window afterward.

## Languages

Hungarian is the original language and the default at startup. **English support is partial, not complete.** Press **F2** to toggle the display language. Text uses the English lookup when available and otherwise keeps the Hungarian source. Already-open text may retain its previous language until the next message or UI refresh.

The English catalog currently contains 6,173 entries: 2,917 differ from the source, while 3,256 are identical. Identical entries include legitimate names and symbols, but also many untranslated Hungarian dialogue lines. All 6,131 keys in the existing Hungarian catalog have an English entry; that is not proof of translation completeness.

The runtime vocabulary has additional gaps: 48 of its 110 nonblank term fields have no English lookup, including the New Game, Load, save/load prompt, and End Game labels. Some inn strings also bypass localization. Translation quality, text fitting, text embedded in graphics, and a complete English playthrough still need verification.

The catalogs are in [assets/i18n](assets/i18n); the runtime lookup is in [amnezia/src/i18n.rs](amnezia/src/i18n.rs).

## Controls

| Key | Action |
| --- | --- |
| Arrow keys | Walk; navigate menus, choices, shops, and battles |
| Space / Enter | Interact, advance a completed message, or confirm |
| Escape | Advance a completed message or release its key-wait; otherwise open the menu or go back |
| Left / Right in battle item/skill lists | Switch list columns |
| F2 | Toggle Hungarian / partial English |
| S | Open saving when the original events allow it |

Enter, Space, and Escape do not skip text while it is being typed, matching the original message window.

Amnézia uses save crystals. The menu's Save command and the S shortcut work only while the original events allow saving. There are fifteen save slots, named `slot1.ron` through `slot15.ron`. Development saves live in `saves/`; macOS release saves live in `~/Library/Application Support/Amnezia/saves/`.

## Development

The converter reads the original `.ldb`, `.lmt`, `.lmu`, `.xyz`, and MIDI files offline and writes RON, PNG, and OGG assets. The game depends on the shared data types, not the legacy-format parsers.

| Crate | Purpose |
| --- | --- |
| `amnezia` | Bevy game runtime |
| `amnezia-data` | Shared, engine-independent data format |
| `amnezia-convert` | Offline asset converter |
| `lcf` | RPG Maker 2000 database and map parser |
| `xyz` | RPG Maker 2000 XYZ image decoder |

To regenerate assets, put the extracted original project in the ignored `original/` directory:

```sh
cargo run -p amnezia-convert --bin amnezia-convert --locked -- --input original --output assets
```

Conversion overwrites generated assets. Add `--data-only` to update structured data without reconverting graphics or music, or `--graphics-only` to update only images. MIDI synthesis also requires `amnezia-convert/assets/soundfont/GeneralUser-GS.sf2`, which is not tracked; the existing converted OGG files do not need it.

### Checks

```sh
cargo fmt --all -- --check
cargo test -p amnezia --locked
cargo test --workspace --exclude amnezia --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

See [Testing](docs/TESTING.md) for native-window and offscreen scenarios, screenshot checks, and their limitations. Run graphical scenarios one at a time; passing them does not establish full campaign compatibility.

### Debug controls

Run `cargo run -p amnezia --locked -- --debug-tools` to enable development shortcuts. These shortcuts are disabled in release builds; F3 also works without the flag in debug builds.

| Key | Action |
| --- | --- |
| F3 | Toggle the map, tile, and event diagnostic HUD |
| P | Toggle the passability overlay |
| F5 / F9 | Development save/load, subject to event and transition guards |
| F6 | Start a test battle |
| F7 / F8 | Open a test shop / inn |

### macOS packaging

`bash scripts/bundle-mac.sh` builds the release binary and replaces `target/Amnézia.app`, bundling converted assets in `Contents/Resources/assets`. Dev-only MIDI intermediates are excluded. Packaging and a clean-machine release run still need validation after the current restoration changes.

## Project status

Implemented systems include event interpretation, map rendering and movement, dialogue, music, menus, shops/inns, turn-based battles, progression, and save/load. The native canvas is 320×240 with whole-pixel scaling and letterboxing.

Choice and number input now share the original bitmap message window, including prompts embedded below preceding dialogue. Both standalone and embedded choices type before accepting input. Remaining work includes message-window lifecycle and transitions, shop/inn presentation, battle text timing, full English localization, and end-to-end campaign and release verification. Original-data regression tests and focused native checks are not a substitute for those checks.

## Credits and third-party materials

Original game © MoonDragon Entertainment, 2001/2004. This is a non-commercial restoration project. The original game assets retain their respective owners' rights; inclusion here does not grant permission to redistribute them.

See the separate notices for the [font](amnezia/fonts/LICENSE.txt) and [conversion soundfont](amnezia-convert/assets/soundfont/LICENSE.txt).

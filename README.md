# Amnézia — Rust restoration

[Project status](#project-status) · [Development checks](#checks) · [Contributing](CONTRIBUTING.md)

A Rust and Bevy restoration of *Amnézia*, MoonDragon Entertainment's Hungarian RPG Maker 2000 game (2001/2004). It runs the original maps and converted assets without the RPG Maker runtime.

Playable on macOS: the bandit story branch has reached the original **VÉGE** screen and happy Ron/Tiffany epilogue. Optional tower puzzles, dragon encounters, strongest weapons for Ron/Tiffany/Stark/Lance, and saving across fifteen slots have also been checked. Final encounters used raised levels; Ron/Tiffany weapon purchases used granted EP. See [Project status](#project-status) for remaining acceptance work.

## Getting started

Download the [1.0.0-rc.0 prerelease](https://github.com/rust-dd/amnezia-rs/releases/tag/v1.0.0-rc.0) to play without installing Rust. Repository access is required. Each archive includes the converted runtime assets:

- **macOS:** extract `amnezia-1.0.0-rc.0-macos-universal.zip` and open `Amnézia.app` on Apple Silicon or Intel. The app is ad-hoc signed and not notarized; macOS may require approval in Privacy & Security after the first launch attempt.
- **Windows x86_64:** extract the entire ZIP and launch `amnezia.exe`.
- **Linux x86_64:** extract the entire `.tar.gz` and run `./amnezia` from the extracted directory. Install the runtime libraries listed in [Windows and Linux packages](#windows-and-linux-packages).

Keep the adjacent `assets` directory in the Windows and Linux packages. The release also includes `SHA256SUMS` for checking the downloaded archives.

You need Git, Rust 1.98.0 (the tested toolchain) or a compatible newer stable toolchain, a native build toolchain, and a graphics adapter supported by Bevy. On macOS, install the Xcode Command Line Tools.

The converted runtime assets are included in this repository. Access to the private repository is required; you do not need to run the converter just to play.

```sh
git clone git@github.com:rust-dd/amnezia-rs.git
cd amnezia-rs
cargo run -p amnezia --locked
```

The game starts at the title screen. Select *Új játék* (New Game) or *Betöltés* (Load) with the arrow keys and confirm.

On macOS, the game opens centered on the active built-in display, regardless of the focused or primary monitor. If the built-in display is unavailable, it uses the primary display. You can move the window afterward.

To build a standalone Mac app, run `bash scripts/bundle-mac.sh` and open `target/Amnézia.app`. It includes graphics, music and runtime data; the RPG Maker runtime and original project files are unnecessary for playing. See [macOS packaging](#macos-packaging) for validation and save locations.

## Languages

Hungarian is the original language and the default at startup. **English support is partial, not complete.** Press **F2** to toggle the display language. Text uses the English lookup when available and otherwise keeps the Hungarian source. Already-open text may retain its previous language until the next message or UI refresh.

The English catalog currently contains 6,173 entries: 2,917 differ from the source, while 3,256 are identical. Identical entries include legitimate names and symbols, but also many untranslated Hungarian dialogue lines. All 6,131 keys in the existing Hungarian catalog have an English entry; that is not proof of translation completeness.

The runtime vocabulary has additional gaps: 48 of its 110 nonblank term fields have no English lookup, including the New Game, Load, save/load prompt, and End Game labels. Inn dialogue now uses the same localization lookup as other runtime terms. Translation quality, text fitting, text embedded in graphics, and a complete English playthrough still need verification.

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
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

GitHub Actions runs formatting, workspace and documentation tests, Clippy, release compilation, and Python syntax checks on macOS. Workspace tests use nextest with a completed/total counter and each test's name and result. Checks that require the untracked original project files or conversion SoundFont are opt-in; default vehicle-parser and MIDI tests use in-memory fixtures. Native gameplay and graphical smoke scenarios remain separate acceptance checks.

Run graphical scenarios one at a time; passing them does not establish full campaign compatibility. Detailed testing notes and archived evidence are local-only under the ignored `docs/` directory. When those archives are available, `python3 scripts/verify-playtest-evidence.py` checks their integrity; CI does not require them.

### Debug controls

Run `cargo run -p amnezia --locked -- --debug-tools` to enable development shortcuts. These shortcuts are disabled in release builds; F3 also works without the flag in debug builds.

| Key | Action |
| --- | --- |
| F3 | Toggle the map, tile, and event diagnostic HUD |
| P | Toggle the passability overlay |
| F5 / F9 | Development save/load, subject to event and transition guards |
| F6 | Start a test battle |
| F7 / F8 | Open a test shop / inn |

### Campaign input driver

Debug builds accept `--playtest /absolute/output/directory`. Add
`--playtest-window` to keep a visible native window with normal audio. Without
that flag, rendering is offscreen and audio is muted. Saves use the output
directory's `saves/` folder.

```sh
cargo run -p amnezia --locked -- --playtest /tmp/amnezia-playtest --playtest-window
python3 scripts/playtest.py /tmp/amnezia-playtest state
python3 scripts/playtest.py /tmp/amnezia-playtest tap enter
```

The driver supplies ordinary key input independently of desktop focus. Visible
commands use elapsed wall time for 60 logical ticks per second, including catch-up
when rendering is slower. Offscreen commands advance one logical tick per render.
Game time pauses between commands; audio continues normally. This permits story
and state verification, but does not establish real-time completion of timed puzzles. Commands and dialogue
remain in the output directory; `state.ron` holds the latest observation. Only
requested screenshots retain a numbered full state snapshot. Diagnostic write
errors are reported without terminating the game. The driver starts at the title screen and does not insert campaign fixtures.

### macOS packaging

`bash scripts/bundle-mac.sh` builds the locked release and produces `target/Amnézia.app`, bundling converted assets in `Contents/Resources/assets`. Dev-only MIDI intermediates are excluded. The script verifies the copied assets and installation data, records an asset checksum manifest, and ad-hoc signs the app. Any previous bundle is retained in the printed staging directory.

For an Apple Silicon and Intel universal app, install the other architecture's Rust target (`rustup target add x86_64-apple-darwin` on Apple Silicon, or `rustup target add aarch64-apple-darwin` on Intel), then run `bash scripts/bundle-mac.sh universal`. The app records the complete package version in `Contents/Resources/Version.txt`; the macOS bundle version uses the numeric `1.0.0` portion.

You can copy the app outside the checkout. Installation diagnostics run without opening a window or audio device:

```sh
'target/Amnézia.app/Contents/MacOS/amnezia' --check-installation
'target/Amnézia.app/Contents/MacOS/amnezia' --check-save-permissions
```

The second command also checks the release save directory with a temporary write/rename/read operation and removes its own probe; existing save slots are untouched. A copied Apple Silicon bundle has passed these checks with filesystem access to the checkout denied. Its title screen, new-game introduction, movement and main menu have also been checked in a native window. This is same-machine gameplay validation. The app is not notarized; native graphical gameplay on Intel macOS, Windows and Linux remains unverified. The separate Release installation checks workflow verifies downloaded packages, their checksums and revision, installation data and save permissions on Windows, Linux, Intel macOS and Apple Silicon macOS.

### Windows and Linux packages

The cross-build scripts produce Windows x86_64 and Linux x86_64 release binaries from the development Mac. Windows needs MinGW-w64 (`x86_64-w64-mingw32-gcc`); Linux needs Zig (tested with 0.14.1), `pkg-config`, `ar`, and the isolated Debian 12 sysroot prepared below. Packaging also needs `curl`, a tar reader with xz support, `rsync` and `zip`.

```sh
rustup target add x86_64-pc-windows-gnu x86_64-unknown-linux-gnu
cargo install cargo-zigbuild --version 0.23.4 --locked --root target/cross-tools --target-dir target/cross-tools/build -j 4
bash scripts/build-cross.sh windows
bash scripts/cross/linux-sdk.sh
bash scripts/build-cross.sh linux /absolute/path/printed/by/the/sdk/script/sysroot
bash scripts/bundle-portable.sh windows
bash scripts/bundle-portable.sh linux
```

The SDK script checks pinned package hashes without installing host libraries. `scripts/bundle-portable.sh` packages each executable with its runtime assets, version, revision, notices and checksums; do not include the build-only SDK. Archive names include the workspace version. Native graphical gameplay on Windows and Linux still needs acceptance testing.

Extract the entire archive before launching `amnezia.exe` on Windows or `./amnezia` on Linux; keep the adjacent `assets` directory. Linux targets glibc 2.36 and requires ALSA, udev, a supported X11/Wayland desktop and a working graphics driver. Release saves use `%LOCALAPPDATA%/Amnezia/saves` on Windows and `$XDG_DATA_HOME/amnezia/saves` (normally `~/.local/share/amnezia/saves`) on Linux.

The `1.0.0-rc.0` GitHub prerelease includes Windows, Linux and universal macOS archives. Full campaign and native platform acceptance remain prerequisites for the stable `1.0.0` release. Building an archive locally does not create a tag or publish a release.

## Project status

The runtime implements map movement and rendering, original event scripts, dialogue, music, menus, shops and inns, turn-based combat, progression, and saving/loading. Gameplay uses complete 60 Hz logical updates independently of rendering, with a 320×240 canvas, whole-pixel scaling and letterboxing.

The completed bandit-route playtest does not establish compatibility for every branch or platform. Remaining acceptance work includes:

- Final combat at naturally earned levels and continuous real-time timed puzzles.
- Other story branches and endings, broader optional-event coverage, and remaining native strongest-weapon/technique checks.
- Complete English translation, text fitting, and an English campaign run.
- Comparison against the running original Windows executable.
- Clean-machine installation, Intel macOS, and native Windows/Linux validation.

Detailed restoration notes, campaign records, screenshots, and archived saves remain local-only; they are not required to build the game or run CI.

## Credits and third-party materials

Original game © MoonDragon Entertainment, 2001/2004. This is a non-commercial restoration project. The original game assets retain their respective owners' rights; inclusion here does not grant permission to redistribute them.

See the separate notices for the [font](amnezia/fonts/LICENSE.txt) and [conversion soundfont](amnezia-convert/assets/soundfont/LICENSE.txt).

# Amnézia — Rust restoration

[Project status](#project-status) · [Development checks](#checks) · [Contributing](CONTRIBUTING.md)

A Rust and Bevy restoration of *Amnézia*, MoonDragon Entertainment's Hungarian RPG Maker 2000 game (2001/2004). It runs the original maps and converted assets without the RPG Maker runtime.

## Getting started

Download the [1.0.0-rc.0 prerelease](https://github.com/rust-dd/amnezia-rs/releases/tag/v1.0.0-rc.0) to play without installing Rust. Repository access is required. Each archive includes the converted runtime assets:

- **macOS:** extract `amnezia-1.0.0-rc.0-macos-universal.zip`, copy `Amnézia.app` to `/Applications`, and open it. The bundle supports Apple Silicon and Intel.
- **Windows x86_64:** extract the entire ZIP and launch `amnezia.exe`.
- **Linux x86_64:** extract the entire `.tar.gz` and run `./amnezia` from the extracted directory. Install the runtime libraries listed in [Windows and Linux packages](#windows-and-linux-packages).

Keep the adjacent `assets` directory in the Windows and Linux packages. The release also includes `SHA256SUMS` for checking the downloaded archives.

The macOS app is ad-hoc signed and not notarized. If macOS blocks it after download, run this in Terminal, then open the app again:

```sh
xattr -dr com.apple.quarantine "/Applications/Amnézia.app"
```

If you installed it elsewhere, use that path instead.

### Build from source

You need Git, Rust 1.98.0 (the tested toolchain) or a compatible newer stable toolchain, a native build toolchain, and a graphics adapter supported by Bevy. On macOS, install the Xcode Command Line Tools.

The converted runtime assets are included in this repository; you do not need to run the converter to play.

```sh
git clone git@github.com:rust-dd/amnezia-rs.git
cd amnezia-rs
cargo run -p amnezia --locked
```

The game starts at the title screen. Select *Új játék* (New Game) or *Betöltés* (Load) with the arrow keys and confirm.

To build a standalone Mac app with bundled assets, run `bash scripts/bundle-mac.sh` and open `target/Amnézia.app`. See [macOS packaging](#macos-packaging) for universal builds and installation checks.

## Languages

Hungarian is the default language. Press **F2** to toggle partial English translation; untranslated text stays Hungarian. Already-open text may keep its previous language until the next message or UI refresh.

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

GitHub Actions also checks documentation tests, release compilation, and Python syntax. Tests requiring the original project files or conversion SoundFont are opt-in; see [Contributing](CONTRIBUTING.md). Run graphical scenarios one at a time.

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

The driver supplies ordinary key input independently of desktop focus and starts
at the title screen. Visible commands advance at 60 logical ticks per second;
offscreen commands advance one logical tick per render. Game time pauses between
commands, so this mode cannot verify continuous real-time timed puzzles.
Commands and dialogue are recorded in the output directory; `state.ron` holds
the latest observation.

### macOS packaging

`bash scripts/bundle-mac.sh` builds `target/Amnézia.app` with converted assets and an ad-hoc signature. Any previous bundle is retained in the printed staging directory.

For an Apple Silicon and Intel universal app, install the other architecture's Rust target (`rustup target add x86_64-apple-darwin` on Apple Silicon, or `rustup target add aarch64-apple-darwin` on Intel), then run `bash scripts/bundle-mac.sh universal`.

You can copy the app outside the checkout. Installation diagnostics run without opening a window or audio device:

```sh
'target/Amnézia.app/Contents/MacOS/amnezia' --check-installation
'target/Amnézia.app/Contents/MacOS/amnezia' --check-save-permissions
```

The second command checks write access to `~/Library/Application Support/Amnezia/saves/` using a temporary file.

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

The SDK script checks pinned package hashes without installing host libraries. `scripts/bundle-portable.sh` packages each executable with its runtime assets, notices and checksums; do not include the build-only SDK.

Extract the entire archive before launching `amnezia.exe` on Windows or `./amnezia` on Linux; keep the adjacent `assets` directory. Linux targets glibc 2.36 and requires ALSA, udev, a supported X11/Wayland desktop and a working graphics driver. Release saves use `%LOCALAPPDATA%/Amnezia/saves` on Windows and `$XDG_DATA_HOME/amnezia/saves` (normally `~/.local/share/amnezia/saves`) on Linux.

## Project status

The runtime implements map movement and rendering, original event scripts, dialogue, music, menus, shops and inns, turn-based combat, progression, and saving/loading. Gameplay uses complete 60 Hz logical updates independently of rendering, with a 320×240 canvas, whole-pixel scaling and letterboxing.

The bandit route has been played through on macOS with development level and EP adjustments. Remaining work includes:

- Final combat at naturally earned levels and continuous real-time timed puzzles.
- Other story branches and endings, and broader optional-event coverage.
- Complete English translation, text fitting, and an English campaign run.
- Comparison against the running original Windows executable.
- Clean-machine installation, Intel macOS, and native Windows/Linux validation.

## Credits and third-party materials

Original game © MoonDragon Entertainment, 2001/2004. This is a non-commercial restoration project. The original game assets retain their respective owners' rights; inclusion here does not grant permission to redistribute them.

See the separate notices for the [font](amnezia/fonts/LICENSE.txt) and [conversion soundfont](amnezia-convert/assets/soundfont/LICENSE.txt).

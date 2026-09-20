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

On macOS, the game opens centered on the active built-in display, regardless of the focused or primary monitor. If the built-in display is unavailable (for example, with the laptop lid closed), it uses the primary display. This only sets the initial position; the window can still be moved normally.

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
cargo run -p amnezia --locked -- --smoke-test --smoke-menu
cargo run -p amnezia --locked -- --smoke-test --smoke-equipment
cargo run -p amnezia --locked -- --smoke-test --smoke-item-menu
cargo run -p amnezia --locked -- --smoke-test --smoke-skill-menu
cargo run -p amnezia --locked -- --smoke-test --smoke-actor-names
cargo run -p amnezia --locked -- --smoke-test --smoke-actor-graphics
cargo run -p amnezia --locked -- --smoke-test --smoke-battle
cargo run -p amnezia --locked -- --smoke-test --smoke-airship-escape
cargo run -p amnezia --locked -- --smoke-test --smoke-timer
cargo run -p amnezia --locked -- --smoke-test --smoke-pictures
cargo run -p amnezia --locked -- --smoke-test --smoke-colors
cargo run -p amnezia --locked -- --smoke-test --smoke-font-colors
cargo run -p amnezia --locked -- --smoke-test --smoke-display
cargo run -p amnezia --locked -- --smoke-test --smoke-animation-colors
cargo run -p amnezia --locked -- --smoke-test --smoke-map-animations
cargo run -p amnezia --locked -- --smoke-test --smoke-world-tones
cargo run -p amnezia --locked -- --smoke-test --smoke-map-flashes
cargo run -p amnezia --locked -- --smoke-test --smoke-ui-layers
cargo run -p amnezia --locked -- --smoke-test --smoke-water
cargo run -p amnezia --locked -- --smoke-test --smoke-transitions
cargo run -p amnezia --locked -- --smoke-test --smoke-screen-events
cargo run -p amnezia --locked -- --smoke-test --smoke-weather
cargo run -p amnezia --locked -- --smoke-test --smoke-battle-transitions
cargo run -p amnezia --locked -- --smoke-test --smoke-gameover
cargo run -p amnezia --locked -- --smoke-test --smoke-battle-defeat
cargo run -p amnezia --locked -- --smoke-test --smoke-return-title
cargo run -p amnezia --locked -- --smoke-test --smoke-save-music
cargo run -p amnezia --locked -- --smoke-test --smoke-save-slots
cargo run -p amnezia --locked -- --smoke-test --smoke-load-slots
cargo run -p amnezia --locked -- --smoke-test --smoke-save-camera
cargo run -p amnezia --locked -- --smoke-test --smoke-save-pictures
cargo run -p amnezia --locked -- --smoke-test --smoke-save-screen
cargo run -p amnezia --locked -- --smoke-test --smoke-save-weather
cargo run -p amnezia --locked -- --smoke-test --smoke-save-animations
cargo run -p amnezia --locked -- --smoke-test --smoke-save-npcs
cargo run -p amnezia --locked -- --smoke-test --smoke-save-hero
cargo run -p amnezia --locked -- --smoke-test --smoke-save-vehicles
cargo run -p amnezia --locked -- --smoke-test --smoke-dialogue-timing
```

Run them one at a time. These are focused regression scenarios, not a full campaign playthrough. The battle-menu fixture adds party members, skills, and items for coverage but keeps the actors' real HP/SP.

The `menu` scenario also checks equipment selection over 92 states and 10 exact sounds: swapping the original Ton-Kard for Karpenge, removing equipment with an empty bag, cancellation, and actor 9's original equipment lock. Candidate items retain database id order with the empty removal entry last.

The main command window, member selection and End Game prompt use original wrapping, PageUp/PageDown and 24/4-frame held-arrow repetition. Window navigation runs before decision/cancel, with separate sounds for simultaneous directions and no false page-key sounds at a boundary. A single member still produces arrow sounds; an empty party does not. Transitions and file selection block input without resetting the global hold phase. The extended `menu` scenario adds 312 state/audio frames, 29 exact sounds and four cursor captures covering 1,136 original skin pixels; existing layout, portrait and text references remain in place.

The main command, gold and status windows use fixed-point native background rasters as well. Their six layout/cursor and three status-text captures now compare complete images: 691,200 pixels covering all three backgrounds, portraits, text, both selection cursors, resizing, disabled commands and an empty party. The full offscreen scenario passes; the new native comparison remains pending while the Mac is locked.

The End Game windows also use the original fixed-point background sampling. Changing a label resizes the cached native background without replacing its window or background entity. The `return-title` scenario compares three complete confirmation images against 230,400 reference pixels, including the background and both cursor phases. The offscreen run passes; the latest native repeat is pending because macOS locked and put the built-in display to sleep.

The `equipment` scenario verifies the original four-window layout, native bitmap text, original labels, current and preview stats, two-column candidates, six visible rows and separate slot/item cursors. Thirty-one full captures compare 2,380,800 reference pixels; 916 state/audio frames check 67 exact sounds, a real weapon swap, the four-frame scroll, blank unequip preview, inactive-list offsets, page keys, slot wrapping, fixed equipment and long text. Stats clamp before state modifiers, and comparison updates precede item navigation as in the reference. Both native-window and offscreen runs pass. Cancel returns directly to the equipment command; re-entering starts member selection at the first actor. Fresh Left/Right switches actors through the original six-frame erase and six-frame show, retaining the slot and restarting all five list banks only at the black midpoint. Eight complete transitions verify frozen windows, blocked input and two full blended references. Held keys do not cycle repeatedly, fixed actors remain viewable, and item-picker arrows still move items. Item, skill and equipment cells clear their original 144×12 area before drawing, including the blank unequip cell. Long-name tests preserve the uncleared gap and bottom shadow, distinguish an absent neighbor, and verify raster-cache refresh. Three equipment captures exercise overlap, an empty neighbor and a cleared list. Other field-menu scene fades remain a separate restoration task.

The `item-menu` scenario drives the original two-column field inventory with actual keys. Eleven complete captures compare 844,800 reference pixels: empty inventory, original descriptions and disabled colors, both directions of four-frame scrolling, clipped rows, cursor/arrow phases, and selection retained after using an item. It checks recovery against Ron's real 63 HP maximum, item counts, and return navigation. The target must stay open after consuming the last item; another confirmation fails without changing HP or inventory. It also verifies the exact item-use and buzzer requests. The background uses the reference renderer's fixed-point sampling; it is cached independently of the original system image. Eighteen further full captures verify 1,382,400 item/skill target-window pixels: original three-window layout, four portraits, native bitmap text and colors, fixed/self/whole-party cursors, half SP cost, item depletion, and long-name clipping. Successful field skills keep the target open and play only the animation's first enabled sound immediately; ineffective casts buzz without spending SP. Enter/Space, live HP/SP refresh and returning to the same skill are checked. A further 207 input states and 17 exact sound requests cover held-arrow wrapping, 24/4-frame repetition, page keys, and immovable self/party targets. General menu scene fades and the separate original scene-frame counter remain restoration tasks.

The `skill-menu` scenario drives the original help/status/list windows with actual keys. Seventeen full captures compare 1,305,600 reference pixels, including both directions of four-frame scrolling, empty and disabled entries, half SP cost, live names and clipped long text. It checks 702 audio-observation frames and 38 exact sound requests. Two real field casts update Tiffany's SP from 75 to 60 to 45 and Ron's HP from 7 to 57 to 63; returning from the target preserves the selected skill and scroll position. Leaving the skill scene returns directly to its main command; re-entering starts member selection at the first actor. Another 83 states and eight exact sounds verify a continuously held arrow across closed menus, a transition, a fixed target and a switch to the item list. The long-list fixture temporarily learns original database skills without changing assets or player saves. Both offscreen and native-window runs are supported.

The `save-npcs` scenario uses an isolated temporary save to check NPC position, appearance, in-progress movement and route continuation through a real map rebuild. It compares three rendered poses with the original character sheet against an isolated black background and verifies legacy NPC defaults without rewriting the old file.

The `save-hero` scenario saves during a jump with a following camera. It checks exact motion and camera restoration throughout the loading fade, the original reset from temporary route graphics to the party leader's costume, continued movement without repeated commands and legacy defaults. Three rendered poses are checked against 2304 original character/background pixels. It uses an isolated slot.

The `save-vehicles` scenario saves during airship ascent and during simultaneous boat, ship and airship movement. It checks exact state throughout both loading fades, rider/camera synchronization, retained temporary vehicle graphics and opacity, completed routes and legacy defaults. Five captures compare 11,520 original character, opacity and isolated-background pixels; the mounted hero must remain hidden. It uses its own temporary slot. Foreground interpreter continuation is checked by `save-music`.

The `dialogue-timing` scenario checks the original Tiffany/Ron slow headers at 60/144 FPS, rendered text visibility, a long pause and automatic page closure. It verifies the pause arrow's logical cycle and four visible/hidden captures against 512 source-image pixels. It presses keys during typing, closes completed pages/choices with Escape, edits a number and checks that no menu opens accidentally. It also verifies message ownership, all three original waiting key queries and the Draco's parallel held-Cancel query. These checks do not establish full font, window-animation or original-executable visual parity.

The `font` scenario checks dialogue bitmap glyphs, palette/shadow colors, portrait placement, four-line spacing, window background/frame composition, transparency and long-line clipping against the source graphics. It covers top, middle and bottom placement. Pause-arrow timing is covered by `dialogue-timing`; message opening/closing remains a separate restoration task.

A successful run logs `completed all final checks` and returns exit code 0. Closing the test window before verification finishes returns a nonzero status. The diagnostic `--smoke-test --smoke-close-early` deliberately closes its own window early and must return exit code 1; it is a failure-path check, not a gameplay scenario.

Add `--smoke-offscreen` to render all camera layers into a GPU texture without a native window, including on a locked desktop. This still requires a working graphics adapter. Captures use the separate `amnezia-smoke-offscreen-*.png` prefix; empty images fail the check. Offscreen runs test rendering and scripted input, not native window/input integration.

The `save-music` scenario uses its own temporary save directory, never the player's slot. Two original crystal command lists save to the first and fifteenth slots, verifying that saving suspends the event and writes the selected file before its remaining commands run. The first file stays unchanged throughout the second slot's save and reloads. Reloading must preserve the exact foreground execution state throughout the entire fade, then resume its tail once without rewriting the slot. The scenario checks actual map reloads and decoded audio playback for current music, memorized music, saved silence and legacy saves. Loading from an erased screen must start black and reveal the map after the transition; this checks visibility, not original-map pixel parity. It also reopens messages after loading to check portrait pixels, a cleared face, placement, transparency and legacy message defaults. The legacy load also repairs out-of-range HP/SP, gold, item counts, variables, experience and countdown values. It preserves the first four unique valid party members and removes invalid item, gear, skill and state references without rewriting the file. It removes its own fixture files on success.

The `save-slots` scenario drives the fifteen-slot save selector using Enter, Escape, arrows and Page Up. It checks an empty list, cancellation without writing, a real save to slot 2, saved-party previews, a corrupt fixture, scrolling and overwriting slot 15 without changing its neighbours. Original map 2/260 crystal commands also open the selector: cancellation preserves the file, confirmation saves before the event tail runs once, and an event-erased screen cannot hide the selector. Eleven captures compare 328,178 original window, bitmap text, cursor, arrow, portrait and fade pixels. These include the original six-frame scene fades, manual cancellation at a cursor/arrow phase boundary, and crystal entry from visible and erased screens. Saving happens before the exit fade; returning to the menu or map finishes before input and events resume. Both selector scenarios use opposite saved-timestamp and filesystem-date ordering, so file copying cannot silently choose the wrong starting slot. Legacy files without timestamps retain their filesystem-date fallback. Its temporary files are isolated from player saves and removed on success. The other save scenarios also navigate and confirm the actual selector, with a bounded wait for the dialog.

The `load-slots` scenario starts at the title without a first-slot file, browses the fifteen-slot load list, rejects corrupt and empty slots, cancels back to the fully open title command window without restarting music, then loads slot 15 into the actual map. Seven pixel-checked captures cover 309,676 original UI and fade samples, including the six-frame entry fade with its initially hidden arrows. File-window regressions also cover the initial cursor step, decision-frame updates, paused clocks and simultaneous confirmation/navigation without changing the confirmed slot. The selected name, party and HP are verified after loading, and all three temporary files must remain byte-for-byte unchanged before cleanup. No player saves are used.

The `gameover` scenario returns from an erased map through Game Over to the title, selects New Game with Enter and completes the intro. It checks that the raw 60 Hz transition clock resets on selection, counts the six-frame title exit and continues through the map rebuild without resetting again. This does not verify the separate original scene-frame counter, which must pause during asynchronous transitions and remains a restoration task. The outgoing title capture compares all 76,800 pixels with the original graphics and fade blend, alongside the existing Game Over image check.

The `save-camera` scenario also uses an isolated temporary slot. It saves a locked camera during a pan, verifies restoration after rebuilding the map, continues the pan, unlocks following and checks legacy centering without camera data.

The `save-pictures` scenario saves moving, tinted and waving pictures alongside screen-pinned and map-anchored graphics. Its isolated slot checks exact restoration, continued animation, 18 original graphic samples while the camera moves, and clearing stale pictures when loading empty or legacy saves.

The `screen-events` scenario also checks strong and weak FlashScreen commands against byte-level reference blends on black, colored and white backgrounds. Seven captures verify the decay and unchanged UI above the flash. Additional transfers verify that local teleports freeze and retain the flash, while a different map clears it permanently.

The `map-animations` scenario checks moving character targets against original animation pixels. A local teleport keeps the same visible cells frozen throughout both fades; playback resumes afterward. Rebuilding a different map clears the animation and its remaining flashes.

The `weather` scenario executes original rain commands from Map0028/0038. Eight captures check the 6×24 streak, all three strengths, lifetime alpha, 320×160 wrapping, tone, camera pan/shake and unchanged UI against reference pixels. It also checks real menu pauses and map transfers, then shows the original forest and a live battle. The snow/fog compatibility renderers are separate; original game events only use rain.

The `save-screen` scenario uses an isolated slot to restore an unfinished tone change, flash and camera shake. It opens and closes the actual menu to check that effects pause and resume, samples toned world and untinted foreground pixels, and loads legacy screen defaults.

The `save-weather` scenario restores weather scrolling during a camera pan, with fresh raindrops on load. Three captures check the continued rain against reference pixels; the isolated slot also covers legacy defaults, unchanged legacy files and scrolling saved while weather is disabled.

The `save-animations` scenario saves the original Robbanás1 map animation, restores its exact playback phase and target flash, and checks that the reload fade holds it still. Two captures compare the resumed effect against 1824 original bitmap pixels. Legacy and explicitly empty slots must clear stale effects; all files belong to the isolated fixture.

The game renders its world, text and menus together at 320×240, then scales the result in whole physical pixels with black letterboxing. Windows smaller than the native canvas are downscaled proportionally. The display scenario checks normal, wide, portrait, odd-sized and small outputs against direct GPU readbacks of the native canvas. On macOS, its native run also verifies the visible window's actual monitor against the built-in-display preference.

### Packaging (macOS)

`bash scripts/bundle-mac.sh` builds the release binary and replaces `target/Amnézia.app`, bundling converted assets inside `Contents/Resources/assets` (dev-only `.mid` intermediates excluded). Release assets are resolved relative to the executable. Packaging and a clean-machine release run still need validation after the current restoration changes.

## Controls

The game opens on a **title screen** — pick *Új játék* (New Game) or *Betöltés* (Load) with the arrows and confirm.

| Key | Action |
| --- | --- |
| **Arrow keys** | Walk (hold to keep walking); move the cursor in any menu, choice, shop or battle |
| **Space** / **Enter** | Action: talk to people, open doors, read signs; advance a message; confirm a menu/choice/shop/battle selection |
| **Escape** | Advance a completed message or release its key-wait; otherwise open the in-game menu or return to the previous menu/target selection |
| **↑ / ↓** (menu open) | Select a command, party member, or list entry |
| **← / →** (battle items/skills) | Move between the two list columns |
| **F2** | Toggle the display language: Magyar / English |

Enter, Space and Escape do not skip text while it is being typed, matching the original message window.

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

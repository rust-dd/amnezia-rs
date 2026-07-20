//! Single-slot save and load: `F5` snapshots the full game state (switches,
//! variables, party, inventory, plus the hero's map, tile, and facing) to a RON
//! file at a fixed, working-directory-independent path (see [`save_dir`]); `F9`
//! restores it. A load rebuilds the world by restoring the resources and then
//! reusing the teleport machinery to reload the saved map at the saved tile — the
//! fade's `swap_map` reads the freshly restored state when it respawns the map's
//! events.

use crate::dialogue::Dialogue;
use crate::equipment::Equipment;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::progression::Progression;
use crate::screenfx::{TintState, Weather, WeatherStrength};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport};
use crate::text::HeroName;
use crate::timer::{GameClock, PlayTime};
use crate::vitals::Vitals;
use crate::world::MapData;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The save slot's file name within the save directory.
const SLOT_FILE: &str = "slot1.ron";

/// The directory holding save slots, resolved once and independent of the current
/// working directory. The old CWD-relative `"saves/slot1.ron"` meant the crystal
/// save and the title's Continue could resolve *different* files whenever the game
/// was launched from another directory (notably the packaged `.app`, whose CWD is
/// not the project root): a fresh save then appeared to vanish, or a stale slot
/// loaded, so Continue "restarted from the beginning". Debug builds keep the
/// in-tree `saves/` beside the workspace; release builds resolve it next to the
/// executable, mirroring [`crate::assets::asset_root`], so a save written on one
/// launch is found on the next regardless of the working directory.
fn save_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if cfg!(debug_assertions) {
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../saves"))
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join("saves")))
                .unwrap_or_else(|| PathBuf::from("saves"))
        }
    })
    .as_path()
}

/// The single v1 save slot's resolved, working-directory-independent path.
fn save_path() -> PathBuf {
    save_dir().join(SLOT_FILE)
}

/// A request to load the save slot, honoured by [`save_or_load`] on the next
/// frame exactly as if `F9` had been pressed. The title screen's "Betöltés"
/// (Continue) sets it so a resume reuses the same restore path without duplicating
/// the load body.
#[derive(Resource, Default)]
pub struct LoadRequest(pub bool);

/// A request to save, honoured by [`save_or_load`] as if `F5` had been pressed.
/// The in-game menu's Save action sets it, reusing the same snapshot path.
#[derive(Resource, Default)]
pub struct SaveRequest(pub bool);

/// A save requested by the event interpreter (opcode 11910, `OpenSaveMenu`) — the
/// save crystal. Unlike [`SaveRequest`], [`save_or_load`] honours it even while
/// the requesting event is still `running.active()`; only a fade defers it. Kept
/// distinct so the F5 / menu save stays gated behind a running event as before.
#[derive(Resource, Default)]
pub struct EventSaveRequest(pub bool);

/// Whether the in-menu Save command is allowed (RM2000 `ChangeSaveAccess`, opcode
/// 11930). Defaults enabled; a cutscene disables it to block manual saves and
/// re-enables it afterwards. Only the menu's Save entry (and the `Esc`-`S` quick
/// save) honour this — the save crystal ([`EventSaveRequest`], `OpenSaveMenu`) and
/// the `F5` dev hotkey are deliberately ungated, matching RPG_RT's `SetAllowSave`.
#[derive(Resource)]
pub struct SaveAccess(pub bool);

impl Default for SaveAccess {
    fn default() -> Self {
        Self(true)
    }
}

/// The resolved path the save/load systems read and write. Held as a resource so a
/// headless test can point them at a temp file; the real game uses the
/// working-directory-independent [`save_path`].
#[derive(Resource)]
pub struct SaveLocation(pub PathBuf);

impl Default for SaveLocation {
    fn default() -> Self {
        Self(save_path())
    }
}

/// Whether the single save slot exists on disk, for the title's Continue gate.
pub fn save_slot_exists() -> bool {
    slot_exists(&save_path())
}

/// Whether `path` names an existing file. Split out so the gate is testable
/// without depending on the resolved [`save_path`].
fn slot_exists(path: &Path) -> bool {
    path.exists()
}

/// A serialisable snapshot of the whole runtime game state. Maps are stored as
/// sorted-order-independent `Vec`s of pairs so RON stays diffable and stable.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SaveGame {
    map_id: u32,
    x: u32,
    y: u32,
    dir: u32,
    switches: Vec<(u32, bool)>,
    variables: Vec<(u32, i32)>,
    party: Vec<u32>,
    items: Vec<(u32, u32)>,
    gold: i32,
    #[serde(default)]
    progression: Vec<(u32, u32)>,
    #[serde(default)]
    vitals: Vec<(u32, (i32, i32))>,
    // Added after #46; every field is `#[serde(default)]` so slots written before
    // them still load. The hero's name and current CharSet graphic default empty —
    // an old slot then keeps the boot default rather than being blanked — and the
    // tone defaults to neutral so an untinted slot isn't blacked out on load.
    #[serde(default)]
    hero_name: String,
    #[serde(default)]
    charset: String,
    #[serde(default)]
    charset_index: u32,
    #[serde(default = "neutral_tone")]
    tone: (i32, i32, i32, i32),
    #[serde(default)]
    weather: i32,
    #[serde(default)]
    weather_strength: i32,
    // The runtime equipment store (per-actor five-slot loadouts), so a Continue
    // keeps gear changed on the equip screen. `#[serde(default)]` keeps pre-#47
    // slots loadable — they carry no entries, so every actor restores to its
    // `ActorDef` starting gear, exactly the state those saves were written in.
    #[serde(default)]
    equipment: Vec<(u32, [u32; 5])>,
    // Playtime and the RM2000 game timer, added after #48. `#[serde(default)]` keeps
    // older slots loadable — they resume with a zeroed playtime and a stopped timer.
    #[serde(default)]
    playtime: u64,
    #[serde(default)]
    timer_remaining: f32,
    #[serde(default)]
    timer_running: bool,
}

/// The RM2000 neutral screen tone (every channel 100), the [`SaveGame::tone`]
/// default so a slot saved before the tone was persisted loads without tinting
/// the screen — a plain `(0, 0, 0, 0)` default would black it out.
fn neutral_tone() -> (i32, i32, i32, i32) {
    (100, 100, 100, 100)
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadRequest>()
            .init_resource::<SaveRequest>()
            .init_resource::<EventSaveRequest>()
            .init_resource::<SaveAccess>()
            .init_resource::<SaveLocation>()
            .add_systems(Update, save_or_load);
    }
}

/// The save/load request flags, the resumed marker, and the resolved slot path,
/// bundled into one `SystemParam` so [`save_or_load`] stays within Bevy's
/// 16-parameter cap.
#[derive(SystemParam)]
struct SaveIo<'w> {
    load_request: ResMut<'w, LoadRequest>,
    save_request: ResMut<'w, SaveRequest>,
    event_save: ResMut<'w, EventSaveRequest>,
    location: Res<'w, SaveLocation>,
    // The runtime equipment store, snapshotted on save and restored on load
    // beside the party and inventory. Bundled here so [`save_or_load`] stays
    // within Bevy's 16-parameter cap.
    equipment: ResMut<'w, Equipment>,
}

/// The scene resources a save now also snapshots and restores beyond the core
/// state: the hero's name ([`HeroName`]), the active weather ([`Weather`] and its
/// [`WeatherStrength`]), and the screen tone ([`TintState`]). Bundled into one
/// `SystemParam` so [`save_or_load`] stays within Bevy's 16-parameter cap.
#[derive(SystemParam)]
struct SceneState<'w> {
    hero_name: ResMut<'w, HeroName>,
    weather: ResMut<'w, Weather>,
    weather_strength: ResMut<'w, WeatherStrength>,
    tone: ResMut<'w, TintState>,
    playtime: ResMut<'w, PlayTime>,
    game_clock: ResMut<'w, GameClock>,
}

/// What [`save_or_load`] does this frame once a fade has been ruled out.
#[derive(PartialEq, Eq, Debug)]
enum Action {
    Save,
    Load,
}

/// Decide the frame's action. A dialogue or a still-running event (`gated`) holds
/// back the hotkey/menu save (`hotkey_save`) and the `F9` dev load (`hotkey_load`),
/// but an interpreter-originated save (`event_save`, opcode 11910) and the title's
/// Continue (`menu_load`, [`LoadRequest`]) both bypass that gate: an event save
/// fires while its own event is deliberately still running, and a resume must never
/// be refused just because the boot intro could still be `running.active()`. A save
/// takes precedence over a load requested in the same frame.
fn resolve(
    event_save: bool,
    hotkey_save: bool,
    menu_load: bool,
    hotkey_load: bool,
    gated: bool,
) -> Option<Action> {
    if event_save || (hotkey_save && !gated) {
        Some(Action::Save)
    } else if menu_load || (hotkey_load && !gated) {
        Some(Action::Load)
    } else {
        None
    }
}

/// Handle the save (`F5`) and load (`F9`) hotkeys, the menu's Save action, and the
/// interpreter's `OpenSaveMenu` (opcode 11910). A fade defers everything so a
/// snapshot is never taken or applied mid-transition. A dialogue or running event
/// additionally holds back the hotkey/menu save and any load, but not the
/// interpreter save — the save crystal saves while its own event still runs.
#[allow(clippy::too_many_arguments)]
fn save_or_load(
    keys: Res<ButtonInput<KeyCode>>,
    dialogue: Res<Dialogue>,
    fade: Res<Fade>,
    running: Res<RunningEvent>,
    map_data: Option<Res<MapData>>,
    mut switches: ResMut<Switches>,
    mut variables: ResMut<Variables>,
    mut party: ResMut<Party>,
    mut inventory: ResMut<Inventory>,
    mut pending: ResMut<PendingTeleport>,
    mut vitals: ResMut<Vitals>,
    mut progression: ResMut<Progression>,
    mut save_io: SaveIo,
    mut scene: SceneState,
    mut players: Query<&mut Player>,
) {
    let hotkey_save = keys.just_pressed(KeyCode::F5) || save_io.save_request.0;
    let hotkey_load = keys.just_pressed(KeyCode::F9);
    let menu_load = save_io.load_request.0;
    // The hotkey/menu save is one-shot: cleared whether or not it runs, so a press
    // during a blocked frame is dropped rather than queued.
    save_io.save_request.0 = false;
    // A fade defers every save and load. A pending interpreter save is left set
    // (not consumed) so it retries once the fade ends.
    if fade.busy() {
        return;
    }
    let event_save = std::mem::take(&mut save_io.event_save.0);
    let gated = dialogue.active || running.active();
    match resolve(event_save, hotkey_save, menu_load, hotkey_load, gated) {
        Some(Action::Save) => {
            let Some(map_data) = map_data else {
                return;
            };
            let Ok(player) = players.single() else {
                return;
            };
            let (items, gold) = inventory.snapshot();
            let [tr, tg, tb, ts] = scene.tone.tone();
            let game = SaveGame {
                map_id: map_data.map_id,
                x: player.tile_x.max(0) as u32,
                y: player.tile_y.max(0) as u32,
                dir: player.dir,
                switches: switches.entries(),
                variables: variables.entries(),
                party: party.snapshot(),
                items,
                gold,
                progression: progression.entries(),
                vitals: vitals.entries(),
                hero_name: scene.hero_name.0.clone(),
                charset: player.charset.clone(),
                charset_index: player.index,
                tone: (
                    tr.round() as i32,
                    tg.round() as i32,
                    tb.round() as i32,
                    ts.round() as i32,
                ),
                weather: scene.weather.code(),
                weather_strength: scene.weather_strength.0,
                equipment: save_io.equipment.entries(),
                playtime: scene.playtime.seconds,
                timer_remaining: scene.game_clock.remaining,
                timer_running: scene.game_clock.running,
            };
            match write_save(&save_io.location.0, &game) {
                Ok(()) => info!("saved game to {}", save_io.location.0.display()),
                Err(e) => error!("save failed: {e}"),
            }
        }
        Some(Action::Load) => {
            // Consume the request whether or not the slot reads back, so a missing
            // or corrupt file can't wedge a waiting Continue on the title screen.
            save_io.load_request.0 = false;
            let Some(game) = read_save(&save_io.location.0) else {
                return;
            };
            switches.load(game.switches);
            variables.load(game.variables);
            party.restore(game.party);
            inventory.restore(game.items, game.gold);
            progression.load(game.progression);
            vitals.load(game.vitals);
            save_io.equipment.load(game.equipment);
            scene.playtime.seconds = game.playtime;
            scene.game_clock.remaining = game.timer_remaining;
            scene.game_clock.running = game.timer_running;
            // An empty name/charset is a pre-#46 slot that never stored them; leave
            // the boot default in place rather than blanking it.
            if !game.hero_name.is_empty() {
                scene.hero_name.0 = game.hero_name;
            }
            *scene.weather = Weather::from_code(game.weather);
            scene.weather_strength.0 = game.weather_strength;
            let (tr, tg, tb, ts) = game.tone;
            scene
                .tone
                .set_tone([tr as f32, tg as f32, tb as f32, ts as f32]);
            if let Ok(mut player) = players.single_mut() {
                player.dir = game.dir;
                if !game.charset.is_empty() {
                    player.charset = game.charset;
                    player.index = game.charset_index;
                }
            }
            // The teleport picks this up next, reloading the saved map at the saved
            // tile; `swap_map` reads the state we just restored above.
            pending.0 = Some((game.map_id, game.x, game.y));
            info!("loaded game from {}", save_io.location.0.display());
        }
        None => {}
    }
}

/// Serialise `game` to pretty RON and write it to `path`, creating the parent
/// save directory as needed.
fn write_save(path: &Path, game: &SaveGame) -> Result<(), String> {
    let ron =
        ron::ser::to_string_pretty(game, PrettyConfig::default()).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, ron).map_err(|e| e.to_string())
}

/// Read and deserialise the save slot at `path`, or `None` if it is absent; a
/// corrupt file is logged and treated as absent.
fn read_save(path: &Path) -> Option<SaveGame> {
    let text = std::fs::read_to_string(path).ok()?;
    match ron::from_str(&text) {
        Ok(game) => Some(game),
        Err(e) => {
            error!("load failed: parsing {}: {e}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests;

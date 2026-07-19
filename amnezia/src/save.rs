//! Single-slot save and load: `F5` snapshots the full game state (switches,
//! variables, party, inventory, plus the hero's map, tile, and facing) to a RON
//! file at a fixed, working-directory-independent path (see [`save_dir`]); `F9`
//! restores it. A load rebuilds the world by restoring the resources and then
//! reusing the teleport machinery to reload the saved map at the saved tile — the
//! fade's `swap_map` reads the freshly restored state when it respawns the map's
//! events.

use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::progression::Progression;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport};
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

/// Set once any save has been loaded this session (Continue or the `F9` dev load).
/// The start map (`start.ron`, map 5) carries an *unconditional* autostart intro
/// that renames the hero, builds the party, and teleports into the opening; it is
/// meant to run only on New Game. A Continue that resolved onto the start map — a
/// stale or degenerate slot, or any resume landing there — would otherwise let that
/// intro replay and hurl the player back to the beginning. `autorun` reads this to
/// suppress the start map's autostart. No map ever teleports back to the start map,
/// so this stays set harmlessly for the rest of the session once a load happens.
#[derive(Resource, Default)]
pub struct ResumedFromSave(pub bool);

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
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadRequest>()
            .init_resource::<SaveRequest>()
            .init_resource::<EventSaveRequest>()
            .init_resource::<SaveAccess>()
            .init_resource::<SaveLocation>()
            .init_resource::<ResumedFromSave>()
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
    resumed: ResMut<'w, ResumedFromSave>,
    location: Res<'w, SaveLocation>,
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
            };
            match write_save(&save_io.location.0, &game) {
                Ok(()) => info!("saved game to {}", save_io.location.0.display()),
                Err(e) => error!("save failed: {e}"),
            }
        }
        Some(Action::Load) => {
            // Consume the request and mark the session resumed whether or not the
            // slot reads back, so a missing or corrupt file can't wedge a waiting
            // Continue — and can't drop it back into the start map's intro.
            save_io.load_request.0 = false;
            save_io.resumed.0 = true;
            let Some(game) = read_save(&save_io.location.0) else {
                return;
            };
            switches.load(game.switches);
            variables.load(game.variables);
            party.restore(game.party);
            inventory.restore(game.items, game.gold);
            progression.load(game.progression);
            vitals.load(game.vitals);
            if let Ok(mut player) = players.single_mut() {
                player.dir = game.dir;
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
mod tests {
    use super::*;

    #[test]
    fn save_game_ron_round_trip() {
        let game = SaveGame {
            map_id: 5,
            x: 12,
            y: 7,
            dir: 2,
            switches: vec![(1, true), (3, false), (10, true)],
            variables: vec![(2, -4), (5, 100)],
            party: vec![1, 3],
            items: vec![(181, 2), (200, 1)],
            gold: 250,
            progression: vec![(1, 500), (3, 20)],
            vitals: vec![(1, (40, 12)), (3, (30, 0))],
        };
        let ron = ron::ser::to_string_pretty(&game, PrettyConfig::default()).unwrap();
        let decoded: SaveGame = ron::from_str(&ron).unwrap();
        assert_eq!(game, decoded);
    }

    /// A unique temp slot path per test, so file-touching tests never race on a
    /// shared file (cargo runs them in parallel) and never touch the real save.
    fn temp_slot(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("amnezia_{tag}_{}.ron", std::process::id()))
    }

    /// A minimal but complete resource set for driving [`save_or_load`] headlessly,
    /// with the save slot pointed at `location` so the real developer save is never
    /// read or written.
    fn save_app(location: PathBuf) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Dialogue>()
            .init_resource::<Fade>()
            .init_resource::<PendingTeleport>()
            .init_resource::<Switches>()
            .init_resource::<Variables>()
            .init_resource::<Party>()
            .init_resource::<Inventory>()
            .init_resource::<LoadRequest>()
            .init_resource::<SaveRequest>()
            .init_resource::<EventSaveRequest>()
            .init_resource::<Vitals>()
            .init_resource::<Progression>()
            .init_resource::<ResumedFromSave>();
        app.insert_resource(SaveLocation(location));
        app.add_systems(Update, save_or_load);
        app
    }

    #[test]
    fn slot_exists_tracks_the_file() {
        let path = temp_slot("exists");
        let _ = std::fs::remove_file(&path);
        assert!(!slot_exists(&path));
        std::fs::write(&path, "x").unwrap();
        assert!(slot_exists(&path));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn resolved_save_path_is_absolute_and_cwd_independent() {
        // The bug's core: the save path must not depend on the working directory,
        // so a save written on one launch is found on the next wherever the game
        // (or the packaged `.app`) was started from.
        assert!(
            save_path().is_absolute(),
            "the save path must be absolute so it is independent of the CWD"
        );
        assert!(save_dir().is_absolute());
    }

    #[test]
    fn resolve_lets_event_save_and_menu_load_bypass_the_running_gate() {
        // Gated (a dialogue or a still-running event): the interpreter save (opcode
        // 11910) and the title's Continue (menu_load) both still fire, but the F5 /
        // menu save and the F9 dev load are held back.
        assert_eq!(resolve(true, false, false, false, true), Some(Action::Save));
        assert_eq!(resolve(false, true, false, false, true), None);
        assert_eq!(resolve(false, false, true, false, true), Some(Action::Load));
        assert_eq!(resolve(false, false, false, true, true), None);
        // Ungated: every request fires; a save wins a tie against a load.
        assert_eq!(
            resolve(false, true, false, false, false),
            Some(Action::Save)
        );
        assert_eq!(
            resolve(false, false, false, true, false),
            Some(Action::Load)
        );
        assert_eq!(
            resolve(false, false, true, false, false),
            Some(Action::Load)
        );
        assert_eq!(resolve(false, true, false, true, false), Some(Action::Save));
        assert_eq!(resolve(false, false, false, false, false), None);
    }

    #[test]
    fn open_save_menu_saves_while_its_event_is_running() {
        let path = temp_slot("eventsave");
        let _ = std::fs::remove_file(&path);
        let mut app = save_app(path.clone());
        app.insert_resource(MapData::for_test(20, 15));
        // An event is mid-run when the save is requested (as `OpenSaveMenu` is).
        let mut running = RunningEvent::default();
        running.start(1, Vec::new());
        assert!(running.active());
        app.insert_resource(running);
        app.world_mut().spawn(Player {
            tile_x: 3,
            tile_y: 4,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        });
        // Stand in for the interpreter's OpenSaveMenu (opcode 11910) arm.
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();

        let saved = path.exists();
        let consumed = !app.world().resource::<EventSaveRequest>().0;
        let _ = std::fs::remove_file(&path);
        assert!(
            saved,
            "an OpenSaveMenu save must be written even while its event runs"
        );
        assert!(
            consumed,
            "the event-save request must be consumed once saved"
        );
    }

    #[test]
    fn continue_load_targets_the_saved_map_even_when_an_autostart_is_pending() {
        // A valid save at the map-2 save crystal (16, 6), with a switch set so the
        // restore is observable.
        let path = temp_slot("continue");
        let game = SaveGame {
            map_id: 2,
            x: 16,
            y: 6,
            dir: 4,
            switches: vec![(8, true)],
            variables: vec![(3, 42)],
            party: vec![1, 3],
            items: vec![(181, 2)],
            gold: 250,
            progression: vec![(1, 500)],
            vitals: vec![(1, (40, 12))],
        };
        write_save(&path, &game).unwrap();

        let mut app = save_app(path.clone());
        // The boot intro autostart is mid-run: `gated` would refuse a plain load.
        let mut running = RunningEvent::default();
        running.start(1, Vec::new());
        assert!(running.active());
        app.insert_resource(running);
        app.world_mut().spawn(Player {
            tile_x: 0,
            tile_y: 0,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        });
        // The title's Continue sets this; it must load despite the running event.
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();

        let world = app.world();
        assert_eq!(
            world.resource::<PendingTeleport>().0,
            Some((2, 16, 6)),
            "Continue must teleport to the SAVED map/tile, not the start map"
        );
        assert!(
            world.resource::<Switches>().get(8),
            "restored switches must be applied"
        );
        assert_eq!(world.resource::<Variables>().get(3), 42);
        assert_eq!(world.resource::<Party>().snapshot(), vec![1, 3]);
        assert!(
            world.resource::<ResumedFromSave>().0,
            "a load must mark the session resumed so the boot intro is suppressed"
        );
        assert!(
            !world.resource::<LoadRequest>().0,
            "the Continue request must be consumed"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_round_trips_to_the_resolved_path_and_is_found_after_restart() {
        let path = temp_slot("roundtrip");
        let _ = std::fs::remove_file(&path);
        let game = SaveGame {
            map_id: 2,
            x: 16,
            y: 6,
            dir: 2,
            switches: vec![(8, true)],
            variables: vec![],
            party: vec![1],
            items: vec![],
            gold: 0,
            progression: vec![],
            vitals: vec![],
        };
        write_save(&path, &game).unwrap();
        // "Restart": a fresh read at the same resolved path finds and decodes it.
        assert!(
            slot_exists(&path),
            "the written slot must be found on re-read"
        );
        assert_eq!(read_save(&path), Some(game));
        let _ = std::fs::remove_file(&path);
    }
}

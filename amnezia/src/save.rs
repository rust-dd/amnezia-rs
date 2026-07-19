//! Single-slot save and load: `F5` snapshots the full game state (switches,
//! variables, party, inventory, plus the hero's map, tile, and facing) to a RON
//! file next to the working directory; `F9` restores it. A load rebuilds the
//! world by restoring the resources and then reusing the teleport machinery to
//! reload the saved map at the saved tile — the fade's `swap_map` reads the
//! freshly restored state when it respawns the map's events.

use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::progression::Progression;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport};
use crate::vitals::Vitals;
use crate::world::MapData;
use bevy::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Directory (relative to the working directory) holding save slots.
const SAVE_DIR: &str = "saves";
/// The single v1 save slot.
const SAVE_PATH: &str = "saves/slot1.ron";

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

/// Whether the single save slot exists on disk, for the title's Continue gate.
pub fn save_slot_exists() -> bool {
    slot_exists(SAVE_PATH)
}

/// Whether `path` names an existing file. Split out so the gate is testable
/// without depending on the fixed [`SAVE_PATH`].
fn slot_exists(path: &str) -> bool {
    Path::new(path).exists()
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
            .add_systems(Update, save_or_load);
    }
}

/// What [`save_or_load`] does this frame once a fade has been ruled out.
#[derive(PartialEq, Eq, Debug)]
enum Action {
    Save,
    Load,
}

/// Decide the frame's action. A dialogue or a still-running event (`gated`) holds
/// back the hotkey/menu save (`hotkey_save`) and any load, but an
/// interpreter-originated save (`event_save`, opcode 11910) bypasses that gate —
/// it fires while its own event is deliberately still running. A save takes
/// precedence over a load requested in the same frame.
fn resolve(event_save: bool, hotkey_save: bool, load: bool, gated: bool) -> Option<Action> {
    if event_save || (hotkey_save && !gated) {
        Some(Action::Save)
    } else if load && !gated {
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
    mut load_request: ResMut<LoadRequest>,
    mut save_request: ResMut<SaveRequest>,
    mut event_save: ResMut<EventSaveRequest>,
    mut vitals: ResMut<Vitals>,
    mut progression: ResMut<Progression>,
    mut players: Query<&mut Player>,
) {
    let hotkey_save = keys.just_pressed(KeyCode::F5) || save_request.0;
    let load = keys.just_pressed(KeyCode::F9) || load_request.0;
    // The hotkey/menu save is one-shot: cleared whether or not it runs, so a press
    // during a blocked frame is dropped rather than queued.
    save_request.0 = false;
    // A fade defers every save and load. A pending interpreter save is left set
    // (not consumed) so it retries once the fade ends.
    if fade.busy() {
        return;
    }
    let event_save = std::mem::take(&mut event_save.0);
    let gated = dialogue.active || running.active();
    match resolve(event_save, hotkey_save, load, gated) {
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
            match write_save(&game) {
                Ok(()) => info!("saved game to {SAVE_PATH}"),
                Err(e) => error!("save failed: {e}"),
            }
        }
        Some(Action::Load) => {
            // Consume the request whether or not the slot reads back, so a missing
            // or corrupt file can't wedge a waiting Continue.
            load_request.0 = false;
            let Some(game) = read_save() else {
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
            info!("loaded game from {SAVE_PATH}");
        }
        None => {}
    }
}

/// Serialise `game` to pretty RON and write it to the save slot, creating the
/// save directory as needed.
fn write_save(game: &SaveGame) -> Result<(), String> {
    let ron =
        ron::ser::to_string_pretty(game, PrettyConfig::default()).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(SAVE_DIR).map_err(|e| e.to_string())?;
    std::fs::write(SAVE_PATH, ron).map_err(|e| e.to_string())
}

/// Read and deserialise the save slot, or `None` if it is absent; a corrupt
/// file is logged and treated as absent.
fn read_save() -> Option<SaveGame> {
    let text = std::fs::read_to_string(SAVE_PATH).ok()?;
    match ron::from_str(&text) {
        Ok(game) => Some(game),
        Err(e) => {
            error!("load failed: parsing {SAVE_PATH}: {e}");
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

    #[test]
    fn slot_exists_tracks_the_file() {
        let path = std::env::temp_dir().join(format!("amnezia_slot_{}.ron", std::process::id()));
        let path = path.to_str().unwrap();
        let _ = std::fs::remove_file(path);
        assert!(!slot_exists(path));
        std::fs::write(path, "x").unwrap();
        assert!(slot_exists(path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn resolve_lets_an_event_save_bypass_the_running_gate() {
        // Gated (a dialogue or a still-running event): an interpreter save (opcode
        // 11910) still saves, but the F5 / menu save and any load are held back.
        assert_eq!(resolve(true, false, false, true), Some(Action::Save));
        assert_eq!(resolve(false, true, false, true), None);
        assert_eq!(resolve(false, false, true, true), None);
        // Ungated: the hotkey save and load work as before, a save winning a tie.
        assert_eq!(resolve(false, true, false, false), Some(Action::Save));
        assert_eq!(resolve(false, false, true, false), Some(Action::Load));
        assert_eq!(resolve(false, true, true, false), Some(Action::Save));
        assert_eq!(resolve(false, false, false, false), None);
    }

    #[test]
    fn open_save_menu_saves_while_its_event_is_running() {
        // Protect any real slot: back it up, then restore it before asserting so a
        // failure can never clobber a developer's save.
        let backup = std::fs::read_to_string(SAVE_PATH).ok();
        let _ = std::fs::remove_file(SAVE_PATH);

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
            .init_resource::<Progression>();
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
        app.add_systems(Update, save_or_load);
        app.update();

        let saved = Path::new(SAVE_PATH).exists();
        let consumed = !app.world().resource::<EventSaveRequest>().0;
        match &backup {
            Some(contents) => std::fs::write(SAVE_PATH, contents).unwrap(),
            None => {
                let _ = std::fs::remove_file(SAVE_PATH);
                let _ = std::fs::remove_dir(SAVE_DIR);
            }
        }
        assert!(
            saved,
            "an OpenSaveMenu save must be written even while its event runs"
        );
        assert!(
            consumed,
            "the event-save request must be consumed once saved"
        );
    }
}

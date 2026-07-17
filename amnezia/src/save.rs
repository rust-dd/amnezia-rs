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
/// frame exactly as if `F9` had been pressed. The title screen's "Folytatás" sets
/// it so a resume reuses the same restore path without duplicating the load body.
#[derive(Resource, Default)]
pub struct LoadRequest(pub bool);

/// A request to save, honoured by [`save_or_load`] as if `F5` had been pressed.
/// The in-game menu's Save action sets it, reusing the same snapshot path.
#[derive(Resource, Default)]
pub struct SaveRequest(pub bool);

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
            .add_systems(Update, save_or_load);
    }
}

/// Handle the save (`F5`) and load (`F9`) hotkeys. Both are suppressed while a
/// dialogue, fade, or event script is in progress so a snapshot is never taken
/// or applied mid-transition (mirrors the movement guard).
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
    mut vitals: ResMut<Vitals>,
    mut progression: ResMut<Progression>,
    mut players: Query<&mut Player>,
) {
    let save = keys.just_pressed(KeyCode::F5) || save_request.0;
    let load = keys.just_pressed(KeyCode::F9) || load_request.0;
    save_request.0 = false;
    if (!save && !load) || dialogue.active || fade.busy() || running.active() {
        return;
    }
    if save {
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
    } else {
        // Consume the request whether or not the slot reads back, so a missing or
        // corrupt file can't wedge a waiting Continue.
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
}

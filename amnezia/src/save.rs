//! Save crystals and title-screen loading share a persistent, single-slot snapshot.

mod storage;
#[cfg(test)]
use storage::save_dir;
use storage::{read_save, save_path, slot_exists, write_save};

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
#[cfg(test)]
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A request to load the save slot, honoured by [`save_or_load`] on the next
/// frame exactly as if `F9` had been pressed. The title screen's "Betöltés"
/// (Continue) sets it so a resume reuses the same restore path without duplicating
/// the load body.
#[derive(Resource, Default)]
pub struct LoadRequest(pub bool);

#[derive(Resource, Default)]
pub struct LoadOutcome(pub Option<bool>);

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
/// 11930). Amnézia is a crystal-save game: it forbids manual saves everywhere and
/// enables them only for the instant a save crystal runs `OpenSaveMenu` (the 16
/// crystals `enable`, 35 sites `disable`), so this starts **disabled** — unlike
/// RPG_RT's enabled default — and the player can never save from the menu. Only the
/// menu's Save entry (and the `Esc`-`S` quick save) honour it; the save crystal
/// ([`EventSaveRequest`], `OpenSaveMenu`) and the `F5` dev hotkey are deliberately
/// ungated, matching RPG_RT's `SetAllowSave`.
#[derive(Resource, Default)]
pub struct SaveAccess(pub bool);

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
    slot_exists(&save_path()) || storage::legacy_save_path().is_some_and(|p| slot_exists(&p))
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
    learned_skills: Vec<(u32, Vec<u32>)>,
    #[serde(default)]
    vitals: Vec<(u32, (i32, i32))>,
    #[serde(default)]
    conditions: Vec<(u32, Vec<u32>)>,
    #[serde(default)]
    field_steps: u64,
    #[serde(default)]
    hero_name: String,
    #[serde(default)]
    charset: String,
    #[serde(default)]
    charset_index: u32,
    #[serde(default)]
    hero_hidden: bool,
    #[serde(default = "neutral_tone")]
    tone: (i32, i32, i32, i32),
    #[serde(default)]
    weather: i32,
    #[serde(default)]
    weather_strength: i32,
    #[serde(default)]
    equipment: Vec<(u32, [u32; 5])>,
    #[serde(default)]
    playtime: u64,
    #[serde(default)]
    timer_remaining: f32,
    #[serde(default)]
    timer_running: bool,
    #[serde(default)]
    vehicles: crate::vehicles::VehicleSave,
    #[serde(default)]
    system_bgm: crate::system_bgm::SystemBgm,
    #[serde(default)]
    panorama: Option<crate::panorama::Panorama>,
    #[serde(default)]
    appearance: crate::appearance::Appearance,
    #[serde(default)]
    menu_access: Option<bool>,
    #[serde(default)]
    save_access: bool,
}

/// The RM2000 neutral screen tone (every channel 100), the [`SaveGame::tone`]
/// default so a slot saved before the tone was persisted loads without tinting
/// the screen — a plain `(0, 0, 0, 0)` default would black it out.
fn neutral_tone() -> (i32, i32, i32, i32) {
    (100, 100, 100, 100)
}

pub struct SavePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SaveSet;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadRequest>()
            .init_resource::<LoadOutcome>()
            .init_resource::<SaveRequest>()
            .init_resource::<EventSaveRequest>()
            .init_resource::<SaveAccess>()
            .init_resource::<SaveLocation>()
            .add_systems(
                PreUpdate,
                save_or_load
                    .in_set(SaveSet)
                    .after(bevy::input::InputSystems),
            );
    }
}

/// The save/load request flags, the resumed marker, and the resolved slot path,
/// bundled into one `SystemParam` so [`save_or_load`] stays within Bevy's
/// 16-parameter cap.
#[derive(SystemParam)]
struct SaveIo<'w, 's> {
    commands: Commands<'w, 's>,
    outcome: ResMut<'w, LoadOutcome>,
    load_request: ResMut<'w, LoadRequest>,
    save_request: ResMut<'w, SaveRequest>,
    event_save: ResMut<'w, EventSaveRequest>,
    location: Res<'w, SaveLocation>,
    // The runtime equipment store, snapshotted on save and restored on load
    // beside the party and inventory. Bundled here so [`save_or_load`] stays
    // within Bevy's 16-parameter cap.
    equipment: ResMut<'w, Equipment>,
    battle: Option<Res<'w, crate::battle::BattleActive>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    shop: Option<Res<'w, crate::shop::ShopOpen>>,
}

/// The scene resources a save now also snapshots and restores beyond the core
/// state: the hero's name ([`HeroName`]), the active weather ([`Weather`] and its
/// [`WeatherStrength`]), and the screen tone ([`TintState`]). Bundled into one
/// `SystemParam` so [`save_or_load`] stays within Bevy's 16-parameter cap.
#[derive(SystemParam)]
struct SceneState<'w> {
    appearance: Option<ResMut<'w, crate::appearance::Appearance>>,
    menu_access: Option<ResMut<'w, crate::menu::MenuAccess>>,
    save_access: Option<ResMut<'w, SaveAccess>>,
    panorama: Option<ResMut<'w, crate::panorama::Panorama>>,
    hero_hidden: Option<ResMut<'w, crate::player::HeroHidden>>,
    field_steps: Option<ResMut<'w, crate::conditions::FieldSteps>>,
    vehicles: Option<ResMut<'w, crate::vehicles::Vehicles>>,
    system_bgm: Option<ResMut<'w, crate::system_bgm::SystemBgm>>,
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
    let hotkey_save =
        (crate::debug::tools_enabled() && keys.just_pressed(KeyCode::F5)) || save_io.save_request.0;
    let hotkey_load = crate::debug::tools_enabled() && keys.just_pressed(KeyCode::F9);
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
    let gated = dialogue.active
        || running.active()
        || save_io.battle.as_ref().is_some_and(|s| s.0)
        || save_io.title.as_ref().is_some_and(|s| s.0)
        || save_io.gameover.as_ref().is_some_and(|s| s.0)
        || save_io.shop.as_ref().is_some_and(|s| s.0);
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
                learned_skills: progression.skill_entries(),
                vitals: vitals.entries(),
                conditions: vitals.condition_entries(),
                field_steps: scene.field_steps.as_ref().map_or(0, |s| s.count),
                hero_name: scene.hero_name.0.clone(),
                charset: player.charset.clone(),
                charset_index: player.index,
                hero_hidden: scene.hero_hidden.as_ref().is_some_and(|h| h.0),
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
                vehicles: scene
                    .vehicles
                    .as_ref()
                    .map_or_else(Default::default, |v| v.save.clone()),
                system_bgm: scene.system_bgm.as_deref().cloned().unwrap_or_default(),
                panorama: scene.panorama.as_deref().cloned(),
                appearance: scene.appearance.as_deref().cloned().unwrap_or_default(),
                menu_access: scene.menu_access.as_ref().map(|v| v.0),
                save_access: scene.save_access.as_ref().is_some_and(|v| v.0),
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
            save_io.outcome.0 = Some(false);
            let Some(game) = read_save(&save_io.location.0) else {
                return;
            };
            if !valid_destination(&game) {
                error!(
                    "load failed: invalid map or party in {}",
                    save_io.location.0.display()
                );
                return;
            }
            save_io.commands.queue(crate::session::clear_transient);
            switches.load(game.switches);
            variables.load(game.variables);
            party.restore(game.party);
            inventory.restore(game.items, game.gold);
            progression.load(game.progression);
            progression.load_skills(game.learned_skills);
            vitals.load(game.vitals);
            vitals.load_conditions(game.conditions);
            if let Some(steps) = scene.field_steps.as_deref_mut() {
                *steps = default();
                steps.count = game.field_steps;
            }
            save_io.equipment.load(game.equipment);
            scene.playtime.seconds = game.playtime;
            scene.game_clock.remaining = game.timer_remaining;
            scene.game_clock.running = game.timer_running;
            if let Some(hidden) = scene.hero_hidden.as_mut() {
                hidden.0 = game.hero_hidden;
            }
            if let Some(vehicles) = scene.vehicles.as_mut() {
                vehicles.restore(game.vehicles);
            }
            if let Some(music) = scene.system_bgm.as_deref_mut() {
                *music = game.system_bgm;
            }
            if let Some(panorama) = scene.panorama.as_deref_mut() {
                *panorama = game.panorama.unwrap_or_default();
            }
            if let Some(appearance) = scene.appearance.as_deref_mut() {
                *appearance = game.appearance;
                if let Some(actor) = party.snapshot().first()
                    && appearance.get(*actor).is_none()
                    && !game.charset.is_empty()
                {
                    appearance.set(*actor, game.charset.clone(), game.charset_index);
                }
            }
            if let Some(access) = scene.menu_access.as_mut() {
                access.0 = game.menu_access.unwrap_or(true);
            }
            if let Some(access) = scene.save_access.as_mut() {
                access.0 = game.save_access;
            }
            // Older saves omit these fields, so keep the initial graphic and name.
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
            pending.reload(game.map_id, game.x, game.y);
            save_io.outcome.0 = Some(true);
            info!("loaded game from {}", save_io.location.0.display());
        }
        None => {}
    }
}

fn valid_destination(game: &SaveGame) -> bool {
    let path = format!(
        "{}/maps/map_{:04}.ron",
        crate::assets::asset_root(),
        game.map_id
    );
    let map = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| ron::from_str::<amnezia_data::Map>(&text).ok());
    !game.party.is_empty()
        && game.timer_remaining.is_finite()
        && map.is_some_and(|map| game.x < map.width && game.y < map.height)
}

#[cfg(test)]
mod tests;

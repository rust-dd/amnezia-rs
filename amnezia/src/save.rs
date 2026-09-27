//! Save crystals and title-screen loading share versioned persistent snapshots.

pub(crate) mod animation_smoke;
pub(crate) mod camera_smoke;
pub(crate) mod hero_smoke;
mod identities;
pub(crate) mod music_smoke;
pub(crate) mod npc_smoke;
mod numeric;
pub(crate) mod picture_smoke;
pub(crate) mod preview;
pub(crate) mod screen_smoke;
pub(crate) mod slots;
pub(crate) mod smoke_slot;
mod snapshot;
mod storage;
pub(crate) mod vehicle_smoke;
pub(crate) mod weather_smoke;
use snapshot::SaveGame;
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
use std::path::PathBuf;

pub(crate) const SAVE_FORMAT_VERSION: u32 = 24;

/// A request to load the save slot, honoured by [`save_or_load`] on the next
/// frame exactly as if `F9` had been pressed. The title screen's "Betöltés"
/// (Continue) sets it so a resume reuses the same restore path without duplicating
/// the load body.
#[derive(Resource, Default)]
pub struct LoadRequest(pub bool);

#[derive(Resource, Default)]
pub struct LoadOutcome(pub Option<bool>);

/// A request to save, honoured by [`save_or_load`] as if `F5` had been pressed.
/// Confirming a slot in the in-game menu sets it, reusing the same snapshot path.
#[derive(Resource, Default)]
pub struct SaveRequest(pub bool);

/// An event-requested save scene (opcode 11910, `OpenSaveMenu`). The selector
/// keeps it set while choosing, cancels it without writing, or lets the save
/// system consume it after approval while the foreground event is still paused.
#[derive(Resource, Default)]
pub struct EventSaveRequest(pub bool);

/// Whether the in-menu Save command is allowed (RM2000 `ChangeSaveAccess`, opcode
/// 11930). Starts disabled until the original events set the menu permission.
/// The menu's Save entry and shortcut honour it; save crystals invoke
/// [`EventSaveRequest`] independently, without temporarily enabling menu saves.
/// The development F5 hotkey also bypasses this permission.
#[derive(Resource, Default)]
pub struct SaveAccess(pub bool);

/// The first slot's path and the directory containing subsequent slots.
/// Tests can retain custom first-slot filenames; the real game uses the
/// working-directory-independent [`save_path`].
#[derive(Resource)]
pub struct SaveLocation(pub PathBuf);

impl Default for SaveLocation {
    fn default() -> Self {
        Self(save_path())
    }
}

impl SaveLocation {
    pub(crate) fn has_saves(&self) -> bool {
        (1..=slots::COUNT).any(|number| {
            let path = slots::ActiveSlot::new(number).unwrap().path(&self.0);
            slot_exists(&storage::source_path(&path))
        })
    }
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
        crate::audio::saved::register(app);
        crate::player::saved_camera::register(app);
        crate::screenfx::saved::register(app);
        crate::world::saved::register(app);
        crate::vehicles::saved::register(app);
        app.init_resource::<LoadRequest>()
            .init_resource::<LoadOutcome>()
            .init_resource::<SaveRequest>()
            .init_resource::<EventSaveRequest>()
            .init_resource::<SaveAccess>()
            .init_resource::<SaveLocation>()
            .init_resource::<slots::ActiveSlot>();
        crate::timing::logical::pre(app, || {
            save_or_load
                .in_set(SaveSet)
                .after(bevy::input::InputSystems)
        });
    }
}

/// The save/load request flags, the resumed marker, and the resolved slot path,
/// bundled into one `SystemParam` so [`save_or_load`] stays within Bevy's
/// 16-parameter cap.
#[derive(SystemParam)]
struct SaveIo<'w, 's> {
    files: Option<Res<'w, crate::menu::save_files::SaveFiles>>,
    data: Option<Res<'w, crate::gamedata::GameData>>,
    transition: Option<Res<'w, crate::transitions::Transition>>,
    commands: Commands<'w, 's>,
    outcome: ResMut<'w, LoadOutcome>,
    load_request: ResMut<'w, LoadRequest>,
    save_request: ResMut<'w, SaveRequest>,
    event_save: ResMut<'w, EventSaveRequest>,
    location: Res<'w, SaveLocation>,
    slot: Option<Res<'w, slots::ActiveSlot>>,
    equipment: ResMut<'w, Equipment>,
    battle: Option<Res<'w, crate::battle::BattleActive>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    shop: Option<Res<'w, crate::shop::ShopOpen>>,
}

impl SaveIo<'_, '_> {
    fn path(&self) -> PathBuf {
        self.slot
            .as_deref()
            .copied()
            .unwrap_or_default()
            .path(&self.location.0)
    }
}

/// Scene resources grouped to keep [`save_or_load`] within Bevy's parameter limit.
#[derive(SystemParam)]
struct SceneState<'w, 's> {
    characters: crate::world::saved::Capture<'w, 's>,
    animation: crate::animation::saved::Capture<'w, 's>,
    screen: crate::screenfx::saved::Capture<'w>,
    pictures: crate::picture::saved::Capture<'w, 's>,
    camera: Option<Res<'w, crate::player::CameraPan>>,
    message: crate::dialogue::saved::Capture<'w>,
    music: crate::audio::saved::Capture<'w>,
    transitions: Option<ResMut<'w, crate::transitions::Settings>>,
    game_frames: Option<ResMut<'w, crate::timing::GameFrames>>,
    scene_frames: Option<ResMut<'w, crate::timing::SceneFrames>>,
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
    if fade.busy() || save_io.transition.as_ref().is_some_and(|t| t.busy()) {
        return;
    }
    if save_io
        .files
        .as_ref()
        .is_some_and(|files| files.blocks_io())
    {
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
            let mut game = SaveGame {
                saved_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|elapsed| elapsed.as_secs()),
                foreground: running.snapshot(),
                vehicle_motion: scene
                    .vehicles
                    .as_ref()
                    .map(|vehicles| vehicles.motion_snapshot()),
                hero_motion: scene.characters.hero(player),
                map_events: scene.characters.snapshot(),
                map_animation: scene.animation.snapshot(),
                screen: Some(scene.screen.snapshot(&scene.tone)),
                pictures: scene.pictures.snapshot(),
                camera: scene.camera.as_ref().map(|camera| camera.snapshot()),
                message: scene.message.snapshot(&dialogue),
                music: scene.music.snapshot(),
                format_version: SAVE_FORMAT_VERSION,
                game_frames: scene.game_frames.as_deref().copied().unwrap_or_default(),
                scene_frame: Some(scene.scene_frames.as_ref().map_or(0, |clock| clock.frame)),
                transitions: scene.transitions.as_deref().cloned().unwrap_or_default(),
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
                timer_visible: scene.game_clock.visible,
                timer_in_battle: scene.game_clock.in_battle,
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
            if !identities::prepare(&mut game, save_io.data.as_deref())
                || !numeric::prepare(&mut game, save_io.data.as_deref())
            {
                error!("save failed: invalid gameplay data");
                return;
            }
            let path = save_io.path();
            match write_save(&path, &game) {
                Ok(()) => info!("saved game to {}", path.display()),
                Err(e) => error!("save failed: {e}"),
            }
        }
        Some(Action::Load) => {
            // Consume the request whether or not the slot reads back, so a missing
            // or corrupt file can't wedge a waiting Continue on the title screen.
            save_io.load_request.0 = false;
            save_io.outcome.0 = Some(false);
            let path = save_io.path();
            let Some(mut game) = read_save(&path) else {
                return;
            };
            if !valid_destination(&game, &scene.animation)
                || !identities::prepare(&mut game, save_io.data.as_deref())
                || !numeric::prepare(&mut game, save_io.data.as_deref())
            {
                error!(
                    "load failed: unsupported format or invalid saved state in {}",
                    path.display()
                );
                return;
            }
            save_io.commands.queue(crate::session::clear_for_reload);
            let map_id = game.map_id;
            save_io.commands.queue(move |world: &mut World| {
                crate::interpreter::saved::restore(world, game.foreground);
                crate::vehicles::saved::prepare(world, map_id, game.vehicle_motion);
                crate::world::saved::hero::prepare(world, map_id, game.hero_motion);
                crate::world::saved::prepare(world, map_id, game.map_events);
                crate::animation::saved::prepare(world, map_id, game.map_animation);
                crate::screenfx::saved::prepare(world, map_id, game.screen);
                crate::picture::saved::prepare(world, map_id, game.pictures);
                game.message.restore(world);
                crate::player::saved_camera::prepare(world, map_id, game.camera);
                crate::audio::saved::prepare(world, map_id, game.music);
            });
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
            scene.playtime.restore(game.playtime);
            if let Some(frames) = scene.game_frames.as_deref_mut() {
                *frames = game.game_frames;
                frames.sanitize();
            }
            if let Some(frames) = scene.scene_frames.as_deref_mut() {
                frames.frame = game.scene_frame.unwrap_or(game.game_frames.frame);
            }
            if let Some(transitions) = scene.transitions.as_deref_mut() {
                *transitions = game.transitions;
            }
            scene.game_clock.remaining = game.timer_remaining;
            scene.game_clock.running = game.timer_running;
            scene.game_clock.visible = game.timer_visible;
            scene.game_clock.in_battle = game.timer_in_battle;
            scene.game_clock.expired = false;
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
                    && game.format_version == 0
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
            if game.format_version > 0 || !game.hero_name.is_empty() {
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
            pending.reload(game.map_id, game.x, game.y);
            save_io.outcome.0 = Some(true);
            info!("loaded game from {}", path.display());
        }
        None => {}
    }
}

fn valid_destination(game: &SaveGame, animation: &crate::animation::saved::Capture) -> bool {
    if game.format_version > SAVE_FORMAT_VERSION {
        return false;
    }
    let path = format!(
        "{}/maps/map_{:04}.ron",
        crate::assets::asset_root(),
        game.map_id
    );
    let map = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| ron::from_str::<amnezia_data::Map>(&text).ok());
    !game.party.is_empty()
        && game
            .foreground
            .as_ref()
            .is_none_or(crate::interpreter::saved::State::valid)
        && game
            .vehicle_motion
            .as_ref()
            .is_none_or(|state| state.valid(&game.vehicles))
        && game
            .hero_motion
            .as_ref()
            .is_none_or(|state| game.dir < 4 && state.valid())
        && game.screen.as_ref().is_none_or(|screen| screen.valid())
        && crate::picture::saved::valid(&game.pictures)
        && game
            .camera
            .as_ref()
            .is_none_or(crate::player::saved_camera::CameraState::valid)
        && game
            .music
            .as_ref()
            .is_none_or(crate::audio::saved::MusicState::valid)
        && game.timer_remaining.is_finite()
        && map.is_some_and(|map| {
            game.x < map.width
                && game.y < map.height
                && animation.valid(&game.map_animation, &map)
                && crate::world::saved::valid(&game.map_events, &map)
                && game
                    .foreground
                    .as_ref()
                    .is_none_or(|state| state.valid_map(game.map_id, &map))
        })
}

#[cfg(test)]
pub(crate) mod tests;

//! The interpreter's Bevy `SystemParam` bundles. `run_interpreter` sits at Bevy's
//! 16-parameter cap, so its resources, writers, and queries are grouped into a
//! handful of nested `SystemParam` structs here — keeping the system signature
//! small and the bundle definitions out of the dispatch loop.

use super::event_rng::EventRng;
use crate::animation::{AnimationLibrary, ShowMapAnimation};
use crate::appearance::SpriteChange;
use crate::battle::{BattleActive, BattleRequest, BattleResult};
use crate::dialogue::{MessagePosition, MessageTransparent};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::gameover::GameOverActive;
use crate::inputnumber::InputNumber;
use crate::menu::{MenuAccess, MenuOpen};
use crate::picture::PictureCommand;
use crate::player::{CameraPan, HeroHidden, Player};
use crate::progression::Progression;
use crate::save::{EventSaveRequest, SaveAccess};
use crate::screenfx::{ScreenEffect, Weather, WeatherStrength};
use crate::shop::{ShopOpen, ShopOutcome, ShopRequest};
use crate::text::HeroName;
use crate::timer::GameClock;
use crate::title::TitleActive;
use crate::vitals::Vitals;
use crate::world::{MapData, MapEvents, RelocateEvent};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// The overlays that pause the running event (menu, shop, battle). Bundled into
/// one `SystemParam` so `run_interpreter` stays within Bevy's 16-parameter cap.
/// The title screen is checked separately through [`FlowCtx`], which holds it
/// mutably for the `ReturnToTitle` opcode.
#[derive(SystemParam)]
pub(super) struct Blockers<'w> {
    menu: Res<'w, MenuOpen>,
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
}

impl Blockers<'_> {
    pub(super) fn any(&self) -> bool {
        self.menu.0 || self.shop.0 || self.battle.0
    }
}

/// The small map-effect resources the interpreter writes: camera pan, hero
/// transparency, weather, the message-box position, and the game clock. Bundled
/// into one nested `SystemParam` so [`SubsystemIo`] keeps within Bevy's
/// 16-parameter cap once the access/actor/flow bundles below are added.
#[derive(SystemParam)]
pub(super) struct MapFx<'w> {
    pub(super) transitions: crate::transitions::TransitionIo<'w>,
    pub(super) message_options: ResMut<'w, crate::dialogue::MessageOptions>,
    pub(super) camera_pan: ResMut<'w, CameraPan>,
    pub(super) hero_hidden: ResMut<'w, HeroHidden>,
    pub(super) weather: ResMut<'w, Weather>,
    pub(super) weather_strength: ResMut<'w, WeatherStrength>,
    pub(super) message_position: ResMut<'w, MessagePosition>,
    pub(super) message_transparent: ResMut<'w, MessageTransparent>,
    pub(super) game_clock: ResMut<'w, GameClock>,
    pub(super) vehicles: ResMut<'w, crate::vehicles::Vehicles>,
    pub(super) current_bgm: Option<Res<'w, crate::audio::CurrentBgm>>,
    pub(super) system_bgm: ResMut<'w, crate::system_bgm::SystemBgm>,
    pub(super) panorama: ResMut<'w, crate::panorama::Panorama>,
}

/// The player-access toggles the interpreter flips: whether the in-menu Save
/// command works ([`SaveAccess`], opcode 11930) and whether the main menu can be
/// opened ([`MenuAccess`], opcode 11960). Bundled to stay within the cap.
#[derive(SystemParam)]
pub(super) struct AccessFlags<'w> {
    pub(super) save_access: ResMut<'w, SaveAccess>,
    pub(super) menu_access: ResMut<'w, MenuAccess>,
}

/// The interpreter's actor-state edits: the hero's name ([`HeroName`], opcode
/// 10610 — also read for the `\N[k]` message code) and the experience store that
/// backs `ChangeLevel` (opcode 10420), with the actor database it needs. Bundled
/// to stay within the cap.
#[derive(SystemParam)]
pub(super) struct ActorEdits<'w> {
    pub(super) hero_name: ResMut<'w, HeroName>,
    pub(super) progression: ResMut<'w, Progression>,
    pub(super) equipment: ResMut<'w, Equipment>,
    pub(super) game_data: Res<'w, GameData>,
}

/// The flow/scene context the interpreter reads for `CallEvent` (the map events,
/// to resolve a called page), `MemorizeLocation` (the current map and hero tile),
/// `KeyInputProc` (the keyboard), and `ReturnToTitle` (raising the title). Bundled
/// into one nested `SystemParam` to stay within the cap. `map_events`/`map_data`
/// are optional so the system still runs before a map has loaded.
#[derive(SystemParam)]
pub(super) struct FlowCtx<'w, 's> {
    pub(super) screen: crate::world::MapScreen<'w, 's>,
    pub(super) map_events: Option<Res<'w, MapEvents>>,
    pub(super) map_data: Option<Res<'w, MapData>>,
    pub(super) players: Query<'w, 's, &'static Player>,
    pub(super) keys: ResMut<'w, ButtonInput<KeyCode>>,
    pub(super) title: ResMut<'w, TitleActive>,
}

/// The interpreter's character-visual output writers: an actor reskin
/// ([`SpriteChange`], opcode 10630) and a map battle animation
/// ([`ShowMapAnimation`], opcode 11210), plus the [`AnimationLibrary`] the
/// waiting form of 11210 reads to size its block by the animation's frame count.
/// Bundled into one nested `SystemParam` so [`SubsystemIo`] — and thus
/// `run_interpreter` — keeps within Bevy's 16-parameter cap.
#[derive(SystemParam)]
pub(super) struct CharacterVisuals<'w> {
    pub(super) sprite_writer: MessageWriter<'w, SpriteChange>,
    pub(super) anim_writer: MessageWriter<'w, ShowMapAnimation>,
    pub(super) library: Res<'w, AnimationLibrary>,
}

/// The interpreter's merchant channel: the writer that opens the shop/inn screen
/// (opcodes 10720 / 10730) and the outcome it reads back once the screen closes.
/// Bundled into one nested `SystemParam` so [`SubsystemIo`] — and thus
/// `run_interpreter` — keeps within Bevy's 16-parameter cap once the save-request
/// resource is added.
#[derive(SystemParam)]
pub(super) struct Merchant<'w> {
    pub(super) writer: MessageWriter<'w, ShopRequest>,
    pub(super) outcome: Res<'w, ShopOutcome>,
}

/// The interpreter's channel to the shop and battle subsystems: the writers that
/// open each screen and the finished-battle result it consumes to pick a handler
/// branch, plus the nested effect/access/actor/flow bundles. Bundled into one
/// `SystemParam` so `run_interpreter` stays within Bevy's 16-parameter cap.
#[derive(SystemParam)]
pub struct SubsystemIo<'w, 's> {
    pub(super) battle_result: ResMut<'w, BattleResult>,
    pub(super) merchant: Merchant<'w>,
    pub(super) battle_writer: MessageWriter<'w, BattleRequest>,
    pub(super) screen_writer: MessageWriter<'w, ScreenEffect>,
    pub(super) picture_writer: MessageWriter<'w, PictureCommand>,
    pub(super) gameover: ResMut<'w, GameOverActive>,
    pub(super) vitals: ResMut<'w, Vitals>,
    pub(super) input_number: ResMut<'w, InputNumber>,
    pub(super) visuals: CharacterVisuals<'w>,
    pub(super) relocate_writer: MessageWriter<'w, RelocateEvent>,
    pub(super) mapfx: MapFx<'w>,
    pub(super) event_save: ResMut<'w, EventSaveRequest>,
    pub(super) access: AccessFlags<'w>,
    pub(super) actor_edits: ActorEdits<'w>,
    pub(super) flow: FlowCtx<'w, 's>,
    pub(super) event_rng: ResMut<'w, EventRng>,
}

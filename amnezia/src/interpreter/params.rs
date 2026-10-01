//! Grouped interpreter I/O keeps system signatures within Bevy's parameter limit.

use super::event_rng::EventRng;
use crate::animation::{AnimationLibrary, ShowMapAnimation};
use crate::appearance::SpriteChange;
use crate::battle::{BattleActive, BattleResult};
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

/// Pausing overlays; title access is separate in [`FlowCtx`] for ReturnToTitle.
#[derive(SystemParam)]
pub(super) struct Blockers<'w> {
    destination: Option<Res<'w, super::destination::Visit>>,
    menu: Res<'w, MenuOpen>,
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
    frame: Option<Res<'w, crate::timing::SceneWait>>,
    continuation: Option<Res<'w, super::continuation::Continuation>>,
    menu_flow: Option<Res<'w, crate::menu::SceneFlow>>,
    shop_flow: Option<Res<'w, crate::shop::SceneFlow>>,
}

impl Blockers<'_> {
    pub(super) fn fade_busy(&self, fade: &crate::teleport::Fade) -> bool {
        self.destination.is_none() && fade.busy()
    }

    pub(super) fn battle_active(&self) -> bool {
        self.battle.0
    }

    pub(super) fn any(&self) -> bool {
        let waiting = self.destination.is_none() && self.frame.as_ref().is_some_and(|v| v.0);
        self.menu.0
            || self.shop.0
            || self.battle.0
            || self
                .continuation
                .as_ref()
                .map_or(waiting, |state| state.tail_paused(waiting))
            || self.menu_flow.as_ref().is_some_and(|v| v.blocks_map())
            || self.shop_flow.as_ref().is_some_and(|v| v.active())
    }
}

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
    pub(super) system_bgm: ResMut<'w, crate::system_bgm::SystemBgm>,
    pub(super) panorama: ResMut<'w, crate::panorama::Panorama>,
}

#[derive(SystemParam)]
pub(super) struct AccessFlags<'w> {
    pub(super) menu_calling: Option<ResMut<'w, crate::menu::Calling>>,
    pub(super) save_access: ResMut<'w, SaveAccess>,
    pub(super) menu_access: ResMut<'w, MenuAccess>,
}

#[derive(SystemParam)]
pub(super) struct ActorEdits<'w> {
    pub(super) hero_name: ResMut<'w, HeroName>,
    pub(super) progression: ResMut<'w, Progression>,
    pub(super) equipment: ResMut<'w, Equipment>,
    pub(super) game_data: Res<'w, GameData>,
}

/// Scene/flow access; map resources are optional because execution can begin before loading.
#[derive(SystemParam)]
pub(super) struct FlowCtx<'w, 's> {
    pub(super) screen: crate::world::MapScreen<'w, 's>,
    pub(super) map_events: Option<Res<'w, MapEvents>>,
    pub(super) map_data: Option<Res<'w, MapData>>,
    pub(super) players: Query<'w, 's, &'static Player>,
    pub(super) keys: ResMut<'w, ButtonInput<KeyCode>>,
    pub(super) title: ResMut<'w, TitleActive>,
}

#[derive(SystemParam)]
pub(super) struct CharacterVisuals<'w> {
    pub(super) sprite_writer: MessageWriter<'w, SpriteChange>,
    pub(super) anim_writer: MessageWriter<'w, ShowMapAnimation>,
    pub(super) library: Res<'w, AnimationLibrary>,
}

#[derive(SystemParam)]
pub(super) struct Merchant<'w> {
    pub(super) writer: MessageWriter<'w, ShopRequest>,
    pub(super) outcome: Res<'w, ShopOutcome>,
    pub(super) open: Res<'w, ShopOpen>,
    pub(super) scene: Option<Res<'w, crate::shop::SceneFlow>>,
    pub(super) inn: Option<Res<'w, crate::shop::inn::State>>,
}

#[derive(SystemParam)]
pub struct SubsystemIo<'w, 's> {
    pub(super) battle_result: ResMut<'w, BattleResult>,
    pub(super) merchant: Merchant<'w>,
    pub(super) scenes: ResMut<'w, super::scenes::Requests>,
    pub(super) screen_writer: MessageWriter<'w, ScreenEffect>,
    pub(super) picture_writer: MessageWriter<'w, PictureCommand>,
    pub(super) gameover: ResMut<'w, GameOverActive>,
    pub(super) vitals: ResMut<'w, Vitals>,
    pub(super) input_number: ResMut<'w, InputNumber>,
    pub(super) visuals: CharacterVisuals<'w>,
    pub(super) relocate_writer: MessageWriter<'w, RelocateEvent>,
    pub(super) mapfx: MapFx<'w>,
    pub(super) event_save: Res<'w, EventSaveRequest>,
    pub(super) access: AccessFlags<'w>,
    pub(super) actor_edits: ActorEdits<'w>,
    pub(super) flow: FlowCtx<'w, 's>,
    pub(super) event_rng: ResMut<'w, EventRng>,
}

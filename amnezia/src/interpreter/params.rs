//! The interpreter's Bevy `SystemParam` bundles. `run_interpreter` sits at Bevy's
//! 16-parameter cap, so its resources, writers, and queries are grouped into a
//! handful of nested `SystemParam` structs here — keeping the system signature
//! small and the bundle definitions out of the dispatch loop.

use crate::animation::ShowMapAnimation;
use crate::appearance::SpriteChange;
use crate::battle::{BattleActive, BattleRequest, BattleResult};
use crate::dialogue::{MessagePosition, MessageTransparent};
use crate::gamedata::GameData;
use crate::gameover::GameOverActive;
use crate::inputnumber::InputNumber;
use crate::menu::{MenuAccess, MenuOpen};
use crate::picture::PictureCommand;
use crate::player::{CameraPan, HeroTransparency, Player};
use crate::progression::Progression;
use crate::save::{EventSaveRequest, ResumedFromSave, SaveAccess};
use crate::screenfx::{ScreenEffect, Weather};
use crate::shop::{ShopOpen, ShopOutcome, ShopRequest};
use crate::text::HeroName;
use crate::timer::GameClock;
use crate::title::TitleActive;
use crate::vitals::Vitals;
use crate::world::{MapData, MapEvents, RelocateEvent, StartMap};
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

/// The guard that stops the start map's New Game intro from replaying after a
/// resume. The start map's autostart is unconditional (it renames the hero, builds
/// the party, and teleports into the opening), so a Continue that resolved onto the
/// start map — a stale slot, or any resume landing there — would otherwise re-run
/// it and throw the player back to the beginning. Read by `autorun`.
#[derive(SystemParam)]
pub(super) struct IntroGuard<'w> {
    map_data: Option<Res<'w, MapData>>,
    start_map: Res<'w, StartMap>,
    resumed: Res<'w, ResumedFromSave>,
}

impl IntroGuard<'_> {
    /// Whether the current map's autostart must be suppressed: a save has been
    /// loaded this session and the party is on the start map.
    pub(super) fn suppresses_start_intro(&self) -> bool {
        suppresses_start_intro(
            self.resumed.0,
            self.map_data.as_deref().map(|m| m.map_id),
            self.start_map.0,
        )
    }
}

/// The pure suppression decision, split out so it is unit-testable without a world:
/// suppress when a load has happened and the active map is the start map.
pub(super) fn suppresses_start_intro(resumed: bool, map_id: Option<u32>, start_map: u32) -> bool {
    resumed && map_id == Some(start_map)
}

#[cfg(test)]
mod tests {
    use super::suppresses_start_intro;

    #[test]
    fn suppresses_only_the_start_map_after_a_load() {
        // A resumed game on the start map: the New Game intro must not replay.
        assert!(suppresses_start_intro(true, Some(5), 5));
        // Resumed but already teleported onto the saved map: its autostarts run.
        assert!(!suppresses_start_intro(true, Some(2), 5));
        // A fresh New Game on the start map: the intro must run.
        assert!(!suppresses_start_intro(false, Some(5), 5));
        // No map loaded yet: nothing to suppress.
        assert!(!suppresses_start_intro(true, None, 5));
    }
}

/// The small map-effect resources the interpreter writes: camera pan, hero
/// transparency, weather, the message-box position, and the game clock. Bundled
/// into one nested `SystemParam` so [`SubsystemIo`] keeps within Bevy's
/// 16-parameter cap once the access/actor/flow bundles below are added.
#[derive(SystemParam)]
pub(super) struct MapFx<'w> {
    pub(super) camera_pan: ResMut<'w, CameraPan>,
    pub(super) hero_transparency: ResMut<'w, HeroTransparency>,
    pub(super) weather: ResMut<'w, Weather>,
    pub(super) message_position: ResMut<'w, MessagePosition>,
    pub(super) message_transparent: ResMut<'w, MessageTransparent>,
    pub(super) game_clock: ResMut<'w, GameClock>,
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
    pub(super) game_data: Res<'w, GameData>,
}

/// The flow/scene context the interpreter reads for `CallEvent` (the map events,
/// to resolve a called page), `MemorizeLocation` (the current map and hero tile),
/// `KeyInputProc` (the keyboard), and `ReturnToTitle` (raising the title). Bundled
/// into one nested `SystemParam` to stay within the cap. `map_events`/`map_data`
/// are optional so the system still runs before a map has loaded.
#[derive(SystemParam)]
pub(super) struct FlowCtx<'w, 's> {
    pub(super) map_events: Option<Res<'w, MapEvents>>,
    pub(super) map_data: Option<Res<'w, MapData>>,
    pub(super) players: Query<'w, 's, &'static Player>,
    pub(super) keys: Res<'w, ButtonInput<KeyCode>>,
    pub(super) title: ResMut<'w, TitleActive>,
}

/// The interpreter's character-visual output writers: an actor reskin
/// ([`SpriteChange`], opcode 10630) and a map battle animation
/// ([`ShowMapAnimation`], opcode 11210). Bundled into one nested `SystemParam` so
/// [`SubsystemIo`] — and thus `run_interpreter` — keeps within Bevy's
/// 16-parameter cap.
#[derive(SystemParam)]
pub(super) struct CharacterVisuals<'w> {
    pub(super) sprite_writer: MessageWriter<'w, SpriteChange>,
    pub(super) anim_writer: MessageWriter<'w, ShowMapAnimation>,
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
}

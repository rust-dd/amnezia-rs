//! The turn-based, front-view battle system. The interpreter's
//! `EnemyEncounter` (opcode 10710, `params[1]` = troop id) drives it through a
//! clean message/resource contract so the world stays decoupled from the fight:
//!
//! - **IN** — [`BattleRequest`] `{ troop_id }`: start a fight with that troop.
//! - **PAUSE** — [`BattleActive`]`(bool)`: `true` while a fight runs; the world's
//!   movement/interpreter pause guards read it.
//! - **OUT** — [`BattleResult`]`(Option<`[`BattleOutcome`]`>)`: set at the end so
//!   the interpreter can branch to Victory (20710) / Escape (20711) /
//!   Defeat (20712), then clear it.
//!
//! The flow: on a request the encounter is built ([`model::Battle::build`]), the
//! party picks commands ([`input`]), actions resolve fastest-first, and the fight
//! ends in victory (exp + gold rewarded), defeat, or a successful flee. Party
//! HP/SP persists between fights in [`Vitals`]. The Bevy systems that drive this
//! live in [`systems`]; this module owns the plugin, the message/resource
//! contract, and the battle-only database.

mod events;
mod feedback;
pub(crate) mod flow;
pub(crate) mod hud;
mod input;
mod log_terms;
pub(crate) mod logic;
mod model;
mod outcome_text;
mod resolve;
mod scene;
pub(crate) mod smoke;
mod systems;

pub(crate) use flow::BattleFlow;
pub(crate) use logic::{Stats, actor_hp_sp_at, actor_stats_at, equipment_bonus_slots};
pub(crate) use scene::{EffectsSet, smoke::snapshot as battler_snapshot};
pub(crate) use systems::HudCamera;

use crate::assets::{asset_root, load_ron};
use crate::audio::{AudioRequest, BgmTrack};
use crate::vitals::Vitals;
use amnezia_data::{AttributeDef, MonsterDef, StateDef, SystemDef, TroopDef};
use bevy::prelude::*;
use model::Battle;

/// Encounter settings supplied by the map event that starts a fight.
#[derive(Message, Default)]
pub struct BattleRequest {
    pub troop_id: u32,
    pub background: String,
    pub allow_escape: bool,
    pub first_strike: bool,
    pub defeat_ends_game: bool,
}

pub(crate) fn reset_session(world: &mut World) {
    world.insert_resource(Battle::default());
    world.insert_resource(MapBgm::default());
    world.insert_resource(BattleFlow::default());
}

/// Whether a battle is running. The movement/interpreter pause guards OR this in
/// so the overworld freezes for the duration; this module owns the flag.
#[derive(Resource, Default)]
pub struct BattleActive(pub bool);

/// How a battle ended, read by the interpreter to pick its handler branch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BattleOutcome {
    Victory,
    Escape,
    Defeat,
    Abort,
}

/// The finished battle's outcome, set once when the fight ends and cleared by the
/// interpreter after it branches.
#[derive(Resource, Default)]
pub struct BattleResult(pub Option<BattleOutcome>);

/// The battle database not held by [`GameData`]: monster and troop definitions,
/// loaded once at plugin build like the other converted RON.
///
/// [`GameData`]: crate::gamedata::GameData
#[derive(Resource)]
struct BattleData {
    monsters: Vec<MonsterDef>,
    troops: Vec<TroopDef>,
    /// The attribute (element) and state tables, loaded for the elemental and
    /// status resolution that lands separately.
    #[allow(dead_code)]
    attributes: Vec<AttributeDef>,
    #[allow(dead_code)]
    states: Vec<StateDef>,
    /// The system audio definition: the battle / victory / game-over music and the
    /// per-hit sound effects the battle system plays.
    system: SystemDef,
}

/// The map BGM that was playing when the fight began, remembered so it restores
/// when the battle tears down. `None` means the map was silent, so teardown
/// stops the BGM rather than replaying anything.
#[derive(Resource, Default)]
struct MapBgm(Option<BgmTrack>);

impl MapBgm {
    /// Remember `track` (the pre-battle map BGM) so it can be restored later.
    fn memorize(&mut self, track: Option<BgmTrack>) {
        self.0 = track;
    }

    /// The request that restores the memorized BGM: replay the remembered track,
    /// or stop the BGM when the map was silent.
    fn restore(&self) -> AudioRequest {
        match &self.0 {
            Some(track) => track.replay(),
            None => AudioRequest::StopBgm,
        }
    }
}

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BattleRequest>()
            .init_resource::<BattleActive>()
            .init_resource::<BattleResult>()
            .init_resource::<Battle>()
            .init_resource::<Vitals>()
            .init_resource::<MapBgm>()
            .insert_resource(BattleData {
                monsters: load_ron(&format!("{}/monsters.ron", asset_root())),
                troops: load_ron(&format!("{}/troops.ron", asset_root())),
                attributes: load_ron(&format!("{}/attributes.ron", asset_root())),
                states: load_ron(&format!("{}/states.ron", asset_root())),
                system: load_ron(&format!("{}/system.ron", asset_root())),
            })
            .add_systems(Startup, systems::spawn_hud_camera)
            .add_systems(
                Update,
                (
                    systems::start_on_request.after(crate::audio::AudioRequests),
                    systems::debug_trigger,
                    input::command_input
                        .run_if(flow::playing)
                        .run_if(hud::commands_ready),
                    events::drive
                        .run_if(flow::playing)
                        .after(systems::start_on_request)
                        .after(crate::dialogue::DialogueInput)
                        .before(input::command_input)
                        .before(systems::resolve_tick)
                        .before(systems::apply_victory_rewards)
                        .before(systems::outcome_input),
                    systems::abort_expired_battle
                        .run_if(flow::playing)
                        .after(crate::timer::ClockTick)
                        .after(systems::start_on_request)
                        .before(events::drive)
                        .before(input::command_input)
                        .before(systems::resolve_tick)
                        .before(systems::apply_victory_rewards)
                        .before(systems::outcome_input),
                    systems::resolve_tick.run_if(flow::playing),
                    events::sync_switches
                        .after(systems::resolve_tick)
                        .before(systems::outcome_input),
                    systems::apply_victory_rewards
                        .run_if(flow::playing)
                        .after(systems::resolve_tick)
                        .before(systems::outcome_input),
                    systems::outcome_input.run_if(flow::playing),
                    systems::drain_pending_anims
                        .run_if(flow::playing)
                        .after(systems::resolve_tick),
                    systems::drain_pending_se
                        .run_if(flow::playing)
                        .after(systems::resolve_tick),
                ),
            );
        flow::register(app);
        scene::register(app);
        feedback::register(app);
        hud::register(app);
    }
}

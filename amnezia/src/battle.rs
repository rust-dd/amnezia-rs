//! The turn-based, front-view battle system (Milestone 7). The interpreter's
//! `EnemyEncounter` (opcode 10710, `params[0]` = troop id) drives it through a
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
//! HP/SP persists between fights in [`Vitals`].

mod input;
mod logic;
mod model;
mod resolve;
mod ui;

use crate::assets::{ASSET_ROOT, load_ron};
use crate::gamedata::GameData;
use crate::state::{Inventory, Party};
use crate::vitals::Vitals;
use amnezia_data::{ActorDef, MonsterDef, TroopDef};
use bevy::prelude::*;
use model::{Battle, Phase};

/// The troop the debug key spawns: troop 2 "Rablo2x", two bandits.
const DEBUG_TROOP: u32 = 2;

/// The backdrop every fight uses in v1 (per-troop terrain backdrops are deferred).
const BACKDROP: &str = "Cave1";

/// Start a fight with the given troop. The interpreter emits this from opcode
/// 10710; the debug key emits it too until that wiring lands.
#[derive(Message)]
pub struct BattleRequest {
    pub troop_id: u32,
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
}

/// The finished battle's outcome, set once when the fight ends and cleared by the
/// interpreter after it branches.
#[derive(Resource, Default)]
pub struct BattleResult(pub Option<BattleOutcome>);

/// The battle database not held by [`GameData`]: monster and troop definitions,
/// loaded once at plugin build like the other converted RON.
#[derive(Resource)]
struct BattleData {
    monsters: Vec<MonsterDef>,
    troops: Vec<TroopDef>,
}

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BattleRequest>()
            .init_resource::<BattleActive>()
            .init_resource::<BattleResult>()
            .init_resource::<Battle>()
            .init_resource::<Vitals>()
            .insert_resource(BattleData {
                monsters: load_ron(&format!("{ASSET_ROOT}/monsters.ron")),
                troops: load_ron(&format!("{ASSET_ROOT}/troops.ron")),
            })
            .add_systems(
                Update,
                (
                    start_on_request,
                    debug_trigger,
                    input::command_input,
                    resolve_tick,
                    outcome_input,
                ),
            );
        ui::register(app);
    }
}

/// Build a live encounter when a [`BattleRequest`] arrives (ignored while one is
/// already running), instantiating the troop and the current party roster.
#[allow(clippy::too_many_arguments)]
fn start_on_request(
    mut requests: MessageReader<BattleRequest>,
    data: Res<GameData>,
    battle_data: Res<BattleData>,
    party: Res<Party>,
    vitals: Res<Vitals>,
    mut battle: ResMut<Battle>,
    mut active: ResMut<BattleActive>,
    mut result: ResMut<BattleResult>,
) {
    let Some(request) = requests.read().last() else {
        return;
    };
    if active.0 {
        return;
    }
    let Some(troop) = battle_data.troops.iter().find(|t| t.id == request.troop_id) else {
        return;
    };
    if troop.members.is_empty() {
        return;
    }
    let roster = party.snapshot();
    let actors: Vec<&ActorDef> = roster.iter().filter_map(|id| data.actor(*id)).collect();
    if actors.is_empty() {
        return;
    }
    let seed = seed_now() ^ (request.troop_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    *battle = Battle::build(
        troop,
        &battle_data.monsters,
        &actors,
        &vitals,
        BACKDROP.to_string(),
        seed,
    );
    active.0 = true;
    result.0 = None;
}

/// Debug-only: F6 starts a sample fight so the battle can be exercised before the
/// interpreter emits [`BattleRequest`]. Remove when opcode 10710 is wired.
fn debug_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    active: Res<BattleActive>,
    mut requests: MessageWriter<BattleRequest>,
) {
    if !active.0 && keys.just_pressed(KeyCode::F6) {
        requests.write(BattleRequest {
            troop_id: DEBUG_TROOP,
        });
    }
}

/// Step the resolution phase: apply one queued action per timer tick, then end
/// the fight or open a fresh command round once the queue is spent.
fn resolve_tick(time: Res<Time>, mut battle: ResMut<Battle>) {
    if battle.phase != Phase::Resolve {
        return;
    }
    if !battle.timer.tick(time.delta()).just_finished() {
        return;
    }
    let more = battle.resolve_next();
    if let Some(outcome) = battle.end_state() {
        battle.finish(outcome);
    } else if !more {
        battle.new_round();
    }
}

/// On the confirm key at the outcome screen: persist party HP/SP, pay out gold on
/// a win, publish the [`BattleResult`], and tear the battle down.
fn outcome_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<Battle>,
    mut active: ResMut<BattleActive>,
    mut result: ResMut<BattleResult>,
    mut inventory: ResMut<Inventory>,
    mut vitals: ResMut<Vitals>,
) {
    if battle.phase != Phase::Outcome {
        return;
    }
    if !(keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)) {
        return;
    }
    let outcome = battle.outcome.unwrap_or(BattleOutcome::Escape);
    for fighter in &battle.members {
        vitals.set(fighter.actor_id, fighter.hp.max(0), fighter.sp);
    }
    if outcome == BattleOutcome::Victory {
        inventory.add_gold(battle.reward_gold as i32);
    }
    result.0 = Some(outcome);
    active.0 = false;
    *battle = Battle::default();
}

/// A time-derived battle seed; the low bit is forced set by [`Battle::build`].
fn seed_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::TroopMemberDef;
    use bevy::input::ButtonInput;

    /// A headless app wired with just the battle logic systems (no UI, which
    /// needs an asset server) plus the resources they read.
    fn logic_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<BattleRequest>();
        app.insert_resource(GameData {
            actors: vec![ActorDef {
                id: 1,
                name: "Ron".into(),
                title: "Zsoldos".into(),
                level: 2,
                max_level: 50,
                hp: 63,
                sp: 37,
            }],
            items: vec![],
            skills: vec![],
        });
        app.insert_resource(BattleData {
            monsters: vec![MonsterDef {
                id: 1,
                name: "Rabló".into(),
                battler: "Cannibal".into(),
                max_hp: 30,
                max_sp: 0,
                attack: 20,
                defense: 8,
                spirit: 0,
                agility: 8,
                exp: 10,
                gold: 30,
            }],
            troops: vec![TroopDef {
                id: DEBUG_TROOP,
                name: "Rablo".into(),
                members: vec![TroopMemberDef {
                    enemy_id: 1,
                    x: 100,
                    y: 100,
                }],
            }],
        });
        app.init_resource::<Party>();
        app.init_resource::<Inventory>();
        app.init_resource::<Vitals>();
        app.init_resource::<Battle>();
        app.init_resource::<BattleActive>();
        app.init_resource::<BattleResult>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, (debug_trigger, start_on_request, outcome_input));
        app
    }

    #[test]
    fn f6_starts_a_battle_and_confirming_a_win_publishes_the_contract_out() {
        let mut app = logic_app();
        // F6 -> debug_trigger emits BattleRequest -> start_on_request builds it.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F6);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert!(
            app.world().resource::<BattleActive>().0,
            "battle should be running"
        );
        {
            let battle = app.world().resource::<Battle>();
            assert!(battle.phase == Phase::Command);
            assert_eq!(battle.enemies.len(), 1);
            assert_eq!(battle.members.len(), 1);
        }
        // Force a victory, then confirm at the outcome screen.
        app.world_mut()
            .resource_mut::<Battle>()
            .finish(BattleOutcome::Victory);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert_eq!(
            app.world().resource::<BattleResult>().0,
            Some(BattleOutcome::Victory)
        );
        assert!(
            !app.world().resource::<BattleActive>().0,
            "battle should have ended"
        );
        assert_eq!(
            app.world().resource::<Inventory>().gold(),
            30,
            "gold reward paid out"
        );
        assert!(app.world().resource::<Battle>().phase == Phase::Inactive);
    }
}

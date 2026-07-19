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

mod floaters;
mod hud;
mod input;
mod log_terms;
mod logic;
mod model;
mod resolve;
mod scene;

pub(crate) use logic::{Stats, actor_hp_sp_at, actor_stats_at, equipment_bonus};

use crate::animation::{ActiveAnimations, AnimAnchor, PlayAnimation};
use crate::assets::{asset_root, load_ron};
use crate::audio::{AudioRequest, BgmTrack, CurrentBgm};
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::vitals::Vitals;
use amnezia_data::{ActorDef, AttributeDef, MonsterDef, StateDef, SystemDef, TroopDef};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use model::{Battle, BattleSe, Phase};

/// The troop the debug key spawns: troop 2 "Rablo2x", two bandits.
const DEBUG_TROOP: u32 = 2;

/// The backdrop every fight uses in v1 (per-troop terrain backdrops are deferred).
const BACKDROP: &str = "Cave1";

/// The RM2000 screen offset a screen-scope battle animation centres its cells on:
/// EasyRPG `BattleAnimationBattle::Draw` uses `(screen_w/2, screen_h/3)`, which in
/// our centre-origin 320×240 overlay (`0,0` = centre) is `(0, 80 - 120)` =
/// `(0, -40)` (y downward).
const BATTLE_SCREEN_CENTER: Vec2 = Vec2::new(0.0, -40.0);

/// The target height a party-area animation assumes: RM2000 front view draws no
/// party sprites, so there is no battler to measure, and this matches the
/// fallback EasyRPG uses when a battler bitmap is not yet ready.
const PARTY_TARGET_HEIGHT: f32 = 48.0;

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
/// when the battle tears down. A battle-scoped memorize/restore (the general
/// `MemorizeBGM` opcode is #37); `None` means the map was silent, so teardown
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
            .add_systems(Startup, spawn_hud_camera)
            .add_systems(
                Update,
                (
                    start_on_request,
                    debug_trigger,
                    input::command_input,
                    resolve_tick,
                    apply_victory_rewards
                        .after(resolve_tick)
                        .before(outcome_input),
                    outcome_input,
                    drain_pending_anims.after(resolve_tick),
                    drain_pending_se.after(resolve_tick),
                ),
            );
        scene::register(app);
        floaters::register(app);
        hud::register(app);
    }
}

/// The order-2 HUD camera the battle windows render on. It sits above the order-1
/// effect overlay (backdrop, battlers, animations) and the order-0 world. Its
/// [`RenderLayers`] points at the otherwise-unused layer 2 so its 2D pass draws no
/// world sprites — only the HUD, which `bevy_ui` composites by target camera, not
/// by render layer.
#[derive(Component)]
struct HudCamera;

/// Spawn the HUD camera at startup (order 3, no clear), so [`hud`]'s windows can
/// target it and paint over the effect overlay the battle scene draws on.
fn spawn_hud_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 3,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(2),
        HudCamera,
    ));
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
    progression: Res<Progression>,
    terms: Res<Terms>,
    current_bgm: Res<CurrentBgm>,
    mut map_bgm: ResMut<MapBgm>,
    mut audio: MessageWriter<AudioRequest>,
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
        &data.items,
        &battle_data.attributes,
        &battle_data.states,
        &data.skills,
        &vitals,
        &progression,
        BACKDROP.to_string(),
        seed,
    );
    // Capture the real RM2000 battle-end message terms for the outcome/reward
    // log lines the resolution code (which has no resources) composes.
    battle.text.apply(&terms.0);
    active.0 = true;
    result.0 = None;
    // Remember the map BGM (read before the battle track replaces it this frame),
    // sound the battle-start SE, then start the looping battle BGM.
    let system = &battle_data.system;
    map_bgm.memorize(current_bgm.track());
    if let Some(se) = AudioRequest::se(
        &system.battle_se.name,
        system.battle_se.volume,
        system.battle_se.tempo,
    ) {
        audio.write(se);
    }
    audio.write(AudioRequest::bgm(
        &system.battle_music.name,
        system.battle_music.volume,
        system.battle_music.tempo,
    ));
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
/// the fight or open a fresh command round once the queue is spent. On the end,
/// the victory/game-over fanfare interrupts the battle BGM (the map BGM restores
/// on teardown).
fn resolve_tick(
    time: Res<Time>,
    battle_data: Res<BattleData>,
    active_anims: Res<ActiveAnimations>,
    mut audio: MessageWriter<AudioRequest>,
    mut battle: ResMut<Battle>,
) {
    if battle.phase != Phase::Resolve {
        return;
    }
    // Hold the step while a slain foe plays out its death or explosion, so the
    // beat is seen before the next action lands (RM2000 `SetWait(36, 60)`).
    if battle.death_in_progress() {
        return;
    }
    // Hold while a queued attack animation plays out, so a strike/cast's damage
    // number lands only once the swing/cast finishes (RM2000 sequences the
    // animation, its wait, then the damage). Once it clears, apply the deferred
    // impact at once and re-pace the timer for the next beat; otherwise keep the
    // normal per-action cadence.
    let advance = if battle.anim_hold_active() {
        if battle.tick_anim_hold(active_anims.0 > 0) {
            return;
        }
        battle.timer.reset();
        true
    } else {
        battle.timer.tick(time.delta()).just_finished()
    };
    if !advance {
        return;
    }
    let more = battle.resolve_next();
    if let Some(outcome) = battle.end_state() {
        battle.finish(outcome);
        play_outcome_music(&mut audio, &battle_data.system, outcome);
    } else if !more {
        battle.new_round();
    }
}

/// Play the fanfare for a finished fight: the victory ME on a win, the game-over
/// music on a defeat. Both are sent as the looping BGM so they interrupt the
/// battle track under the outcome screen; the map BGM restores on teardown. A
/// successful escape has no fanfare (only its SE, played at the flee attempt).
fn play_outcome_music(
    audio: &mut MessageWriter<AudioRequest>,
    system: &SystemDef,
    outcome: BattleOutcome,
) {
    let music = match outcome {
        BattleOutcome::Victory => &system.battle_end_music,
        BattleOutcome::Defeat => &system.gameover_music,
        BattleOutcome::Escape => return,
    };
    audio.write(AudioRequest::bgm(&music.name, music.volume, music.tempo));
}

/// Drain the battle's per-hit sound-effect queue into [`AudioRequest`]s, naming
/// each effect from the loaded [`SystemDef`] (a hit landed, a foe felled, an
/// attack evaded, an escape attempt). Guarded on non-empty so an idle fight never
/// marks [`Battle`] changed (which would re-run the UI every frame); an
/// `(OFF)`/absent effect is skipped.
fn drain_pending_se(
    battle_data: Res<BattleData>,
    mut audio: MessageWriter<AudioRequest>,
    mut battle: ResMut<Battle>,
) {
    if battle.pending_se.is_empty() {
        return;
    }
    let system = &battle_data.system;
    for kind in battle.pending_se.drain(..) {
        let sound = match kind {
            BattleSe::EnemyDamaged => &system.enemy_damaged_se,
            BattleSe::ActorDamaged => &system.actor_damaged_se,
            BattleSe::Dodge => &system.dodge_se,
            BattleSe::EnemyDefeated => &system.enemy_defeated_se,
            BattleSe::Escape => &system.escape_se,
        };
        if let Some(req) = AudioRequest::se(&sound.name, sound.volume, sound.tempo) {
            audio.write(req);
        }
    }
}

/// Drain the battle's per-tick animation queue into overlay [`PlayAnimation`]
/// messages: each action resolved this tick queued one animation over its
/// target(s) (see `resolve`). This attaches each target's battler pixel height —
/// looked up from the live battler sprites (see `scene::battler_height_at`), or a
/// party-area default — so the animation's `position` anchor can offset from it,
/// and stamps the battle screen-centre for a screen-scope effect. Emitting here
/// keeps the queue-push Bevy-free and plays each effect once. Guarded on non-empty
/// so an idle fight never marks [`Battle`] changed (which would re-run the UI
/// every frame).
fn drain_pending_anims(
    mut battle: ResMut<Battle>,
    battlers: Query<&scene::Battler>,
    mut plays: MessageWriter<PlayAnimation>,
) {
    if battle.pending_anims.is_empty() {
        return;
    }
    for anim in battle.pending_anims.drain(..) {
        let targets = anim
            .targets
            .iter()
            .map(|&(x, y)| {
                let pos = Vec2::new(x, y);
                AnimAnchor {
                    pos,
                    height: scene::battler_height_at(&battlers, pos).unwrap_or(PARTY_TARGET_HEIGHT),
                }
            })
            .collect();
        plays.write(PlayAnimation {
            anim_id: anim.anim_id,
            targets,
            screen_center: BATTLE_SCREEN_CENTER,
        });
    }
}

/// On entering the victory outcome, pay the fight's reward exactly once (before
/// the outcome screen, so a level-up shows now rather than next fight): add the
/// gold, award the experience to every member — raising their persistent level —
/// and append a level-up line for each actor whose level rose. This is the RM2000
/// `ProcessSceneActionVictory` per-actor `ChangeExp` beat; `rewarded` guards it so
/// it never double-pays while the outcome screen waits for the player.
fn apply_victory_rewards(
    mut battle: ResMut<Battle>,
    data: Res<GameData>,
    mut inventory: ResMut<Inventory>,
    mut progression: ResMut<Progression>,
) {
    if battle.phase != Phase::Outcome
        || battle.outcome != Some(BattleOutcome::Victory)
        || battle.rewarded
    {
        return;
    }
    battle.rewarded = true;
    inventory.add_gold(battle.reward_gold as i32);
    let exp = battle.reward_exp;
    let mut level_ups: Vec<String> = Vec::new();
    for fighter in &battle.members {
        // RM2000 awards experience only to the active (living) members
        // (`GetActiveBattlers`); a fallen member gains none.
        if !fighter.alive() {
            continue;
        }
        if let Some(def) = data.actor(fighter.actor_id) {
            let before = progression.level(def);
            progression.add(def, exp);
            let after = progression.level(def);
            if after > before {
                level_ups.push(format!("{} elérte a(z) {after}. szintet!", fighter.name));
                // Each learning crossed by the level gain is learned now; RM2000
                // logs a line per newly learned skill.
                for learn in &def.learnings {
                    if learn.level > before
                        && learn.level <= after
                        && let Some(skill) = data.skills.iter().find(|s| s.id == learn.skill_id)
                    {
                        level_ups.push(format!("{} megtanulta: {}", fighter.name, skill.name));
                    }
                }
            }
        }
    }
    battle.log.extend(level_ups);
}

/// On the confirm key at the outcome screen: persist party HP/SP, publish the
/// [`BattleResult`], and tear the battle down. The victory reward (gold and
/// experience) was already paid by [`apply_victory_rewards`] on entering the
/// outcome, so it is not applied again here.
fn outcome_input(
    keys: Res<ButtonInput<KeyCode>>,
    map_bgm: Res<MapBgm>,
    mut audio: MessageWriter<AudioRequest>,
    mut battle: ResMut<Battle>,
    mut active: ResMut<BattleActive>,
    mut result: ResMut<BattleResult>,
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
    result.0 = Some(outcome);
    active.0 = false;
    *battle = Battle::default();
    // Restore the map BGM that the battle (and any fanfare) replaced.
    audio.write(map_bgm.restore());
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
    use amnezia_data::{MusicDef, SkillDef, SoundDef, TroopMemberDef};
    use bevy::input::ButtonInput;

    /// A System audio def with a battle track, start SE, and the per-hit effects
    /// set, so the battle-audio systems have names to play.
    fn test_system() -> SystemDef {
        let music = |name: &str| MusicDef {
            name: name.into(),
            volume: 90,
            tempo: 100,
            ..default()
        };
        let sound = |name: &str| SoundDef {
            name: name.into(),
            volume: 90,
            tempo: 100,
            ..default()
        };
        SystemDef {
            battle_music: music("Battle"),
            battle_end_music: music("Victory"),
            gameover_music: music("Gameover"),
            battle_se: sound("Start"),
            enemy_damaged_se: sound("Damage"),
            actor_damaged_se: sound("Bite"),
            dodge_se: sound("Evasion"),
            enemy_defeated_se: sound("Die"),
            escape_se: sound("Escape"),
            ..default()
        }
    }

    /// A headless app wired with just the battle logic systems (no UI, which
    /// needs an asset server) plus the resources they read.
    fn logic_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<BattleRequest>();
        app.add_message::<AudioRequest>();
        app.insert_resource(GameData {
            actors: vec![ActorDef {
                id: 1,
                name: "Ron".into(),
                title: "Zsoldos".into(),
                level: 2,
                max_level: 50,
                hp: 63,
                sp: 37,
                curves: Default::default(),
                learnings: Vec::new(),
                exp_base: 30,
                exp_inflation: 30,
                exp_correction: 0,
                weapon: 0,
                shield: 0,
                armor: 0,
                helmet: 0,
                accessory: 0,
                two_weapons: false,
                fix_equipment: false,
                unarmed_animation: 0,
                face_name: String::new(),
                face_index: 0,
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
                attribute_ranks: vec![],
                state_ranks: vec![],
                actions: vec![],
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
            attributes: vec![],
            states: vec![],
            system: test_system(),
        });
        app.init_resource::<Party>();
        app.init_resource::<Inventory>();
        app.init_resource::<Vitals>();
        app.init_resource::<Progression>();
        app.init_resource::<Battle>();
        app.init_resource::<BattleActive>();
        app.init_resource::<BattleResult>();
        app.init_resource::<MapBgm>();
        app.init_resource::<CurrentBgm>();
        app.init_resource::<Terms>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(
            Update,
            (
                debug_trigger,
                start_on_request,
                apply_victory_rewards.before(outcome_input),
                outcome_input,
            ),
        );
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
            assert!(battle.phase == Phase::PartyCommand);
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

    #[test]
    fn a_threshold_victory_levels_up_before_the_outcome_and_pays_exactly_once() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Ron at level 1: reaching level 2 needs exp_for_level(2) = exp_base = 30.
        let ron = ActorDef {
            id: 1,
            name: "Ron".into(),
            title: String::new(),
            level: 1,
            max_level: 50,
            hp: 40,
            sp: 10,
            curves: Default::default(),
            // Ron learns skill 1 at level 2, so crossing into level 2 learns it.
            learnings: vec![amnezia_data::Learning {
                level: 2,
                skill_id: 1,
            }],
            exp_base: 30,
            exp_inflation: 30,
            exp_correction: 0,
            weapon: 0,
            shield: 0,
            armor: 0,
            helmet: 0,
            accessory: 0,
            two_weapons: false,
            fix_equipment: false,
            unarmed_animation: 0,
            face_name: String::new(),
            face_index: 0,
        };
        app.insert_resource(GameData {
            actors: vec![ron.clone()],
            items: vec![],
            skills: vec![SkillDef {
                id: 1,
                name: "Tűzcsapás".into(),
                description: String::new(),
                sp_cost: 0,
                power: 10,
                hit: 100,
                skill_type: 0,
                scope: 0,
                animation_id: 0,
                physical_rate: 0,
                magical_rate: 0,
                variance: 0,
                affect_hp: true,
                affect_sp: false,
                absorb: false,
                attributes: vec![],
                affected_states: vec![],
            }],
        });
        app.init_resource::<Inventory>();
        app.init_resource::<Vitals>();
        app.init_resource::<Progression>();
        app.init_resource::<BattleResult>();
        app.init_resource::<BattleActive>();
        app.init_resource::<MapBgm>();
        app.add_message::<AudioRequest>();
        app.init_resource::<ButtonInput<KeyCode>>();
        // A foe worth exactly 30 exp — enough to lift Ron from level 1 to 2 — and
        // 30 gold.
        let monsters = vec![MonsterDef {
            id: 1,
            name: "Rabló".into(),
            battler: String::new(),
            max_hp: 30,
            max_sp: 0,
            attack: 20,
            defense: 8,
            spirit: 0,
            agility: 8,
            exp: 30,
            gold: 30,
            attribute_ranks: vec![],
            state_ranks: vec![],
            actions: vec![],
        }];
        let troop = TroopDef {
            id: 1,
            name: "T".into(),
            members: vec![TroopMemberDef {
                enemy_id: 1,
                x: 100,
                y: 100,
            }],
        };
        let battle = Battle::build(
            &troop,
            &monsters,
            &[&ron],
            &[],
            &[],
            &[],
            &[],
            &Vitals::default(),
            &Progression::default(),
            "Cave1".into(),
            1,
        );
        app.insert_resource(battle);
        app.add_systems(
            Update,
            (apply_victory_rewards.before(outcome_input), outcome_input),
        );
        // Win the fight, then enter the outcome without confirming: the reward must
        // pay and the level rise now, before the outcome screen.
        app.world_mut()
            .resource_mut::<Battle>()
            .finish(BattleOutcome::Victory);
        app.update();
        assert_eq!(
            app.world().resource::<Progression>().level(&ron),
            2,
            "the level rises on entering the outcome, not next fight"
        );
        assert_eq!(
            app.world().resource::<Inventory>().gold(),
            30,
            "gold is paid once, at victory time"
        );
        assert!(
            app.world()
                .resource::<Battle>()
                .log
                .iter()
                .any(|l| l.contains("szintet")),
            "a level-up line is staged into the battle log"
        );
        assert!(
            app.world()
                .resource::<Battle>()
                .log
                .iter()
                .any(|l| l.contains("megtanulta") && l.contains("Tűzcsapás")),
            "crossing level 2 learns and logs the level-2 skill"
        );
        let total_once = app.world().resource::<Progression>().total(&ron);
        // Confirm at the outcome screen: gold and exp must not be applied a second
        // time.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert_eq!(
            app.world().resource::<Inventory>().gold(),
            30,
            "gold is not double-paid on confirm"
        );
        assert_eq!(
            app.world().resource::<Progression>().total(&ron),
            total_once,
            "experience is not double-applied on confirm"
        );
    }

    #[test]
    fn memorize_and_restore_round_trips_the_map_bgm() {
        let mut memory = MapBgm::default();
        // A silent map restores to a stop, not a phantom track.
        assert_eq!(memory.restore(), AudioRequest::StopBgm);
        memory.memorize(Some(BgmTrack {
            name: "Field".into(),
            volume: 0.8,
            speed: 1.0,
        }));
        assert_eq!(
            memory.restore(),
            AudioRequest::Bgm {
                name: "Field".into(),
                volume: 0.8,
                speed: 1.0,
                fade_in: 0.0,
            }
        );
        // Memorizing "nothing playing" restores to a stop.
        memory.memorize(None);
        assert_eq!(memory.restore(), AudioRequest::StopBgm);
    }

    #[test]
    fn battle_start_plays_battle_music_and_stores_the_prior_bgm() {
        let mut app = logic_app();
        // A map track is playing when the fight starts.
        app.insert_resource(CurrentBgm::with_track("Field", 0.7, 1.0));
        // Request the fight directly (avoids the debug-key intra-frame ordering).
        app.world_mut().write_message(BattleRequest {
            troop_id: DEBUG_TROOP,
        });
        app.update();
        // The pre-battle map BGM is remembered for the teardown restore.
        assert_eq!(
            app.world().resource::<MapBgm>().0,
            Some(BgmTrack {
                name: "Field".into(),
                volume: 0.7,
                speed: 1.0
            })
        );
        // The battle BGM and the battle-start SE were requested on build.
        let messages = app.world().resource::<Messages<AudioRequest>>();
        let mut cursor = messages.get_cursor();
        let played: Vec<AudioRequest> = cursor.read(messages).cloned().collect();
        assert!(
            played
                .iter()
                .any(|r| matches!(r, AudioRequest::Bgm { name, .. } if name.as_str() == "Battle")),
            "battle BGM should start on battle build: {played:?}"
        );
        assert!(
            played
                .iter()
                .any(|r| matches!(r, AudioRequest::Sound { name, .. } if name.as_str() == "Start")),
            "battle-start SE should sound"
        );
    }
}

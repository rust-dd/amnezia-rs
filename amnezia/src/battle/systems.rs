//! The Bevy systems that drive a fight: starting an encounter on request, the
//! debug trigger, stepping resolution, paying victory rewards, the confirm/teardown
//! at the outcome, draining the per-tick sound/animation queues, and the HUD camera.

use super::model::{Battle, BattleSe, Phase};
use super::scene;
use super::{BattleActive, BattleData, BattleOutcome, BattleRequest, BattleResult, MapBgm};
use crate::animation::{ActiveAnimations, AnimAnchor, PlayAnimation};
use crate::audio::{AudioRequest, CurrentBgm};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::vitals::Vitals;
use amnezia_data::{ActorDef, SystemDef};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

/// The troop the debug key spawns: troop 2 "Rablo2x", two bandits.
const DEBUG_TROOP: u32 = 2;

/// Fallback backdrop for the developer's test encounter.
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

/// The order-2 HUD camera the battle windows render on. It sits above the order-1
/// effect overlay (backdrop, battlers, animations) and the order-0 world. Its
/// [`RenderLayers`] points at the otherwise-unused layer 2 so its 2D pass draws no
/// world sprites — only the HUD, which `bevy_ui` composites by target camera, not
/// by render layer.
#[derive(Component)]
pub(super) struct HudCamera;

/// Spawn the HUD camera at startup (order 3, no clear), so [`crate::battle::hud`]'s
/// windows can target it and paint over the effect overlay the battle scene draws on.
pub(super) fn spawn_hud_camera(mut commands: Commands) {
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
pub(super) fn start_on_request(
    mut requests: MessageReader<BattleRequest>,
    data: Res<GameData>,
    battle_data: Res<BattleData>,
    party: Res<Party>,
    vitals: Res<Vitals>,
    progression: Res<Progression>,
    equipment: Res<Equipment>,
    terms: Res<Terms>,
    current_bgm: Res<CurrentBgm>,
    system_bgm: Option<Res<crate::system_bgm::SystemBgm>>,
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
        warn!("cannot start undefined troop {}", request.troop_id);
        result.0 = Some(BattleOutcome::Defeat);
        return;
    };
    if troop.members.is_empty() {
        result.0 = Some(BattleOutcome::Victory);
        return;
    }
    let roster = party.snapshot();
    let actors: Vec<&ActorDef> = roster.iter().filter_map(|id| data.actor(*id)).collect();
    if actors.is_empty() {
        result.0 = Some(BattleOutcome::Defeat);
        return;
    }
    // Resolve each member's runtime loadout so the fight reads the gear the equip
    // menu changed, not the static ActorDef starting gear.
    let equipped: Vec<[u32; 5]> = actors.iter().map(|a| equipment.slots(a)).collect();
    let seed = seed_now() ^ (request.troop_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    *battle = Battle::build(
        troop,
        &battle_data.monsters,
        &actors,
        &equipped,
        &data.items,
        &battle_data.attributes,
        &battle_data.states,
        &data.skills,
        &vitals,
        &progression,
        if request.background.is_empty() {
            BACKDROP.to_string()
        } else {
            request.background.clone()
        },
        seed,
    );
    battle.allow_escape = request.allow_escape;
    battle.first_strike = request.first_strike;
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
    audio.write(AudioRequest::from_music(crate::system_bgm::resolve(
        system_bgm.as_deref(),
        0,
        &system.battle_music,
    )));
}

/// Debug-only: F6 starts a sample fight so the battle can be exercised before the
/// interpreter emits [`BattleRequest`]. Remove when opcode 10710 is wired.
pub(super) fn debug_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    active: Res<BattleActive>,
    mut requests: MessageWriter<BattleRequest>,
) {
    if !active.0 && keys.just_pressed(KeyCode::F6) {
        requests.write(BattleRequest {
            troop_id: DEBUG_TROOP,
            allow_escape: true,
            ..default()
        });
    }
}

/// Step the resolution phase: apply one queued action per timer tick, then end
/// the fight or open a fresh command round once the queue is spent. On the end,
/// the victory/game-over fanfare interrupts the battle BGM (the map BGM restores
/// on teardown).
pub(super) fn resolve_tick(
    time: Res<Time>,
    battle_data: Res<BattleData>,
    active_anims: Res<ActiveAnimations>,
    system_bgm: Option<Res<crate::system_bgm::SystemBgm>>,
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
        play_outcome_music(
            &mut audio,
            &battle_data.system,
            system_bgm.as_deref(),
            outcome,
        );
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
    overrides: Option<&crate::system_bgm::SystemBgm>,
    outcome: BattleOutcome,
) {
    let (slot, music) = match outcome {
        BattleOutcome::Victory => (1, &system.battle_end_music),
        BattleOutcome::Defeat => (6, &system.gameover_music),
        BattleOutcome::Escape => return,
    };
    audio.write(AudioRequest::from_music(crate::system_bgm::resolve(
        overrides, slot, music,
    )));
}

/// Drain the battle's per-hit sound-effect queue into [`AudioRequest`]s, naming
/// each effect from the loaded [`SystemDef`] (a hit landed, a foe felled, an
/// attack evaded, an escape attempt). Guarded on non-empty so an idle fight never
/// marks [`Battle`] changed (which would re-run the UI every frame); an
/// `(OFF)`/absent effect is skipped.
pub(super) fn drain_pending_se(
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
            BattleSe::Cursor => &system.cursor_se,
            BattleSe::Decision => &system.decision_se,
            BattleSe::Cancel => &system.cancel_se,
            BattleSe::Buzzer => &system.buzzer_se,
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
pub(super) fn drain_pending_anims(
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
            global: false,
        });
    }
}

/// On entering the victory outcome, pay the fight's reward exactly once (before
/// the outcome screen, so a level-up shows now rather than next fight): add the
/// gold, award the experience to every member — raising their persistent level —
/// and append a level-up line for each actor whose level rose. This is the RM2000
/// `ProcessSceneActionVictory` per-actor `ChangeExp` beat; `rewarded` guards it so
/// it never double-pays while the outcome screen waits for the player.
pub(super) fn apply_victory_rewards(
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
pub(super) fn outcome_input(
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
mod tests;

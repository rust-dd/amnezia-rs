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
use amnezia_data::SystemDef;
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

/// The order-3 HUD camera the battle windows render on. It sits above the order-2
/// effect overlay (backdrop, battlers, animations) and the order-0 world. Its
/// [`RenderLayers`] points at the otherwise-unused layer 2 so its 2D pass draws no
/// world sprites — only the HUD, which `bevy_ui` composites by target camera, not
/// by render layer.
#[derive(Component)]
pub(crate) struct HudCamera;

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

/// Prepare the troop and current party roster before the scene controller
/// transitions from the map. Requests during another encounter are ignored.
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
    hero_name: Res<crate::text::HeroName>,
    current_bgm: Res<CurrentBgm>,
    mut map_bgm: ResMut<MapBgm>,
    mut audio: MessageWriter<AudioRequest>,
    mut flow: ResMut<super::BattleFlow>,
    mut active: ResMut<BattleActive>,
    mut result: ResMut<BattleResult>,
    mut dialogue: Option<ResMut<crate::dialogue::Dialogue>>,
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
    let actors = roster
        .iter()
        .filter_map(|id| data.actor(*id))
        .collect::<Vec<_>>();
    if actors.is_empty() {
        result.0 = Some(BattleOutcome::Defeat);
        return;
    }
    let equipped = actors
        .iter()
        .map(|a| equipment.slots(a))
        .collect::<Vec<_>>();
    let seed = seed_now() ^ (request.troop_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut battle = Battle::build(
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
    for (fighter, actor) in battle.members.iter_mut().zip(&actors) {
        fighter.name = hero_name.actor(actor).to_owned();
    }
    battle.allow_escape = request.allow_escape;
    battle.first_strike = request.first_strike;
    battle.text.apply(&terms.0);
    flow.enter(battle, request.defeat_ends_game);
    active.0 = true;
    result.0 = None;
    if let Some(dialogue) = dialogue.as_deref_mut() {
        dialogue.face = default();
    }
    let system = &battle_data.system;
    map_bgm.memorize(current_bgm.track());
    if let Some(se) = AudioRequest::se(
        &system.battle_se.name,
        system.battle_se.volume,
        system.battle_se.tempo,
    ) {
        audio.write(se);
    }
}

/// Opt-in sample fight; never interrupt a scripted scene.
pub(super) fn debug_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    active: Res<BattleActive>,
    scene: crate::world::ScenePause,
    running: Option<Res<crate::interpreter::RunningEvent>>,
    dialogue: Option<Res<crate::dialogue::Dialogue>>,
    mut requests: MessageWriter<BattleRequest>,
) {
    if !active.0
        && keys.just_pressed(KeyCode::F6)
        && crate::debug::tools_enabled()
        && !scene.paused()
        && !running.as_ref().is_some_and(|r| r.active())
        && !dialogue.as_ref().is_some_and(|d| d.active)
    {
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
    mut inventory: ResMut<Inventory>,
) {
    if battle.phase != Phase::Resolve || battle.events.blocks_action() {
        return;
    }
    // Hold the step while a slain foe plays out its death or explosion, so the
    // beat is seen before the next action lands (RM2000 `SetWait(36, 60)`).
    if battle.death_in_progress() {
        return;
    }
    // RPG_RT applies the impact only after its animation completes.
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
    let more = battle.resolve_next_with_items(|item_id| {
        if !inventory.has(item_id) {
            return false;
        }
        inventory.remove_item(item_id, 1);
        true
    });
    if battle.steps.is_empty() && !battle.anim_hold_active() {
        battle.events.check_pages();
    }
    if let Some(outcome) = battle.end_state()
        && battle.steps.is_empty()
        && !battle.anim_hold_active()
    {
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
pub(super) fn play_outcome_music(
    audio: &mut MessageWriter<AudioRequest>,
    system: &SystemDef,
    overrides: Option<&crate::system_bgm::SystemBgm>,
    outcome: BattleOutcome,
) {
    let (slot, music) = match outcome {
        BattleOutcome::Victory => (1, &system.battle_end_music),
        BattleOutcome::Defeat => (6, &system.gameover_music),
        BattleOutcome::Escape | BattleOutcome::Abort => return,
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
    // A short animation can finish before the next low-FPS resolution update.
    battle.anim_seen = true;
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
            sound_only: anim.sound_only,
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
    for id in battle.reward_items.clone() {
        inventory.add_item(id, 1);
        if let Some(item) = data.items.iter().find(|item| item.id == id) {
            let line = format!(
                "{}{}",
                crate::i18n::tr(&item.name),
                crate::i18n::tr(&battle.text.item_recieved)
            );
            battle.log.push(line);
        }
    }
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

pub(super) fn abort_expired_battle(
    clock: Option<Res<crate::timer::GameClock>>,
    mut battle: ResMut<Battle>,
) {
    if clock.is_some_and(|clock| clock.expired) && battle.phase != Phase::Inactive {
        battle.finish(BattleOutcome::Abort);
    }
}

/// Persist the confirmed outcome before the exit transitions. Rewards were
/// already paid on entering the outcome; the scene controller publishes the result.
pub(super) fn outcome_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<Battle>,
    mut flow: ResMut<super::BattleFlow>,
    mut vitals: ResMut<Vitals>,
    mut dialogue: Option<ResMut<crate::dialogue::Dialogue>>,
) {
    if battle.phase != Phase::Outcome || flow.busy() {
        return;
    }
    if battle.outcome != Some(BattleOutcome::Abort)
        && !(keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter))
    {
        return;
    }
    let outcome = battle.outcome.unwrap_or(BattleOutcome::Escape);
    if outcome != BattleOutcome::Abort && super::outcome_text::advance(&mut battle) {
        return;
    }
    if let Some(dialogue) = dialogue.as_deref_mut()
        && dialogue.active
    {
        dialogue.close();
    }
    for fighter in &battle.members {
        vitals.set(fighter.actor_id, fighter.hp.max(0), fighter.sp);
        let states = fighter
            .states
            .iter()
            .filter(|(id, _)| {
                battle
                    .states
                    .iter()
                    .any(|state| state.id == *id && state.persistence == 1)
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        vitals.set_states(fighter.actor_id, states);
    }
    flow.leave(outcome);
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

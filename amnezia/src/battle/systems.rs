//! Encounter startup, action resolution and sound/animation queue delivery.

use super::model::{Battle, BattleSe, Phase};
use super::scene;
use super::{BattleActive, BattleData, BattleOutcome, BattleRequest, BattleResult, MapBgm};
use crate::animation::{AnimAnchor, PlayAnimation};
use crate::audio::{AudioRequest, CurrentBgm};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::vitals::Vitals;
use amnezia_data::SystemDef;
use bevy::prelude::*;

/// The troop the debug key spawns: troop 2 "Rablo2x", two bandits.
const DEBUG_TROOP: u32 = 2;

/// Fallback backdrop for the developer's test encounter.
const BACKDROP: &str = "Cave1";

/// EasyRPG `BattleAnimationBattle::Draw` centres at (screen_w/2, screen_h/3),
/// or (0, -40) in the centre-origin y-down overlay.
const BATTLE_SCREEN_CENTER: Vec2 = Vec2::new(0.0, -40.0);

/// Front-view party targets have no bitmap, so use EasyRPG's unloaded-battler fallback.
const PARTY_TARGET_HEIGHT: f32 = 48.0;

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
    battle.begin_encounter();
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

pub(super) fn resolve_tick(
    battle_data: Res<BattleData>,
    animations: Option<Res<crate::animation::AnimationLibrary>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    system_bgm: Option<Res<crate::system_bgm::SystemBgm>>,
    mut world: super::events::EventWorld,
    mut battle: ResMut<Battle>,
    mut inventory: ResMut<Inventory>,
) {
    use super::resolve::timeline::Progress;
    if battle.phase != Phase::Resolve || battle.events.holds_resolution() {
        return;
    }
    let controls = keys
        .as_deref()
        .map(super::message::Controls::from_keys)
        .unwrap_or_default();
    loop {
        let progress = battle.advance_action(
            controls,
            |id| {
                animations
                    .as_ref()
                    .and_then(|library| library.0.iter().find(|a| a.id == id))
                    .map_or(0, |animation| animation.frames.len() as u32 * 2)
            },
            |id, consume| {
                if !inventory.has(id) {
                    return false;
                }
                if consume {
                    inventory.remove_item(id, 1);
                }
                true
            },
        );
        if progress == Progress::Waiting {
            return;
        }
        if let Some(outcome) = battle.end_state() {
            battle.finish(outcome);
            world.outcome_music(&battle_data.system, system_bgm.as_deref(), outcome);
            return;
        }
        if progress == Progress::Boundary && world.action_boundary(&mut battle) {
            return;
        }
        if progress == Progress::Done {
            battle.new_round();
            return;
        }
    }
}

pub(super) fn advance_deaths(mut battle: ResMut<Battle>) {
    if battle.death_in_progress() {
        battle.advance_deaths(1.0 / 60.0);
    }
}

/// Outcome music interrupts battle BGM until teardown restores the map track.
/// Successful escape has only an SE, not a fanfare.
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

/// Map queued roles to system sound effects; disabled entries stay silent.
/// Check emptiness before mutation to avoid triggering idle UI updates.
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
            BattleSe::UseItem => &system.item_se,
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

/// Attach measured target heights and the battle screen centre to queued casts.
/// Check emptiness before mutation to avoid triggering idle UI updates.
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
            map_target: None,
            slot: if anim.sound_only {
                crate::animation::AnimationSlot::Party
            } else {
                crate::animation::AnimationSlot::Enemies
            },
            anim_id: anim.anim_id,
            targets,
            screen_center: BATTLE_SCREEN_CENTER,
            global: false,
            sound_only: anim.sound_only,
        });
    }
}

pub(super) use super::outcome::{apply_victory_rewards, outcome_input};

pub(super) fn abort_expired_battle(
    clock: Option<Res<crate::timer::GameClock>>,
    mut battle: ResMut<Battle>,
) {
    if clock.is_some_and(|clock| clock.expired) && battle.phase != Phase::Inactive {
        battle.finish(BattleOutcome::Abort);
    }
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

use super::{Phase, State};
use crate::audio::{AudioRequest, CurrentBgm, SystemMusic};
use crate::choice::Choice;
use crate::dialogue::{Dialogue, MessagePrompt};
use crate::events::MessageBox;
use crate::inputnumber::InputNumber;
use crate::shop::{ShopOpen, ShopOutcome, ShopRequest, messages};
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::timing::GameFrames;
use crate::transitions::{Kind, Transition};
use crate::vitals::Vitals;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub(super) fn open(
    mut requests: MessageReader<ShopRequest>,
    mut state: ResMut<State>,
    mut open: ResMut<ShopOpen>,
    mut outcome: ResMut<ShopOutcome>,
    mut dialogue: ResMut<Dialogue>,
    mut choice: ResMut<Choice>,
    mut number: ResMut<InputNumber>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
) -> bool {
    let mut free = false;
    for request in requests.read() {
        let ShopRequest::ShowInn {
            cost,
            inn_type,
            foreground,
        } = request
        else {
            continue;
        };
        let cost = (*cost).max(0);
        free = cost == 0;
        *state = State {
            gold: inventory.gold(),
            ..default()
        };
        open.0 = cost == 0;
        outcome.transacted = false;
        if cost == 0 {
            state.phase = Phase::Closing;
            continue;
        }
        *choice = Choice::default();
        *number = InputNumber::default();
        let vocabulary = messages::inn_vocab(*inn_type, cost, &terms);
        let face = dialogue.face.graphic();
        let page = MessageBox {
            face: face.map(|(name, _)| name.to_string()),
            face_index: face.map_or(0, |(_, index)| index),
            lines: vocabulary.greetings.into(),
        };
        dialogue.open(vec![page]);
        dialogue.from_foreground = *foreground;
        dialogue.open_gold();
        dialogue.append_prompt(MessagePrompt::Choice {
            labels: vec![vocabulary.accept, vocabulary.cancel],
            indent: 0,
            cancel: 5,
        });
        if inventory.gold() < cost {
            dialogue.disable_choice(0);
        }
        state.phase = Phase::Prompt { cost };
    }
    free
}

#[derive(SystemParam)]
pub(super) struct Playback<'w, 's> {
    current: Res<'w, CurrentBgm>,
    music: Res<'w, SystemMusic>,
    overrides: Option<Res<'w, crate::system_bgm::SystemBgm>>,
    sinks: Query<'w, 's, &'static bevy::audio::AudioSink>,
    audio: MessageWriter<'w, AudioRequest>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance(
    mut state: ResMut<State>,
    mut open: ResMut<ShopOpen>,
    mut outcome: ResMut<ShopOutcome>,
    mut vitals: ResMut<Vitals>,
    party: Res<Party>,
    dialogue: Res<Dialogue>,
    frames: Res<GameFrames>,
    time: Res<Time<Real>>,
    mut transition: ResMut<Transition>,
    mut playback: Playback,
) {
    let finished = loop {
        break match state.phase {
            Phase::Idle | Phase::Prompt { .. } => return,
            Phase::Closing => {
                if dialogue.busy() || transition.busy() {
                    return;
                }
                open.0 = true;
                transition.start(Kind::Fade, true, frames.frame, IVec2::new(160, 120));
                state.phase = Phase::FadeOut {
                    started: time.elapsed(),
                };
                continue;
            }
            Phase::FadeOut { started } => {
                if transition.busy() {
                    return;
                }
                let request = AudioRequest::music_once(crate::system_bgm::resolve(
                    playback.overrides.as_deref(),
                    2,
                    &playback.music.inn,
                ));
                let silent = request == AudioRequest::StopBgm;
                if !silent {
                    playback.audio.write(request);
                    state.phase = Phase::Resting { started };
                }
                silent
            }
            Phase::Resting { started } => rest_finished(
                playback.current.playing_or_pending(&playback.sinks),
                time.elapsed().saturating_sub(started),
            ),
            Phase::FadeIn => {
                if !transition.busy() {
                    state.phase = Phase::Idle;
                    open.0 = false;
                }
                false
            }
        };
    };
    if finished {
        playback.audio.write(AudioRequest::StopBgm);
        playback.audio.write(
            state
                .before
                .take()
                .map_or(AudioRequest::StopBgm, |track| track.replay()),
        );
        for id in party.snapshot() {
            vitals.heal(id);
        }
        outcome.transacted = true;
        transition.event_erased = false;
        transition.start(Kind::Fade, false, frames.frame, IVec2::new(160, 120));
        state.phase = Phase::FadeIn;
    }
}

pub(super) fn begin_stay(world: &mut World) {
    crate::audio::flush(world);
    let before = world.resource::<CurrentBgm>().track();
    world.resource_mut::<State>().before = before;
    world.write_message(AudioRequest::FadeOutBgm { duration: 0.8 });
    // UpdateInn immediately revisits the already-closing message on acceptance.
    world.resource_mut::<Dialogue>().advance_inn_close();
    world.run_system_cached(advance).unwrap();
}

pub(super) fn accept(world: &mut World) {
    if world.run_system_cached(accept_result).unwrap() {
        begin_stay(world);
        crate::interpreter::continuation::suspend_message(world);
    }
}

fn accept_result(
    mut state: ResMut<State>,
    mut open: ResMut<ShopOpen>,
    mut inventory: ResMut<Inventory>,
    mut choice: ResMut<Choice>,
) -> bool {
    let Phase::Prompt { cost } = state.phase else {
        return false;
    };
    let Some(result) = choice.result.take() else {
        return false;
    };
    if result != 0 {
        state.phase = Phase::Idle;
        open.0 = false;
    } else {
        inventory.remove_gold(cost);
        state.phase = Phase::Closing;
    }
    result == 0
}

pub(super) fn rest_finished(playing: bool, elapsed: Duration) -> bool {
    !playing || elapsed >= Duration::from_secs(10)
}

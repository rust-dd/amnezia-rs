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
) {
    for request in requests.read() {
        let ShopRequest::ShowInn { cost, inn_type } = request else {
            continue;
        };
        let cost = (*cost).max(0);
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
    mut inventory: ResMut<Inventory>,
    mut vitals: ResMut<Vitals>,
    party: Res<Party>,
    dialogue: Res<Dialogue>,
    mut choice: ResMut<Choice>,
    frames: Res<GameFrames>,
    time: Res<Time<Real>>,
    mut transition: ResMut<Transition>,
    mut playback: Playback,
) {
    accept_result(&mut state, &mut open, &mut inventory, &mut choice);
    let finished = match state.phase {
        Phase::Idle | Phase::Prompt { .. } => return,
        Phase::Closing => {
            if dialogue.busy() || transition.busy() {
                return;
            }
            open.0 = true;
            state.before = playback.current.track();
            playback
                .audio
                .write(AudioRequest::FadeOutBgm { duration: 0.8 });
            transition.start(Kind::Fade, true, frames.frame, IVec2::new(160, 120));
            state.phase = Phase::FadeOut {
                started: time.elapsed(),
            };
            false
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

pub(super) fn accept(
    mut state: ResMut<State>,
    mut open: ResMut<ShopOpen>,
    mut inventory: ResMut<Inventory>,
    mut choice: ResMut<Choice>,
) {
    accept_result(&mut state, &mut open, &mut inventory, &mut choice);
}

fn accept_result(
    state: &mut State,
    open: &mut ShopOpen,
    inventory: &mut Inventory,
    choice: &mut Choice,
) {
    let Phase::Prompt { cost } = state.phase else {
        return;
    };
    let Some(result) = choice.result.take() else {
        return;
    };
    if result != 0 {
        state.phase = Phase::Idle;
        open.0 = false;
    } else {
        inventory.remove_gold(cost);
        state.phase = Phase::Closing;
    }
}

pub(super) fn rest_finished(playing: bool, elapsed: Duration) -> bool {
    !playing || elapsed >= Duration::from_secs(10)
}

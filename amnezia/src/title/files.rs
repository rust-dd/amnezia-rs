use super::{AudioRequest, SystemSounds, TitleActive, TitleState, flow::Stage, play_se};
use crate::gamedata::GameData;
use crate::menu::{MenuOpen, save_files::SaveFiles};
use crate::save::{LoadOutcome, LoadRequest, SaveLocation};
use crate::transitions::{Kind, TransitionIo};
use bevy::prelude::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    title: Res<TitleActive>,
    data: Res<GameData>,
    location: Res<SaveLocation>,
    mut state: ResMut<TitleState>,
    mut files: ResMut<SaveFiles>,
    mut open: ResMut<MenuOpen>,
    mut transition: TransitionIo,
    mut load: ResMut<LoadRequest>,
    mut outcome: ResMut<LoadOutcome>,
    music: Option<Res<crate::audio::SystemMusic>>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !title.0 || transition.state.busy() {
        return;
    }
    let now = transition.frames.frame;
    match state.stage {
        Stage::Files if !files.active() => {
            files.open_load(&location, &data, now);
            open.0 = true;
            show(&mut transition);
        }
        Stage::Files => {
            if let Some(load) = files.decision() {
                if load {
                    audio.write(AudioRequest::FadeOutBgm { duration: 0.8 });
                }
                transition
                    .state
                    .start_for(Kind::Fade, true, now, IVec2::new(160, 120), 6);
                state.stage = Stage::FileLeaving(load);
            }
        }
        Stage::FileLeaving(load_selected) => {
            open.0 = false;
            if load_selected {
                files.suspend();
                load.0 = true;
                outcome.0 = None;
                state.stage = Stage::FileLoading;
            } else {
                *files = default();
                show(&mut transition);
                state.stage = Stage::FileReturning;
            }
        }
        Stage::FileRetry => {
            files.reject_load(now);
            if let Some(music) = music {
                audio.write(AudioRequest::from_music(&music.title));
            }
            play_se(&mut audio, sounds.as_deref(), |sounds| &sounds.buzzer);
            open.0 = true;
            show(&mut transition);
            state.stage = Stage::Files;
        }
        Stage::FileReturning => state.stage = Stage::Ready,
        _ => {}
    }
}

fn show(transition: &mut TransitionIo) {
    transition.state.event_erased = false;
    transition.state.start_for(
        Kind::Fade,
        false,
        transition.frames.frame,
        IVec2::new(160, 120),
        6,
    );
}

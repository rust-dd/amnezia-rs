use super::*;
use crate::audio::SystemMusic;
use crate::save::{LoadOutcome, LoadRequest, SaveLocation};
use crate::session::NewGameRequest;
use crate::teleport::{Fade, PendingTeleport};
use crate::transitions::{Kind, Transition, TransitionIo};
use crate::world::MapChanged;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    #[default]
    Inactive,
    Enter,
    Wait(u32),
    Prepare,
    Showing,
    Ready,
    Leaving(TitleAction),
    Loading,
    Files,
    FileLeaving(bool),
    FileLoading,
    FileRetry,
    FileReturning,
}

impl Stage {
    pub(super) fn visible(self) -> bool {
        matches!(
            self,
            Self::Showing | Self::Ready | Self::Leaving(_) | Self::FileReturning
        )
    }

    pub(super) fn suspended(self) -> bool {
        matches!(
            self,
            Self::Files | Self::FileLeaving(_) | Self::FileLoading | Self::FileRetry
        )
    }
}

pub(super) fn entered(
    title: Res<TitleActive>,
    location: Res<SaveLocation>,
    mut state: ResMut<TitleState>,
    mut transition: TransitionIo,
    mut was_active: Local<bool>,
    mut visited: Local<bool>,
) {
    let entered = title.0 && (!*was_active || state.stage == Stage::Inactive);
    *was_active = title.0;
    if !entered {
        return;
    }
    state.cursor = default_cursor(location.has_saves());
    if !*visited {
        transition.state.hold_black();
        state.stage = Stage::Prepare;
        *visited = true;
    } else {
        let now = transition.frames.frame;
        transition.state.erase_previous(now, 6);
        state.stage = Stage::Enter;
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn input(
    keys: Res<ButtonInput<KeyCode>>,
    directions: Res<crate::menu::DirectionInput>,
    location: Res<SaveLocation>,
    title: Res<TitleActive>,
    mut state: ResMut<TitleState>,
    mut transition: ResMut<Transition>,
    mut frames: ResMut<crate::timing::GameFrames>,
    mut scene_frames: Option<ResMut<crate::timing::SceneFrames>>,
    mut new_game: ResMut<NewGameRequest>,
    mut audio: MessageWriter<AudioRequest>,
    sounds: Option<Res<SystemSounds>>,
) {
    if !title.0 || state.stage != Stage::Ready || transition.busy() {
        return;
    }
    let sounds = sounds.as_deref();
    for repeated in directions.slot_steps() {
        for (action, pressed) in repeated.into_iter().enumerate() {
            if !pressed {
                continue;
            }
            state.cursor = match action {
                0 => wrap_cursor(state.cursor, 1, ROWS.len()),
                1 => wrap_cursor(state.cursor, -1, ROWS.len()),
                4 if state.cursor < ROWS.len() - 1 => ROWS.len() - 1,
                5 if state.cursor > 0 => 0,
                _ => continue,
            };
            play_se(&mut audio, sounds, |s| &s.cursor);
        }
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        let action = action_for(state.cursor, location.has_saves());
        if action == TitleAction::ContinueDisabled {
            play_se(&mut audio, sounds, |s| &s.buzzer);
            return;
        }
        play_se(&mut audio, sounds, |s| &s.decision);
        if action == TitleAction::NewGame {
            audio.write(AudioRequest::FadeOutBgm { duration: 0.8 });
            new_game.prepare_clock(&mut frames);
            if let Some(scene_frames) = scene_frames.as_deref_mut() {
                *scene_frames = default();
            }
        }
        let duration = if action == TitleAction::Shutdown {
            35
        } else {
            6
        };
        transition.start_for(
            Kind::Fade,
            true,
            frames.frame,
            IVec2::new(160, 120),
            duration,
        );
        state.stage = Stage::Leaving(action);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn drive(
    title: Res<TitleActive>,
    mut state: ResMut<TitleState>,
    mut transition: TransitionIo,
    mut new_game: ResMut<NewGameRequest>,
    music: Option<Res<SystemMusic>>,
    mut audio: MessageWriter<AudioRequest>,
    mut exit: MessageWriter<AppExit>,
) {
    if !title.0 || transition.state.busy() {
        return;
    }
    let now = transition.frames.frame;
    if state.stage == Stage::Prepare
        || matches!(state.stage, Stage::Wait(until) if now.wrapping_sub(until) < u32::MAX / 2)
    {
        if let Some(music) = music {
            audio.write(AudioRequest::from_music(&music.title));
        }
        transition.state.event_erased = false;
        transition
            .state
            .start(Kind::Fade, false, now, IVec2::new(160, 120));
        state.stage = Stage::Showing;
        return;
    }
    match state.stage {
        Stage::Enter => state.stage = Stage::Wait(now.wrapping_add(20)),
        Stage::Showing => state.stage = Stage::Ready,
        Stage::Leaving(TitleAction::NewGame) => {
            new_game.requested = true;
            state.stage = Stage::Loading;
        }
        Stage::Leaving(TitleAction::Continue) => {
            state.stage = Stage::Files;
        }
        Stage::Leaving(TitleAction::Shutdown) => {
            exit.write(AppExit::Success);
            state.stage = Stage::Inactive;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn loaded(
    mut title: ResMut<TitleActive>,
    mut state: ResMut<TitleState>,
    load_request: Res<LoadRequest>,
    fade: Res<Fade>,
    pending: Res<PendingTeleport>,
    new_game: Res<NewGameRequest>,
    mut outcome: ResMut<LoadOutcome>,
    mut map_changed: MessageReader<MapChanged>,
) {
    if !matches!(state.stage, Stage::Loading | Stage::FileLoading) {
        map_changed.clear();
        return;
    }
    let swapped = !map_changed.is_empty();
    map_changed.clear();
    if outcome.0.take() == Some(false) {
        state.stage = if state.stage == Stage::FileLoading {
            Stage::FileRetry
        } else {
            Stage::Prepare
        };
        return;
    }
    let settled = !load_request.0 && !new_game.requested && !fade.busy() && pending.0.is_none();
    if swapped || settled {
        state.stage = Stage::Inactive;
        title.0 = false;
    }
}

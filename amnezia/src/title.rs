//! Title command selection and scene handoffs share the original transition clock.

mod flow;
pub(crate) mod smoke;
mod view;

#[cfg(test)]
mod tests;

use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use amnezia_data::SoundDef;
use bevy::prelude::*;

#[derive(Resource)]
pub struct TitleActive(pub bool);

impl Default for TitleActive {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource, Default)]
struct TitleState {
    cursor: usize,
    stage: flow::Stage,
}

const ROWS: [&str; 3] = ["Új játék", "Betöltés", "Kilépés"];
const NEW_GAME: usize = 0;
const CONTINUE: usize = 1;
const SHUTDOWN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitleAction {
    NewGame,
    Continue,
    ContinueDisabled,
    Shutdown,
}

fn action_for(cursor: usize, has_save: bool) -> TitleAction {
    match cursor {
        CONTINUE if has_save => TitleAction::Continue,
        CONTINUE => TitleAction::ContinueDisabled,
        SHUTDOWN => TitleAction::Shutdown,
        _ => TitleAction::NewGame,
    }
}

fn default_cursor(has_save: bool) -> usize {
    if has_save { CONTINUE } else { NEW_GAME }
}

pub struct TitlePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TitleFlow;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleActive>()
            .init_resource::<TitleState>()
            .add_systems(Startup, view::spawn)
            .add_systems(
                Update,
                (
                    flow::entered,
                    flow::loaded,
                    flow::input,
                    flow::drive,
                    view::update,
                )
                    .chain()
                    .in_set(TitleFlow)
                    .after(crate::menu::MenuInput)
                    .after(crate::gameover::GameOverFlowSet),
            );
    }
}

fn play_se(
    audio: &mut MessageWriter<AudioRequest>,
    sounds: Option<&SystemSounds>,
    pick: impl FnOnce(&SystemSounds) -> &SoundDef,
) {
    if let Some(sounds) = sounds {
        play_system_se(audio, pick(sounds));
    }
}

fn row_text(label: &str, selected: bool) -> String {
    let marker = if selected { "▶ " } else { "  " };
    format!("{marker}{label}")
}

fn wrap_cursor(cursor: usize, delta: i32, len: usize) -> usize {
    (cursor as i32 + delta).rem_euclid(len as i32) as usize
}

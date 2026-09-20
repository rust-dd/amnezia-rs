//! Game Over uses its original 80-frame fades and keeps confirmation out of handoffs.

pub(crate) mod smoke;
mod view;

#[cfg(test)]
mod tests;

use crate::audio::{AudioRequest, SystemMusic};
use crate::title::TitleActive;
use crate::transitions::{Kind, TransitionIo};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameOverActive(pub bool);

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    #[default]
    Inactive,
    Erasing,
    Prepare,
    Revealing,
    Showing,
    Leaving,
}

#[derive(Resource, Default)]
pub(crate) struct GameOverFlow(Stage);

impl GameOverFlow {
    pub(crate) fn waiting_for_scene(&self) -> bool {
        self.0 != Stage::Showing
    }

    pub(crate) fn prepare_from_battle(&mut self) {
        self.0 = Stage::Prepare;
    }

    fn visible(&self) -> bool {
        matches!(self.0, Stage::Revealing | Stage::Showing | Stage::Leaving)
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GameOverFlowSet;

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameOverActive>()
            .init_resource::<GameOverFlow>()
            .add_systems(Startup, view::spawn)
            .add_systems(
                Update,
                (drive, view::update)
                    .chain()
                    .in_set(GameOverFlowSet)
                    .after(crate::battle::flow::BattleFlowSet),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn drive(
    keys: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<GameOverActive>,
    mut flow: ResMut<GameOverFlow>,
    mut title: ResMut<TitleActive>,
    mut transition: TransitionIo,
    mut audio: MessageWriter<AudioRequest>,
    music: Option<Res<SystemMusic>>,
    overrides: Option<Res<crate::system_bgm::SystemBgm>>,
) {
    if transition.state.busy() {
        return;
    }
    let now = transition.frames.frame;
    let center = IVec2::new(160, 120);
    if matches!(flow.0, Stage::Erasing | Stage::Prepare)
        || (flow.0 == Stage::Inactive && active.0 && transition.state.erased())
    {
        if let Some(music) = music {
            audio.write(AudioRequest::from_music(crate::system_bgm::resolve(
                overrides.as_deref(),
                6,
                &music.gameover,
            )));
        }
        transition.state.event_erased = false;
        transition
            .state
            .start_for(Kind::Fade, false, now, center, 80);
        flow.0 = Stage::Revealing;
        return;
    }
    match flow.0 {
        Stage::Inactive if active.0 => {
            transition.state.start(Kind::Fade, true, now, center);
            transition.state.event_erased = false;
            flow.0 = Stage::Erasing;
        }
        Stage::Revealing => flow.0 = Stage::Showing,
        Stage::Showing => {
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                transition
                    .state
                    .start_for(Kind::Fade, true, now, center, 80);
                flow.0 = Stage::Leaving;
            }
        }
        Stage::Leaving => {
            title.0 = true;
            active.0 = false;
            flow.0 = Stage::Inactive;
        }
        _ => {}
    }
}

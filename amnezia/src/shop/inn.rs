use crate::audio::BgmTrack;
use bevy::prelude::*;
use std::time::Duration;

mod flow;
pub(crate) mod smoke;
#[cfg(test)]
mod tests;
mod view;

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Prompt {
        cost: i32,
    },
    Closing,
    FadeOut {
        started: Duration,
    },
    Resting {
        started: Duration,
    },
    FadeIn,
}

#[derive(Resource, Default)]
pub(crate) struct State {
    phase: Phase,
    before: Option<BgmTrack>,
    gold: i32,
}

impl State {
    pub(crate) fn active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    pub(crate) fn prompting(&self) -> bool {
        matches!(self.phase, Phase::Prompt { .. })
    }

    pub(crate) fn resting(&self) -> bool {
        !matches!(self.phase, Phase::Idle | Phase::Prompt { .. })
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Startup, view::spawn)
        .add_systems(
            Update,
            flow::open
                .after(crate::interpreter::InterpreterStep)
                .before(crate::dialogue::PromptInput),
        )
        .add_systems(
            Update,
            (flow::advance, view::update)
                .chain()
                .after(crate::dialogue::MessageUpdate)
                .after(crate::audio::AudioRequests),
        );
}

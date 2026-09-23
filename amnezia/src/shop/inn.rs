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
        !matches!(
            self.phase,
            Phase::Idle | Phase::Prompt { .. } | Phase::Closing
        )
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Startup, view::spawn)
        .add_systems(
            Update,
            (flow::open, flow::advance)
                .chain()
                .after(crate::interpreter::InterpreterStep)
                .after(crate::audio::AudioRequests)
                .before(crate::dialogue::DialogueView),
        )
        .add_systems(
            Update,
            flow::accept
                .after(crate::dialogue::MessageUpdate)
                .before(crate::interpreter::InterpreterStep),
        )
        .add_systems(Update, view::update.after(crate::dialogue::DialogueView));
}

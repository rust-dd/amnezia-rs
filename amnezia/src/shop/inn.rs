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
    pub(crate) fn cancel_prompt(&mut self) {
        if self.prompting() {
            self.phase = Phase::Idle;
        }
    }

    pub(crate) fn active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    pub(crate) fn prompting(&self) -> bool {
        matches!(self.phase, Phase::Prompt { .. })
    }

    pub(crate) fn resting(&self) -> bool {
        !matches!(self.phase, Phase::Idle | Phase::Prompt { .. })
    }

    pub(crate) fn closing(&self) -> bool {
        matches!(self.phase, Phase::Closing)
    }
}

pub(crate) fn open_pending(world: &mut World) {
    if world.contains_resource::<State>()
        && world.contains_resource::<crate::terms::Terms>()
        && world.run_system_cached(flow::open).unwrap()
    {
        flow::begin_stay(world);
    }
}

pub(crate) fn start_free(world: &mut World) {
    let gold = world.resource::<crate::state::Inventory>().gold();
    *world.resource_mut::<State>() = State {
        phase: Phase::Closing,
        gold,
        ..default()
    };
    world.resource_mut::<crate::shop::ShopOpen>().0 = true;
    world.resource_mut::<crate::shop::ShopOutcome>().transacted = false;
    flow::begin_stay(world);
}

pub(super) fn register(app: &mut App) {
    register_flow(app);
    app.add_systems(Startup, view::spawn);
    crate::dialogue::presentation::register(
        app,
        crate::dialogue::presentation::Stage::Inn,
        view::update,
    );
}

fn register_flow(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(
            Update,
            open_pending
                .after(crate::interpreter::InterpreterStep)
                .before(crate::dialogue::DialogueView),
        )
        .add_systems(
            Update,
            flow::advance
                .before(crate::teleport::MapTransfer)
                .before(crate::interpreter::ParallelStep),
        )
        .add_systems(
            Update,
            flow::accept
                .after(crate::dialogue::MessageUpdate)
                .before(crate::timer::ClockTick)
                .before(crate::screenfx::ScreenAdvance)
                .before(crate::animation::AnimationSet::Advance)
                .before(crate::interpreter::InterpreterStep),
        );
}

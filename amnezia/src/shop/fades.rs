use super::{Screen, ShopOpen, ShopState, ShopUpdate};
use crate::gamedata::GameData;
use crate::state::Inventory;
use crate::timing::GameFrames;
use crate::transitions::{Kind, Transition};
use bevy::prelude::*;

#[derive(Default)]
enum Stage {
    #[default]
    Idle,
    EnterRequested(Box<ShopState>),
    ErasingToShop(Box<ShopState>),
    ExitRequested,
    ErasingToMap,
    Showing,
}

#[derive(Resource, Default)]
pub(crate) struct Flow(Stage);

impl Flow {
    pub(crate) fn active(&self) -> bool {
        !matches!(self.0, Stage::Idle)
    }

    pub(super) fn enter(&mut self, state: Box<ShopState>) {
        self.0 = Stage::EnterRequested(state);
    }

    pub(super) fn leave(&mut self) {
        self.0 = Stage::ExitRequested;
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<Flow>()
        .init_resource::<Transition>()
        .add_systems(
            Update,
            advance
                .after(crate::interpreter::InterpreterStep)
                .before(super::flow::open_requests)
                .in_set(ShopUpdate),
        )
        .add_systems(Update, begin.after(ShopUpdate));
}

fn begin(mut flow: ResMut<Flow>, frames: Res<GameFrames>, mut transition: ResMut<Transition>) {
    if !matches!(flow.0, Stage::EnterRequested(_) | Stage::ExitRequested) {
        return;
    }
    transition.event_erased = false;
    if !transition.start_for(Kind::Fade, true, frames.frame, IVec2::new(160, 120), 6) {
        return;
    }
    flow.0 = match std::mem::take(&mut flow.0) {
        Stage::EnterRequested(state) => Stage::ErasingToShop(state),
        Stage::ExitRequested => Stage::ErasingToMap,
        _ => unreachable!(),
    };
}

fn advance(
    mut flow: ResMut<Flow>,
    frames: Res<GameFrames>,
    mut transition: ResMut<Transition>,
    mut screen: ResMut<Screen>,
    mut open: ResMut<ShopOpen>,
    data: Res<GameData>,
    inventory: Res<Inventory>,
) {
    if transition.busy() {
        return;
    }
    match std::mem::take(&mut flow.0) {
        Stage::ErasingToShop(mut state) => {
            super::scene::initialize(&mut state, &data, &inventory);
            *screen = Screen::Shop(state);
            open.0 = true;
        }
        Stage::ErasingToMap => {
            *screen = Screen::Closed;
            open.0 = false;
        }
        Stage::Showing => return,
        stage => {
            flow.0 = stage;
            return;
        }
    }
    transition.start_for(Kind::Fade, false, frames.frame, IVec2::new(160, 120), 6);
    flow.0 = Stage::Showing;
}

#[cfg(test)]
mod tests;

pub(super) mod smoke;

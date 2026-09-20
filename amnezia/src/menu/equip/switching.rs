use crate::menu::{MenuScreen, MenuState};
use crate::timing::GameFrames;
use crate::transitions::{Kind, Transition};
use bevy::prelude::*;

#[derive(Default)]
enum Stage {
    #[default]
    Idle,
    Requested(MenuScreen),
    Erasing(MenuScreen),
    Showing,
}

#[derive(Resource, Default)]
pub(crate) struct Switch {
    stage: Stage,
}

impl Switch {
    pub(in crate::menu) fn request(&mut self, screen: MenuScreen) {
        self.stage = Stage::Requested(screen);
    }

    pub(in crate::menu) fn active(&self) -> bool {
        !matches!(self.stage, Stage::Idle)
    }
}

pub(in crate::menu) fn register(app: &mut App) {
    app.init_resource::<Switch>().add_systems(
        Update,
        (
            advance.before(crate::menu::list_navigation::update_input),
            begin
                .after(crate::menu::input::menu_input)
                .before(super::refresh_actor),
        )
            .in_set(crate::menu::MenuInput),
    );
}

fn begin(mut switch: ResMut<Switch>, frames: Res<GameFrames>, mut transition: ResMut<Transition>) {
    let Stage::Requested(screen) = switch.stage else {
        return;
    };
    if transition.start_for(Kind::Fade, true, frames.frame, IVec2::new(160, 120), 6) {
        switch.stage = Stage::Erasing(screen);
    }
}

fn advance(
    mut switch: ResMut<Switch>,
    frames: Res<GameFrames>,
    mut transition: ResMut<Transition>,
    mut state: ResMut<MenuState>,
) {
    if transition.busy() {
        return;
    }
    match switch.stage {
        Stage::Erasing(screen) => {
            state.screen = screen;
            transition.start_for(Kind::Fade, false, frames.frame, IVec2::new(160, 120), 6);
            switch.stage = Stage::Showing;
        }
        Stage::Showing => switch.stage = Stage::Idle,
        _ => {}
    }
}

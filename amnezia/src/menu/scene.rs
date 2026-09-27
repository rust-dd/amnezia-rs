use super::{MenuInput, MenuOpen, MenuScreen, MenuState, MenuView, equip, input};
use crate::timing::{GameFrames, SceneWait};
use crate::transitions::{Kind, Transition};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Snapshot {
    open: bool,
    screen: MenuScreen,
    cursor: usize,
}

pub(in crate::menu) mod smoke;

impl Snapshot {
    fn capture(open: &MenuOpen, state: &MenuState) -> Self {
        Self {
            open: open.0,
            screen: state.screen,
            cursor: state.cursor,
        }
    }

    fn restore(self, open: &mut MenuOpen, state: &mut MenuState) {
        open.0 = self.open;
        state.screen = self.screen;
        state.cursor = self.cursor;
    }

    fn scene(self) -> u8 {
        if !self.open {
            return 0;
        }
        match self.screen {
            MenuScreen::Command | MenuScreen::MemberSelect { .. } => 1,
            MenuScreen::ItemList { .. } => 2,
            MenuScreen::ItemTarget { .. } => 3,
            MenuScreen::SkillList { .. } => 4,
            MenuScreen::SkillTarget { .. } => 5,
            MenuScreen::Equip { .. } => 6,
            MenuScreen::Status { .. } => 7,
            MenuScreen::EndGame { .. } => 8,
        }
    }
}

#[derive(Default)]
enum Stage {
    #[default]
    Idle,
    Requested(Snapshot),
    Erasing(Snapshot),
    Showing,
}

#[derive(Resource, Default)]
pub(crate) struct Flow {
    before: Option<Snapshot>,
    stage: Stage,
}

impl Flow {
    pub(crate) fn active(&self) -> bool {
        !matches!(self.stage, Stage::Idle)
    }

    pub(crate) fn requested(&self) -> bool {
        matches!(self.stage, Stage::Requested(_))
    }

    pub(crate) fn blocks_map(&self) -> bool {
        self.active() && !self.requested()
    }

    pub(crate) fn request_main_menu(&mut self) {
        self.stage = Stage::Requested(Snapshot {
            open: true,
            screen: MenuScreen::Command,
            cursor: 0,
        });
    }
}

#[derive(SystemParam)]
pub(in crate::menu) struct Pause<'w> {
    transition: crate::transitions::TransitionPause<'w>,
    frame: Option<Res<'w, SceneWait>>,
}

impl Pause<'_> {
    pub(in crate::menu) fn paused(&self) -> bool {
        self.transition.paused() || self.frame.as_ref().is_some_and(|frame| frame.0)
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<Flow>()
        .init_resource::<Transition>()
        .add_systems(
            Update,
            (
                advance.before(super::list_navigation::update_input),
                remember
                    .after(super::command_navigation::update)
                    .before(input::menu_input),
                request
                    .after(input::menu_input)
                    .before(equip::refresh_actor),
            )
                .in_set(MenuInput),
        )
        .add_systems(
            Update,
            begin
                .after(MenuView)
                .after(crate::interpreter::scenes::Commit),
        );
}

fn remember(mut flow: ResMut<Flow>, open: Res<MenuOpen>, state: Res<MenuState>) {
    flow.before = Some(Snapshot::capture(&open, &state));
}

fn request(
    mut flow: ResMut<Flow>,
    mut open: ResMut<MenuOpen>,
    mut state: ResMut<MenuState>,
    title: Res<crate::title::TitleActive>,
    files: Res<super::save_files::SaveFiles>,
    transition: Res<Transition>,
) {
    let Some(before) = flow.before.take() else {
        return;
    };
    if flow.active() || title.0 || files.active() || transition.busy() {
        return;
    }
    let next = Snapshot::capture(&open, &state);
    if before.scene() == next.scene() {
        return;
    }
    before.restore(&mut open, &mut state);
    flow.stage = Stage::Requested(next);
}

fn begin(mut flow: ResMut<Flow>, frames: Res<GameFrames>, mut transition: ResMut<Transition>) {
    let Stage::Requested(next) = flow.stage else {
        return;
    };
    transition.event_erased = false;
    if transition.start_for(Kind::Fade, true, frames.frame, IVec2::new(160, 120), 6) {
        flow.stage = Stage::Erasing(next);
    }
}

fn advance(
    mut flow: ResMut<Flow>,
    frames: Res<GameFrames>,
    mut transition: ResMut<Transition>,
    mut open: ResMut<MenuOpen>,
    mut state: ResMut<MenuState>,
) {
    if transition.busy() {
        return;
    }
    match flow.stage {
        Stage::Erasing(next) => {
            next.restore(&mut open, &mut state);
            transition.start_for(Kind::Fade, false, frames.frame, IVec2::new(160, 120), 6);
            flow.stage = Stage::Showing;
        }
        Stage::Showing => flow.stage = Stage::Idle,
        _ => {}
    }
}

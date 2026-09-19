use super::{Mode, SaveFiles};
use crate::gamedata::GameData;
use crate::menu::MenuOpen;
use crate::save::{EventSaveRequest, SaveLocation, SaveRequest};
use crate::teleport::Fade;
use crate::timing::GameFrames;
use crate::transitions::{Kind, Transition};
use bevy::prelude::*;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    #[default]
    Inactive,
    LeavingParent,
    ShowingList,
    Ready,
    AwaitingWrite,
    Writing,
    LeavingList,
    Returning,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    mut files: ResMut<SaveFiles>,
    mut open: ResMut<MenuOpen>,
    mut event_save: ResMut<EventSaveRequest>,
    mut transition: ResMut<Transition>,
    frames: Res<GameFrames>,
    fade: Res<Fade>,
    data: Res<GameData>,
    location: Res<SaveLocation>,
) {
    if files.mode != Mode::Save || fade.busy() || transition.busy() {
        return;
    }
    let now = frames.frame;
    if files.requested && files.stage == Stage::Inactive {
        transition.event_erased = false;
        transition.start_for(Kind::Fade, true, now, IVec2::new(160, 120), 6);
        files.stage = Stage::LeavingParent;
        if transition.busy() {
            return;
        }
    }
    match files.stage {
        Stage::LeavingParent => {
            files.prepare(&location, &data, now);
            open.0 = true;
            show(&mut transition, now);
            files.stage = Stage::ShowingList;
        }
        Stage::ShowingList => files.stage = Stage::Ready,
        Stage::Ready => match files.finished {
            Some(true) => files.stage = Stage::AwaitingWrite,
            Some(false) => {
                leave(&mut files, &mut transition, now);
            }
            None => {}
        },
        Stage::Writing => leave(&mut files, &mut transition, now),
        Stage::LeavingList => {
            files.entries = None;
            open.0 = files.event_menu.unwrap_or(true);
            show(&mut transition, now);
            files.stage = Stage::Returning;
        }
        Stage::Returning => {
            event_save.0 = false;
            *files = default();
        }
        _ => {}
    }
}

pub(super) fn authorize_write(
    mut files: ResMut<SaveFiles>,
    mut save: ResMut<SaveRequest>,
    transition: Res<Transition>,
    fade: Res<Fade>,
) {
    if files.stage == Stage::AwaitingWrite && !transition.busy() && !fade.busy() {
        if files.event_menu.is_none() {
            save.0 = true;
        }
        files.stage = Stage::Writing;
    }
}

fn leave(files: &mut SaveFiles, transition: &mut Transition, now: u32) {
    // The still-visible list includes the decision frame's final cursor and arrow update.
    transition.start_for(Kind::Fade, true, now, IVec2::new(160, 120), 6);
    files.stage = Stage::LeavingList;
}

fn show(transition: &mut Transition, now: u32) {
    transition.start_for(Kind::Fade, false, now, IVec2::new(160, 120), 6);
}

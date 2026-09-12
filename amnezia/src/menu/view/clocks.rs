use super::{CursorId, MenuCursor, MenuOpen, MenuScreen, MenuState};
use crate::timing::GameFrames;
use bevy::prelude::*;

#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
pub(in crate::menu) struct Clock {
    last: Option<u32>,
    open: bool,
    active: Option<CursorId>,
    phases: [u32; 2],
}

impl Clock {
    fn advance(&mut self, now: u32, open: bool, screen: MenuScreen, paused: bool) {
        let delta = self
            .last
            .replace(now)
            .map_or(0, |last| now.wrapping_sub(last));
        if !open || !self.open {
            self.phases = [0; 2];
        } else if !paused && let Some(active) = self.active {
            // RPG2000 updates the old active window before processing a focus change.
            let phase = &mut self.phases[active as usize];
            *phase = (*phase + delta % 21) % 21;
        }
        self.open = open;
        self.active = match screen {
            MenuScreen::Command if open => Some(CursorId::Command),
            MenuScreen::MemberSelect { .. } if open => Some(CursorId::Status),
            _ => None,
        };
    }

    pub(super) fn source_x(&self, cursor: CursorId) -> f32 {
        if self.phases.get(cursor as usize).copied().unwrap_or(0) <= 10 {
            64.0
        } else {
            96.0
        }
    }
}

pub(in crate::menu) fn update(
    frames: Res<GameFrames>,
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    pause: crate::transitions::TransitionPause,
    mut clock: ResMut<Clock>,
    cursors: Query<(&MenuCursor, &Children)>,
    mut images: Query<&mut ImageNode>,
) {
    clock.advance(frames.frame, open.0, state.screen, pause.paused());
    for (cursor, pieces) in &cursors {
        if matches!(cursor.0, CursorId::Content) {
            continue;
        }
        for piece in pieces {
            if let Ok(mut image) = images.get_mut(*piece) {
                crate::windowskin::cursor_phase(&mut image, clock.source_x(cursor.0));
            }
        }
    }
}

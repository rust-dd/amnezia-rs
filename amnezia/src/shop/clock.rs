use super::{Phase, ShopState, logic};
use crate::gamedata::GameData;
use crate::state::Inventory;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub(super) const CONFIRM_FRAMES: u32 = 60;

#[derive(Default)]
pub(super) struct Clock(Option<u32>);

impl Clock {
    pub(super) fn advance(&mut self, now: u32) -> u32 {
        let elapsed = self.0.replace(now).map_or(0, |last| now.wrapping_sub(last));
        if elapsed > i32::MAX as u32 {
            0
        } else {
            elapsed
        }
    }
}

#[derive(SystemParam)]
pub(super) struct Pause<'w> {
    frame: Option<Res<'w, crate::timing::SceneWait>>,
    transition: crate::transitions::TransitionPause<'w>,
    fade: Option<Res<'w, crate::teleport::Fade>>,
}

impl Pause<'_> {
    pub(super) fn paused(&self) -> bool {
        self.frame.as_ref().is_some_and(|frame| frame.0)
            || self.transition.paused()
            || self.fade.as_ref().is_some_and(|fade| fade.busy())
    }
}

pub(super) fn confirmation(
    state: &mut ShopState,
    ticks: u32,
    data: &GameData,
    inventory: &Inventory,
) -> bool {
    let phase = match &mut state.phase {
        Phase::Bought { remaining, cursor } => {
            *remaining = remaining.saturating_sub(ticks);
            (*remaining == 0).then_some(Phase::Buy { cursor: *cursor })
        }
        Phase::Sold { remaining, cursor } => {
            *remaining = remaining.saturating_sub(ticks);
            (*remaining == 0).then(|| Phase::Sell {
                cursor: (*cursor).min(logic::sell_ids(data, inventory).len().saturating_sub(1)),
            })
        }
        _ => return false,
    };
    if let Some(phase) = phase {
        state.phase = phase;
    }
    true
}

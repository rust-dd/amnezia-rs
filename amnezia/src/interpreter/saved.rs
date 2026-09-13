use super::{
    RunningEvent,
    frame::{Frame, MAX_CALL_DEPTH},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The foreground command stream and its suspended caller frames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct State {
    frame: Frame,
}

impl RunningEvent {
    pub(crate) fn snapshot(&self) -> Option<State> {
        self.active().then(|| State {
            frame: self.frame.clone(),
        })
    }
}

impl State {
    pub(crate) fn valid(&self) -> bool {
        let frame = &self.frame;
        // Scene-dependent waits cannot resume without their unsaved scene.
        frame.active
            && !frame.parallel
            && !frame.choice_pending
            && !frame.input_pending
            && !frame.shop_pending
            && !frame.battle_pending
            && frame.wait.is_finite()
            && frame.ip <= frame.commands.len()
            && frame.call_stack.len() <= MAX_CALL_DEPTH
            && frame
                .call_stack
                .iter()
                .all(|caller| caller.ip <= caller.commands.len())
    }
}

/// Apply after session cleanup, before map autoruns can start.
pub(crate) fn restore(world: &mut World, state: Option<State>) {
    world.insert_resource(RunningEvent {
        frame: state.map(|state| state.frame).unwrap_or_default(),
    });
}

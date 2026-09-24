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
    #[serde(default)]
    queue: super::foreground::Queue,
    #[serde(default)]
    queued_owner: bool,
}

impl RunningEvent {
    pub(crate) fn snapshot(&self) -> Option<State> {
        (self.active() || !self.queue.empty()).then(|| State {
            frame: self.frame.clone(),
            queue: self.queue.clone(),
            queued_owner: self.queued_owner,
        })
    }
}

impl State {
    pub(crate) fn valid(&self) -> bool {
        let frame = &self.frame;
        if !frame.active {
            return *frame == Frame::default() && !self.queue.empty() && self.queue.valid();
        }
        // Scene-dependent waits cannot resume without their unsaved scene.
        self.queue.valid()
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

    pub(crate) fn valid_map(&self, id: u32, map: &amnezia_data::Map) -> bool {
        self.queue.valid_map(id, map)
    }
}

/// Apply after session cleanup, before map autoruns can start.
pub(crate) fn restore(world: &mut World, state: Option<State>) {
    world.insert_resource(
        state.map_or_else(RunningEvent::default, |state| RunningEvent {
            restoring_queue: !state.queue.empty(),
            frame: state.frame,
            queue: state.queue,
            queued_owner: state.queued_owner,
            fresh: false,
        }),
    );
}

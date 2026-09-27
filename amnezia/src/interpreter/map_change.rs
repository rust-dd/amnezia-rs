use super::{Frame, ParallelPool, RunningEvent, opcodes::*};
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::inputnumber::InputNumber;
use bevy::prelude::*;

pub(crate) fn on_map_change(world: &mut World) {
    if let Some(mut running) = world.get_resource_mut::<RunningEvent>() {
        running.frame.event_id = 0;
        for caller in &mut running.frame.call_stack {
            caller.event_id = 0;
        }
    }
    let frame = world.resource::<crate::timing::GameFrames>().frame;
    if !world
        .get_resource_mut::<Dialogue>()
        .is_some_and(|mut dialogue| dialogue.finish_parallel(frame))
    {
        return;
    }
    if let Some(mut choice) = world.get_resource_mut::<Choice>() {
        *choice = default();
    }
    if let Some(mut number) = world.get_resource_mut::<InputNumber>() {
        *number = default();
    }
    if let Some(mut inn) = world.get_resource_mut::<crate::shop::inn::State>() {
        inn.cancel_prompt();
    }
    if let Some(mut running) = world.get_resource_mut::<RunningEvent>() {
        running.frame.cancel_prompt();
    }
    if let Some(mut pool) = world.get_resource_mut::<ParallelPool>() {
        pool.cancel_prompts();
    }
}

impl Frame {
    pub(super) fn cancel_prompt(&mut self) {
        let Some(command) = self.commands.get(self.ip) else {
            return;
        };
        match command.code {
            SHOW_CHOICE if self.choice_pending => {
                self.choice_pending = false;
                self.choices.remove(&command.indent);
            }
            INPUT_NUMBER if self.input_pending => self.input_pending = false,
            SHOW_INN if self.shop_pending => {
                self.shop_pending = false;
                self.shop_transacted = None;
            }
            _ => return,
        }
        // Discarded callbacks never select even the Cancel/NoStay branch.
        self.ip += 1;
    }
}

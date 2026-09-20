use super::Dialogue;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

pub(crate) fn finish_prompt_text(world: &mut World) {
    for _ in 0..10_000 {
        let dialogue = world.resource::<Dialogue>();
        if dialogue.embedded_prompt().is_none()
            || (dialogue.prompt_input_ready()
                && (world.resource::<crate::choice::Choice>().active()
                    || world.resource::<crate::inputnumber::InputNumber>().active()))
        {
            return;
        }
        world.resource_mut::<crate::timing::GameFrames>().frame += 1;
        world
            .run_system_once(super::typewriter::drive_reveal)
            .unwrap();
        update_prompt(world);
    }
    panic!("prompt text did not finish");
}

pub(crate) fn update_prompt(world: &mut World) {
    world.run_system_once(super::embedded::update).unwrap();
}

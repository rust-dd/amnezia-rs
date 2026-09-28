use super::Presentation;
use crate::dialogue::PromptInput;
use crate::timing::SceneFrames;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(crate) struct Clock {
    last: Option<u32>,
    callback: bool,
    battle: bool,
    message: [u32; 2],
    number: [u32; 2],
}

impl Clock {
    pub(crate) fn source_x(&self, bank: usize, number: bool) -> f32 {
        let phase = if number {
            self.number[bank]
        } else {
            self.message[bank]
        };
        if phase <= 10 { 64.0 } else { 96.0 }
    }

    fn advance(&mut self, now: u32, callback: bool, battle: bool, number: bool, paused: bool) {
        let resumed = callback && (!self.callback || self.last != Some(now));
        let delta = if callback {
            u32::from(resumed)
        } else {
            self.last.map_or(0, |last| now.wrapping_sub(last))
        };
        self.last = Some(now);
        self.callback = callback;
        if battle && !self.battle {
            self.message[1] = 0;
            self.number[1] = 0;
        }
        self.battle = battle;
        if paused {
            return;
        }
        let bank = usize::from(battle);
        self.message[bank] = (self.message[bank] + delta % 21) % 21;
        if number {
            self.number[bank] = (self.number[bank] + delta % 21) % 21;
        }
    }
}

pub(in crate::dialogue) fn register(app: &mut App) {
    app.init_resource::<Clock>()
        .init_resource::<SceneFrames>()
        .add_systems(
            Update,
            update.after(crate::menu::MenuInput).before(PromptInput),
        );
}

fn update(
    frames: Res<SceneFrames>,
    step: Option<Res<crate::timing::logical::Step>>,
    prompts: Presentation,
    scene: crate::world::ScenePause,
    battle: Option<Res<crate::battle::BattleActive>>,
    mut clock: ResMut<Clock>,
) {
    clock.advance(
        frames.frame,
        step.is_some_and(|step| step.callback),
        battle.is_some_and(|battle| battle.0),
        prompts.number.is_some_and(|number| number.active()),
        scene.message_paused(),
    );
}

#[cfg(test)]
mod tests;

use bevy::prelude::*;

mod model;
pub(crate) mod render;
pub(crate) mod smoke;
mod snapshots;

use model::Effect;
pub(crate) use model::Kind;

#[derive(Resource, Default)]
pub(crate) struct Transition {
    effect: Option<Effect>,
    erased: bool,
    serial: u64,
    started: u32,
    frame: u32,
}

impl Transition {
    pub(crate) fn busy(&self) -> bool {
        self.effect.is_some()
    }

    pub(crate) fn start(&mut self, kind: Kind, erase: bool, now: u32, center: IVec2) -> bool {
        if self.busy() {
            return false;
        }
        if kind == Kind::None || (erase && self.erased) {
            return true;
        }
        self.serial = self.serial.wrapping_add(1);
        self.started = now;
        self.frame = 0;
        self.effect = Some(Effect::new(kind, erase, self.erased, center));
        true
    }

    pub(crate) fn clear(&mut self) {
        self.effect = None;
        self.erased = false;
    }

    fn advance(&mut self, now: u32) {
        let Some(effect) = &self.effect else {
            return;
        };
        self.frame = now.wrapping_sub(self.started);
        if self.frame >= effect.duration {
            self.erased = effect.erase;
            self.effect = None;
        }
    }
}

pub(crate) struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Transition>()
            .add_systems(PreUpdate, tick.after(crate::timing::FrameClockSet));
    }
}

fn tick(
    frames: Res<crate::timing::GameFrames>,
    mut transition: ResMut<Transition>,
    capture: Option<Res<snapshots::Capture>>,
) {
    if transition.busy() && capture.is_some_and(|capture| !capture.ready(transition.serial)) {
        transition.started = frames.frame;
        return;
    }
    transition.advance(frames.frame);
}

#[cfg(test)]
mod tests;

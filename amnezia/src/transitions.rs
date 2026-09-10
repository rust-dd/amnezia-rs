use bevy::prelude::*;

pub(crate) mod event_smoke;
mod model;
pub(crate) mod render;
mod settings;
pub(crate) mod smoke;
mod snapshots;

use model::Effect;
pub(crate) use model::Kind;
pub(crate) use settings::{Defaults, Settings, TransitionIo};

/// Images sampled by custom materials must also finish loading before show.
#[derive(Component)]
pub(crate) struct SnapshotImage(pub Handle<Image>);

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct TransitionPause<'w> {
    transition: Option<Res<'w, Transition>>,
    battle: Option<Res<'w, crate::battle::BattleFlow>>,
}

impl TransitionPause<'_> {
    pub(crate) fn paused(&self) -> bool {
        self.transition.as_ref().is_some_and(|t| t.busy())
            || self.battle.as_ref().is_some_and(|b| b.busy())
    }
}

pub(crate) fn scene_running(pause: TransitionPause) -> bool {
    !pause.paused()
}

#[derive(Resource, Default)]
pub(crate) struct Transition {
    effect: Option<Effect>,
    erased: bool,
    serial: u64,
    started: u32,
    frame: u32,
    pub(crate) event_erased: bool,
}

impl Transition {
    pub(crate) fn busy(&self) -> bool {
        self.effect.is_some()
    }

    pub(crate) fn age(&self) -> u32 {
        self.frame
    }

    pub(crate) fn erased(&self) -> bool {
        self.erased && !self.busy()
    }

    pub(crate) fn start(&mut self, kind: Kind, erase: bool, now: u32, center: IVec2) -> bool {
        self.start_for(kind, erase, now, center, kind.frames())
    }

    pub(crate) fn start_for(
        &mut self,
        kind: Kind,
        erase: bool,
        now: u32,
        center: IVec2,
        duration: u32,
    ) -> bool {
        if self.busy() {
            return false;
        }
        if duration == 0 || (kind != Kind::None && erase && self.erased) {
            return true;
        }
        self.serial = self.serial.wrapping_add(1);
        self.started = now;
        self.frame = 0;
        let mut effect = Effect::new(kind, erase, self.erased, center);
        effect.duration = duration;
        self.effect = Some(effect);
        true
    }

    pub(crate) fn clear(&mut self) {
        self.effect = None;
        self.erased = false;
        self.event_erased = false;
    }

    pub(crate) fn hold_black(&mut self) {
        self.clear();
        self.erased = true;
    }

    /// A scene's visibility may already have changed; capture its last full image.
    pub(crate) fn erase_previous(&mut self, now: u32, duration: u32) -> bool {
        let accepted = self.start_for(Kind::Fade, true, now, IVec2::new(160, 120), duration);
        if accepted && let Some(effect) = &mut self.effect {
            effect.previous_scene = true;
        }
        accepted
    }

    pub(crate) fn prepend_battle_flashes(&mut self) {
        if let Some(effect) = &mut self.effect {
            effect.flash_frames = 20;
        }
    }

    fn advance(&mut self, now: u32) {
        let Some(effect) = &self.effect else {
            return;
        };
        self.frame = now.wrapping_sub(self.started);
        if self.frame >= effect.flash_frames + effect.duration {
            if effect.kind != Kind::None {
                self.erased = effect.erase;
            }
            self.effect = None;
        }
    }
}

pub(crate) struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Transition>()
            .init_resource::<crate::timing::GameFrames>()
            .init_resource::<Settings>()
            .init_resource::<Defaults>()
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

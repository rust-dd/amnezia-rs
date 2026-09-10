use super::*;
use crate::timing::GameFrames;

pub(in crate::battle::hud) mod smoke;

#[derive(Clone, Copy)]
struct Slide {
    from: i32,
    to: i32,
    age: u32,
}

#[derive(Resource, Default)]
pub(in crate::battle) struct CommandWindows {
    generation: u64,
    last: Option<u32>,
    context: Option<(Phase, MenuLevel)>,
    chooser: Option<usize>,
    status_cursor: Option<usize>,
    x: i32,
    slide: Option<Slide>,
}

impl CommandWindows {
    pub(super) fn status_cursor(&self) -> Option<usize> {
        self.status_cursor
    }

    pub(super) fn moving(&self) -> bool {
        self.slide.is_some()
    }

    pub(super) fn x(&self, panel: Panel) -> Option<f32> {
        let base = match panel {
            Panel::Option => 0,
            Panel::Status => 76,
            Panel::Command => 320,
            _ => return None,
        };
        Some((base + self.x) as f32)
    }

    fn move_to(&mut self, target: i32) {
        self.slide = (self.x != target).then_some(Slide {
            from: self.x,
            to: target,
            age: 0,
        });
    }

    fn observe(&mut self, battle: &Battle) {
        if self.generation != battle.generation {
            *self = Self {
                generation: battle.generation,
                ..default()
            };
        }
        let context = (battle.phase, battle.menu);
        if battle.phase == Phase::PartyCommand {
            self.status_cursor = None;
        } else if battle.phase == Phase::Command {
            if self.chooser != Some(battle.turn)
                || self
                    .context
                    .is_none_or(|(phase, _)| phase != Phase::Command)
            {
                self.status_cursor = Some(battle.turn);
            }
            if battle.menu == MenuLevel::AllyTarget {
                self.status_cursor = Some(battle.cursor);
            }
            self.chooser = Some(battle.turn);
        }
        if self.context == Some(context) {
            return;
        }
        let previous = self.context.replace(context);
        match battle.phase {
            Phase::PartyCommand if previous.is_some_and(|(phase, _)| phase == Phase::Command) => {
                self.move_to(0);
            }
            Phase::PartyCommand => {
                self.x = 0;
                self.slide = None;
            }
            Phase::Command if battle.menu == MenuLevel::Command => self.move_to(-76),
            Phase::Command => {}
            _ => self.slide = None,
        }
    }

    fn advance(&mut self, frames: u32) {
        let Some(slide) = &mut self.slide else { return };
        slide.age = slide.age.saturating_add(frames);
        let age = slide.age.min(8) as i32;
        self.x = slide.from + (slide.to - slide.from) * age / 8;
        // The reference keeps IsMovementActive true at the endpoint frame.
        if slide.age > 8 {
            self.slide = None;
        }
    }
}

fn tick(
    frames: Res<GameFrames>,
    pause: crate::transitions::TransitionPause,
    battle: Res<Battle>,
    mut windows: ResMut<CommandWindows>,
) {
    let delta = windows
        .last
        .replace(frames.frame)
        .map_or(0, |last| frames.frame.wrapping_sub(last));
    if !pause.paused() {
        windows.advance(delta);
    }
    windows.observe(&battle);
    windows.last = Some(frames.frame);
}

pub(super) fn observe(battle: Res<Battle>, mut windows: ResMut<CommandWindows>) {
    windows.observe(&battle);
}

pub(in crate::battle) fn ready(windows: Res<CommandWindows>) -> bool {
    !windows.moving()
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<CommandWindows>().add_systems(
        Update,
        tick.after(crate::battle::flow::drive)
            .before(crate::battle::input::command_input),
    );
}

#[cfg(test)]
mod tests;
